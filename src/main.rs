use grammers_client::{Client, Config};
use grammers_tl_types as tl;
use simple_logger::SimpleLogger;
use tokio::task;
use grammers_session::FileSession;
use grammers_client::types::Chat;
use std::fs::File;
use std::io::Write;
use grammers_tl_types::Serializable;

#[tokio::main]
async fn main() {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Debug)
        .init()
        .unwrap();

    let api_id = env!("TG_ID").parse().expect("TG_ID invalid");
    let api_hash = env!("TG_HASH").to_string();

    println!("Connecting to Telegram...");
    let mut client = Client::connect(Config {
        session: FileSession::load_or_create("dialogs.session").unwrap(),
        api_id,
        api_hash: api_hash.clone(),
        params: Default::default(),
    }).await.unwrap();
    println!("Connected!");

    let mut client_handle = client.handle();
    task::spawn(async move { client.run_until_disconnected().await });

    let mut dialogs = client_handle.iter_dialogs();

    let mut found_chat: Option<Chat> = None;
    while let Some(dialog) = dialogs.next().await.unwrap() {
        let chat = dialog.chat();
        if chat.id() == 59061750 {
            found_chat = Some(chat.clone());
        }
    }

    let nic_chat = found_chat.unwrap();
    let mut messages = client_handle.iter_messages(&nic_chat).limit(100);

    let mut res: Vec<String> = vec![];
    while let Some(message) = messages.next().await.unwrap() {
        res.push(message.text().to_string());
    }

    let string_res = serde_json::to_string_pretty(&res).unwrap();
    let mut file = File::create("rust_backup.txt").unwrap();
    file.write_all(string_res.as_bytes()).unwrap();
}
