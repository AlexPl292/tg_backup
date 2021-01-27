use std::fs;
use std::fs::File;
use std::path::Path;
use std::thread::sleep;

use grammers_client::types::{Dialog, Media, Message};
use grammers_client::{Client, ClientHandle, Config};
use grammers_mtproto::mtp::RpcError;
use grammers_mtsender::InvocationError;
use grammers_session::FileSession;
use serde::ser::SerializeSeq;
use serde::ser::Serializer;
use simple_logger::SimpleLogger;
use tokio::task;
use tokio::time::Duration;

use crate::types::{chat_to_info, msg_to_info, msg_to_photo_info, MessageInfo};
use serde_json::ser::{CompactFormatter, Compound};

mod types;

const PATH: &'static str = "backup";

/// Features:
///  - Support "last backup marker"
///  - Support different message types
///  - Fix photos loading

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
    loop {
        let dialog_res = dialogs.next().await;
        match dialog_res {
            Ok(Some(dialog)) => {
                chat_index += 1;
                let client_handle = client_handle.clone();

                // TODO okay, this should be executed in an async manner, but it doesn't work
                //   not sure why. So let's leave it sync.
                task::spawn(async move {
                    extract_dialog(client_handle, chat_index, dialog).await;
                })
                .await
                .unwrap();
            }
            Ok(None) => break,
            Err(e) => {
                log::error!("{}", e);
                break;
            }
        };
    }
}

async fn extract_dialog(client_handle: ClientHandle, chat_index: i32, dialog: Dialog) {
    let chat = dialog.chat();

    if dialog.chat.id() != 422281 {
        return;
    }
    /*    if let Chat::User(_) = chat {
    } else {
        // Save only one-to-one dialogs at the moment
        return;
    } */

    let path_str = make_path(chat.name(), chat_index);
    let path = Path::new(path_str.as_str());
    fs::create_dir_all(path).unwrap();
    let info_file = path.join("info.json");
    let file = File::create(info_file).unwrap();
    serde_json::to_writer_pretty(&file, &chat_to_info(chat)).unwrap();

    let photos_path = path.join("photos");
    fs::create_dir(&photos_path).unwrap();

    let data_file = path.join("data.json");
    let mut file = File::create(data_file).unwrap();
    let mut ser = serde_json::Serializer::new(std::io::Write::by_ref(&mut file));
    let mut messages = client_handle.iter_messages(chat);
    let total = messages.total().await.ok();
    let mut seq = ser.serialize_seq(total).unwrap();
    loop {
        let msg = messages.next().await;
        match msg {
            Ok(Some(mut message)) => save_message(&mut seq, &mut message, &photos_path).await,
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

async fn save_message(
    seq: &mut Compound<'_, &mut File, CompactFormatter>,
    message: &mut Message,
    photos_path: &Path,
) {
    log::debug!("Write element");

    match message.media() {
        Some(media) => {
            match media {
                Media::Photo(photo) => {
                    let file_name = format!("photo@{}.jpg", photo.id());
                    let photos_path = photos_path.join(file_name);
                    match message.download_media(&photos_path).await {
                        Ok(_) => {}
                        Err(e) => {
                            log::error!("Cannot load file: {}", e)
                        }
                    };

                    let message_with_photo = msg_to_photo_info(message, &photo);
                    seq.serialize_element(&message_with_photo).unwrap();
                }
                _ => save_simple_message(seq, message),
            };
        }
        None => save_simple_message(seq, message),
    };
}

fn save_simple_message(seq: &mut Compound<&mut File, CompactFormatter>, message: &mut Message) {
    let message_info: MessageInfo = msg_to_info(&message);
    seq.serialize_element(&message_info).unwrap();
}

fn make_path(name: &str, id: i32) -> String {
    return format!("{}/{}.{}", PATH, id, name);
}
