use grammers_client::{Client, ClientHandle, Config};
use grammers_session::FileSession;
use tokio::task;
use grammers_mtsender::{ReadError, AuthorizationError};
use tokio::task::JoinHandle;

pub async fn create_connection() -> Result<(ClientHandle, JoinHandle<Result<(), ReadError>>), AuthorizationError> {
    let api_id = env!("TG_ID").parse().expect("TG_ID invalid");
    let api_hash = env!("TG_HASH").to_string();

    println!("Connecting to Telegram...");
    let client = Client::connect(Config {
        session: FileSession::load_or_create("dialogs.session").unwrap(),
        api_id,
        api_hash: api_hash.clone(),
        params: Default::default(),
    })
    .await?;
    println!("Connected!");

    let client_handle = client.handle();

    let main_handle = task::spawn(async move { client.run_until_disconnected().await });
    Ok((client_handle, main_handle))
}
