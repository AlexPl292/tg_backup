use grammers_client::{Client, ClientHandle, Config};
use grammers_session::FileSession;
use tokio::task;

pub async fn create_connection() -> ClientHandle {
    let api_id = env!("TG_ID").parse().expect("TG_ID invalid");
    let api_hash = env!("TG_HASH").to_string();

    println!("Connecting to Telegram...");
    let client = Client::connect(Config {
        session: FileSession::load_or_create("dialogs.session").unwrap(),
        api_id,
        api_hash: api_hash.clone(),
        params: Default::default(),
    })
    .await
    .unwrap();
    println!("Connected!");

    let client_handle = client.handle();

    task::spawn(async move { client.run_until_disconnected().await });
    client_handle
}
