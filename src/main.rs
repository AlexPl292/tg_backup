use grammers_client::types::Chat;
use grammers_client::{Client, Config};
use grammers_session::FileSession;
use serde::ser::SerializeSeq;
use serde::ser::Serializer;
use simple_logger::SimpleLogger;
use std::fs::File;
use tokio::task;

#[tokio::main]
async fn main() {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Debug)
        .init()
        .unwrap();

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

    let mut file = File::create("rust_backup.txt").unwrap();

    let mut ser = serde_json::Serializer::new(std::io::Write::by_ref(&mut file));
    let mut seq = ser.serialize_seq(Some(100)).unwrap();
    for item in res {
        seq.serialize_element(&item).unwrap();
    }
    seq.end().unwrap();
}
