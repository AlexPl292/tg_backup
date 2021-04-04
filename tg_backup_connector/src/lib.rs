/*
 * tg_backup - backup your messages from the Telegram messenger
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

use async_trait::async_trait;
use grammers_client::{Client, ClientHandle, Config, SignInError};
use grammers_mtsender::{AuthorizationError, ReadError};
use grammers_session::FileSession;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::{env, fs, io};
use tokio::task;
use tokio::task::JoinHandle;

const DEFAULT_FILE_NAME: &'static str = "tg_backup.session";

#[async_trait]
pub trait Tg {
    async fn create_connection(
        session_file: &Option<String>,
    ) -> Result<(ClientHandle, JoinHandle<Result<(), ReadError>>), AuthorizationError>;
}

pub struct ProductionTg {}

#[async_trait]
impl Tg for ProductionTg {
    async fn create_connection(
        session_file: &Option<String>,
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
}

fn path_or_default(session_file: &Option<String>) -> Result<PathBuf, ()> {
    if let Some(path) = session_file {
        let mut path_buf = PathBuf::new();
        path_buf.push(shellexpand::tilde(path).into_owned());
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
