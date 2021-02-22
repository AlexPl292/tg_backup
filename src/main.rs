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
use simple_logger::SimpleLogger;
use tokio::task;
use tokio::time::Duration;

use crate::attachment_type::AttachmentType;
use crate::types::{
    chat_to_info, msg_to_file_info, msg_to_info, BackUpInfo, FileInfo, MessageInfo,
};
use chrono::{DateTime, Utc};

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

    let _ = fs::remove_dir_all(PATH);
    fs::create_dir(PATH).unwrap();

    let backup_info = save_current_information();

    let current_time = backup_info.date;

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
                    extract_dialog(client_handle, chat_index, dialog, current_time).await;
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

const MESSAGES: &'static str = "messages";
const PHOTO: &'static str = "photo";
const FILE: &'static str = "file";
const ROUND: &'static str = "round";
const VOICE: &'static str = "voice";

const ACCUMULATOR_SIZE: usize = 1_000;

struct Context {
    types: HashMap<String, AttachmentType>,
    messages_accumulator: Vec<MessageInfo>,
    accumulator_counter: i32,
}

impl Context {
    pub fn init(path: &Path) -> Context {
        let mut types = Context::init_types();
        types.values_mut().for_each(|x| x.init_folder(path));
        Context {
            types,
            messages_accumulator: vec![],
            accumulator_counter: 0,
        }
    }

    fn init_types() -> HashMap<String, AttachmentType> {
        let mut map = HashMap::new();
        map.insert(
            MESSAGES.to_string(),
            AttachmentType::init("messages", MESSAGES, None),
        );
        map.insert(
            PHOTO.to_string(),
            AttachmentType::init("photos", PHOTO, Some(".jpg")),
        );
        map.insert(FILE.to_string(), AttachmentType::init("files", FILE, None));
        map.insert(
            ROUND.to_string(),
            AttachmentType::init("rounds", ROUND, Some(".mp4")),
        );
        map.insert(
            VOICE.to_string(),
            AttachmentType::init("voice_messages", VOICE, Some(".ogg")),
        );
        map
    }

    fn drop_messages(&mut self) {
        if self.messages_accumulator.len() < ACCUMULATOR_SIZE {
            return;
        }
        self.force_drop_messages()
    }

    fn force_drop_messages(&mut self) {
        let data_type = self.types.get(MESSAGES).unwrap();
        let messages_path = data_type.path();
        let file_path = messages_path.join(format!("data-{}.json", self.accumulator_counter));
        let file = File::create(file_path).unwrap();
        serde_json::to_writer_pretty(&file, &self.messages_accumulator).unwrap();

        self.messages_accumulator.clear();
        self.accumulator_counter += 1;
    }
}

async fn extract_dialog(
    client_handle: ClientHandle,
    chat_index: i32,
    dialog: Dialog,
    current_time: DateTime<Utc>,
) {
    let chat = dialog.chat();

    // println!("{}/{}", dialog.chat.name(), dialog.chat.id());
    if dialog.chat.id() != 59061750 {
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

    let mut context = Context::init(chat_path);

    let mut messages = client_handle
        .iter_messages(chat)
        .offset_date(current_time.timestamp() as i32);
    loop {
        let msg = messages.next().await;
        match msg {
            Ok(Some(mut message)) => save_message(&mut message, &mut context).await,
            Ok(None) => {
                break;
            }
            Err(InvocationError::Rpc(RpcError {
                code: _,
                name,
                value,
            })) => {
                if name == "FLOOD_WAIT" {
                    log::warn!("Flood wait: {}", value.unwrap());
                    sleep(Duration::from_secs(value.unwrap() as u64))
                } else if name == "FILE_MIGRATE" {
                    log::warn!("File migrate: {}", value.unwrap());
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
    context.force_drop_messages();
    log::info!("Finish writing data: {}", chat.name());
}

async fn save_message(message: &mut Message, context: &mut Context) {
    let types = &context.types;
    let res = if let Some(photo) = message.photo() {
        log::info!("Loading photo {}", message.text());
        let att_type = types.get(PHOTO).unwrap();
        let photo_id = photo.id();
        let id = match photo_id {
            Some(id) => id,
            None => {
                log::error!("Cannot get photo id");
                return;
            }
        };
        let file_name = format!("photo@{}.jpg", id);
        let photos_path = att_type.path().join(file_name.as_str());
        let thumbs = photo.thumbs();
        let first = thumbs.largest();
        let downloaded = first.unwrap().download(&photos_path).await;
        if let Err(_) = downloaded {
            // TODO process it in a better way
            log::error!("Cannot download photo");
            None
        } else {
            Some((att_type, file_name, id))
        }
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
        let photo_path = format!("../{}/{}", current_type.folder, file_name);
        let attachment_type = current_type.type_name.as_str();
        let attachment_info = FileInfo {
            id,
            attachment_type: attachment_type.to_string(),
            path: photo_path,
        };
        let message_info = msg_to_file_info(&message, attachment_info);
        context.messages_accumulator.push(message_info);
    } else {
        // log::info!("Loading no message {}", message.text());
        context.messages_accumulator.push(msg_to_info(message));
    }
    context.drop_messages();
}

fn make_path(name: &str, id: i32) -> String {
    return format!("{}/chats/{}.{}", PATH, id, name);
}
