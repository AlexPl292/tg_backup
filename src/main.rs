use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::path::Path;
use std::thread::sleep;

use grammers_client::types::photo_sizes::VecExt;
use grammers_client::types::{Dialog, Message};
use grammers_client::{Client, ClientHandle, Config};
use grammers_mtproto::mtp::RpcError;
use grammers_mtsender::InvocationError;
use grammers_session::FileSession;
use serde::ser::SerializeSeq;
use serde::ser::Serializer;
use serde_json::ser::{CompactFormatter, Compound};
use simple_logger::SimpleLogger;
use tokio::task;
use tokio::time::Duration;

use crate::attachment_type::AttachmentType;
use crate::types::{
    chat_to_info, msg_to_file_info, msg_to_info, BackUpInfo, FileInfo, MessageInfo,
};

mod attachment_type;
mod types;

const PATH: &'static str = "backup";

/// Features:
///  - Support loading of messages only before current start
///      (do not load messages that where received during backing up)
///  - Support different message types
///  - Fix photos loading
///
/// Bugs:
///  - Photos are not loading
///  - Other attachments don't loading

#[tokio::main]
async fn main() {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
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

    fs::remove_dir_all(PATH).unwrap();
    fs::create_dir(PATH).unwrap();

    save_current_information();

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

fn save_current_information() -> BackUpInfo {
    let current_backup_info = BackUpInfo::current_info();
    let file = File::create(format!("{}/backup.json", PATH)).unwrap();
    serde_json::to_writer_pretty(&file, &current_backup_info).unwrap();
    current_backup_info
}

const PHOTO: &'static str = "photo";
const FILE: &'static str = "file";
const ROUND: &'static str = "round";
const VOICE: &'static str = "voice";

fn init_types() -> HashMap<&'static str, AttachmentType> {
    let mut map = HashMap::new();
    map.insert(PHOTO, AttachmentType::init("photos", PHOTO, Some(".jpg")));
    map.insert(FILE, AttachmentType::init("files", FILE, None));
    map.insert(ROUND, AttachmentType::init("rounds", ROUND, Some(".mp4")));
    map.insert(
        VOICE,
        AttachmentType::init("voice_messages", VOICE, Some(".ogg")),
    );
    map
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

    let chat_path_string = make_path(chat.name(), chat_index);
    let chat_path = Path::new(chat_path_string.as_str());
    fs::create_dir_all(chat_path).unwrap();
    let info_file = chat_path.join("info.json");
    let file = File::create(info_file).unwrap();
    serde_json::to_writer_pretty(&file, &chat_to_info(chat)).unwrap();

    let mut types = init_types();
    types.values_mut().for_each(|x| x.init_folder(chat_path));

    let data_file = chat_path.join("data.json");
    let mut file = File::create(data_file).unwrap();
    let mut ser = serde_json::Serializer::new(std::io::Write::by_ref(&mut file));
    let mut messages = client_handle.iter_messages(chat);
    let total = messages.total().await.ok();
    let mut seq = ser.serialize_seq(total).unwrap();
    loop {
        let msg = messages.next().await;
        match msg {
            Ok(Some(mut message)) => save_message(&mut seq, &mut message, &types).await,
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
    types: &HashMap<&str, AttachmentType>,
) {
    let res = if let Some(photo) = message.photo() {
        log::info!("Loading photo {}", message.text());
        let att_type = types.get(PHOTO).unwrap();
        let file_name = format!("photo@{}.jpg", photo.id());
        let photos_path = att_type.path().join(file_name.as_str());
        let thumbs = photo.thumbs();
        let first = thumbs.largest();
        first.unwrap().download(&photos_path).await;
        Some((att_type, file_name, photo.id()))
    } else if let Some(doc) = message.document() {
        let current_type = if doc.is_round_message() {
            log::info!("Round message {}", message.text());
            types.get(ROUND).unwrap()
        } else if doc.is_voice_message() {
            log::info!("Voice message {}", message.text());
            types.get(VOICE).unwrap()
        } else {
            log::info!("File {}", message.text());
            types.get(FILE).unwrap()
        };

        let file_name = doc.name().unwrap_or(doc.id().to_string());
        let file_name = current_type.format(file_name);
        let file_path = current_type.path().join(file_name.as_str());
        doc.download(&file_path).await;
        Some((current_type, file_name, doc.id()))
    } else {
        None
    };

    if let Some((current_type, file_name, id)) = res {
        let photo_path = format!("./{}/{}", current_type.folder, file_name);
        save_message_with_file(
            seq,
            message,
            id,
            photo_path,
            current_type.type_name.as_str(),
        );
    } else {
        log::info!("Loading no message {}", message.text());
        save_simple_message(seq, message);
    }
}

fn save_simple_message(seq: &mut Compound<&mut File, CompactFormatter>, message: &mut Message) {
    let message_info: MessageInfo = msg_to_info(&message);
    seq.serialize_element(&message_info).unwrap();
}

fn save_message_with_file(
    seq: &mut Compound<&mut File, CompactFormatter>,
    message: &mut Message,
    id: i64,
    main_folder: String,
    attachment_type: &str,
) {
    let photo_info = FileInfo {
        id,
        attachment_type: attachment_type.to_string(),
        path: main_folder,
    };
    let message_info = msg_to_file_info(&message, photo_info);
    seq.serialize_element(&message_info).unwrap();
}

fn make_path(name: &str, id: i32) -> String {
    return format!("{}/chats/{}.{}", PATH, id, name);
}
