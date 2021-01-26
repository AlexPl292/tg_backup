mod types;

use crate::types::{chat_to_info, msg_to_info, MessageInfo};
use grammers_client::types::Dialog;
use grammers_client::{Client, ClientHandle, Config};
use grammers_mtproto::mtp::RpcError;
use grammers_mtsender::InvocationError;
use grammers_session::FileSession;
use serde::ser::SerializeSeq;
use serde::ser::Serializer;
use simple_logger::SimpleLogger;
use std::fs;
use std::fs::File;
use std::path::Path;
use std::thread::sleep;
use tokio::task;
use tokio::time::Duration;

const PATH: &'static str = "backup";

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

    let mut chat_index = 0;
    while let Some(dialog) = dialogs.next().await.unwrap() {
        chat_index += 1;
        extract_dialog(&client_handle, chat_index, dialog).await;
    }
}

async fn extract_dialog(client_handle: &ClientHandle, chat_index: i32, dialog: Dialog) {
    let chat = dialog.chat();
    let path_str = make_path(chat.name(), chat_index);
    let path = Path::new(path_str.as_str());
    fs::create_dir_all(path).unwrap();
    let info_file = path.join("info.json");
    let file = File::create(info_file).unwrap();
    serde_json::to_writer_pretty(&file, &chat_to_info(chat)).unwrap();

    let data_file = path.join("data.json");
    let mut file = File::create(data_file).unwrap();
    let mut ser = serde_json::Serializer::new(std::io::Write::by_ref(&mut file));
    let mut messages = client_handle.iter_messages(chat);
    let total = messages.total().await.ok();
    let mut seq = ser.serialize_seq(total).unwrap();
    loop {
        let msg = messages.next().await;
        match msg {
            Ok(Some(message)) => {
                log::debug!("Write element");
                let message_info: MessageInfo = msg_to_info(&message);
                seq.serialize_element(&message_info).unwrap();
            }
            Ok(None) => {
                break;
            }
            Err(InvocationError::Rpc(RpcError {
                code: _,
                name,
                value,
            })) => {
                if name == "FLOOD_WAIT" {
                    log::info!("Flood wait: {}", value.unwrap());
                    sleep(Duration::from_secs(value.unwrap() as u64))
                } else {
                    break;
                }
            }
            Err(e) => {
                log::error!("Error {}", e);
                break;
            }
        };
    }
    seq.end().unwrap();
    log::info!("Finish writing data: {}", chat.name());
}

fn make_path(name: &str, id: i32) -> String {
    return format!("{}/{}.{}", PATH, id, name);
}
