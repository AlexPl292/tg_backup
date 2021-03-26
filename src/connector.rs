use grammers_client::{Client, ClientHandle, Config, SignInError};
use grammers_mtsender::{AuthorizationError, ReadError};
use grammers_session::FileSession;
use std::io;
use std::io::{BufRead, Write};
use std::path::Path;
use tokio::task;
use tokio::task::JoinHandle;

const FILE_NAME: &'static str = "tg_backup.session";

pub fn need_auth() -> bool {
    !Path::new(FILE_NAME).exists()
}

pub async fn create_connection(
) -> Result<(ClientHandle, JoinHandle<Result<(), ReadError>>), AuthorizationError> {
    let api_id = env!("TG_ID").parse().expect("TG_ID invalid");
    let api_hash = env!("TG_HASH").to_string();

    log::info!("Connecting to Telegram...");
    let client = Client::connect(Config {
        session: FileSession::load(FILE_NAME).unwrap(),
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

pub async fn auth() {
    let api_id = env!("TG_ID").parse().expect("TG_ID invalid");
    let api_hash = env!("TG_HASH").to_string();

    log::info!("Connecting to Telegram...");
    let mut client = Client::connect(Config {
        session: FileSession::create(FILE_NAME).unwrap(),
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
