/*
 * tg_backup - software to backup data from Telegram messenger
 * Copyright 2021-2021 Alex Plate
 *
 * This file is part of tg_backup.
 *
 * tg_backup is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * tg_backup is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with tg_backup.  If not, see <https://www.gnu.org/licenses/>.
 */

use grammers_client::{Client, ClientHandle, Config, SignInError};
use grammers_mtsender::{AuthorizationError, ReadError};
use grammers_session::FileSession;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::{env, fs, io};
use tokio::task;
use tokio::task::JoinHandle;

const DEFAULT_FILE_NAME: &'static str = "tg_backup.session";

pub fn need_auth(session_file: Option<String>) -> bool {
    let path_result = path_or_default(session_file);
    let path = if let Ok(path) = path_result {
        path
    } else {
        return true;
    };
    !path.exists()
}

pub async fn create_connection(
    session_file: Option<String>,
) -> Result<(ClientHandle, JoinHandle<Result<(), ReadError>>), AuthorizationError> {
    let api_id = env!("TG_ID").parse().expect("TG_ID invalid");
    let api_hash = env!("TG_HASH").to_string();

    let path_result = path_or_default(session_file);
    let path = path_result.expect("Session file expected to be existed at this moment");

    log::info!("Connecting to Telegram...");
    let client = Client::connect(Config {
        session: FileSession::load(path).unwrap(),
        api_id,
        api_hash: api_hash.clone(),
        params: Default::default(),
    })
    .await?;
    log::info!("Connected!");

    let client_handle = client.handle();

    let main_handle = task::spawn(async move { client.run_until_disconnected().await });
    Ok((client_handle, main_handle))
}

pub async fn auth(session_file_path: Option<String>, session_file_name: Option<String>) {
    let api_id = env!("TG_ID").parse().expect("TG_ID invalid");
    let api_hash = env!("TG_HASH").to_string();

    let path_result = make_path(session_file_path, session_file_name);
    let path = if let Ok(path) = path_result {
        path
    } else {
        return;
    };

    log::info!("Connecting to Telegram...");
    let mut client = Client::connect(Config {
        session: FileSession::create(path.as_path()).unwrap(),
        api_id,
        api_hash: api_hash.clone(),
        params: Default::default(),
    })
    .await
    .unwrap();
    log::info!("Connected!");

    if !client.is_authorized().await.unwrap() {
        log::info!("Signing in...");
        let phone = prompt("Enter your phone number (international format): ").unwrap();
        let token = client
            .request_login_code(&phone, api_id, &api_hash)
            .await
            .unwrap();
        let code = prompt("Enter the code you received: ").unwrap();
        let signed_in = client.sign_in(&token, &code).await;
        match signed_in {
            Err(SignInError::PasswordRequired(password_token)) => {
                // Note: this `prompt` method will echo the password in the console.
                //       Real code might want to use a better way to handle this.
                let hint = password_token.hint().unwrap();
                let prompt_message = format!("Enter the password (hint {}): ", &hint);
                let password = prompt(prompt_message.as_str()).unwrap();

                client
                    .check_password(password_token, password.trim())
                    .await
                    .unwrap();
            }
            Ok(_) => (),
            Err(e) => panic!("{}", e),
        };
        log::info!("Signed in!");
        log::info!("Create session file under {:?}", path.as_path());
        println!("Create session file under {:?}", path.as_path());
        match client.session().save() {
            Ok(_) => {}
            Err(e) => {
                log::error!(
                    "NOTE: failed to save the session, will sign out when done: {}",
                    e
                );
            }
        }
    }
}

fn make_path(
    session_file_path: Option<String>,
    session_file_name: Option<String>,
) -> Result<PathBuf, ()> {
    let mut file_path = if let Some(file_path) = session_file_path {
        let mut buf = PathBuf::new();
        buf.push(shellexpand::tilde(file_path.as_str()).into_owned());
        buf
    } else {
        let default_file_path = default_file_path();
        match default_file_path {
            Ok(path) => path,
            Err(error) => {
                println!("{}", error);
                return Err(());
            }
        }
    };
    let file_name = session_file_name.unwrap_or(DEFAULT_FILE_NAME.to_string());

    let _ = fs::create_dir_all(file_path.as_path());

    file_path.push(file_name);
    Ok(file_path)
}

fn path_or_default(session_file: Option<String>) -> Result<PathBuf, ()> {
    if let Some(path) = session_file {
        let mut path_buf = PathBuf::new();
        path_buf.push(shellexpand::tilde(path.as_str()).into_owned());
        Ok(path_buf)
    } else {
        let default_file_path = default_file_path();
        let mut file_path = match default_file_path {
            Ok(path) => path,
            Err(error) => {
                println!("{}", error);
                return Err(());
            }
        };

        let _ = fs::create_dir_all(file_path.as_path());

        file_path.push(DEFAULT_FILE_NAME.to_string());
        Ok(file_path)
    }
}

fn default_file_path() -> Result<PathBuf, String> {
    let os = env::consts::OS;
    let mut home = match home::home_dir() {
        Some(home) => home,
        None => {
            return Err(String::from(
                "Please specify session file path using --session-file-path option",
            ));
        }
    };
    let folder = if os == "linux" {
        String::from(".tg_backup")
    } else if os == "macos" {
        String::from(".tg_backup")
    } else if os == "windows" {
        String::from(".tg_backup")
    } else {
        return Err(String::from(
            "Please specify session file path using --session-file-path option",
        ));
    };
    home.push(folder);
    return Ok(home);
}

type MyResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn prompt(message: &str) -> MyResult<String> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    stdout.write_all(message.as_bytes())?;
    stdout.flush()?;

    let stdin = io::stdin();
    let mut stdin = stdin.lock();

    let mut line = String::new();
    stdin.read_line(&mut line)?;
    Ok(line)
}
