use std::fs;
use std::fs::File;
use std::path::Path;
use std::thread::sleep;

use chrono::{DateTime, Utc};
use grammers_client::types::photo_sizes::VecExt;
use grammers_client::types::{Dialog, Message};
use grammers_client::ClientHandle;
use grammers_mtproto::mtp::RpcError;
use grammers_mtsender::InvocationError;
use simple_logger::SimpleLogger;
use tokio::task;
use tokio::time::Duration;

use crate::context::{Context, FILE, PHOTO, ROUND, VOICE};
use crate::types::{chat_to_info, msg_to_file_info, msg_to_info, BackUpInfo, Error, FileInfo};
use std::io::BufReader;

mod attachment_type;
mod connector;
mod context;
mod types;

const PATH: &'static str = "backup";

#[tokio::main]
async fn main() {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .init()
        .unwrap();

    let client_handle = connector::create_connection().await;

    // let _ = fs::remove_dir_all(PATH);
    let _ = fs::create_dir(PATH);

    let backup_info = save_current_information();

    let mut chat_index = 0;
    let mut dialogs = client_handle.iter_dialogs();
    loop {
        let dialog_res = dialogs.next().await;
        match dialog_res {
            Ok(Some(dialog)) => {
                chat_index += 1;
                let client_handle = client_handle.clone();

                // TODO okay, this should be executed in an async manner, but it doesn't work
                //   not sure why. So let's leave it sync.
                let date = backup_info.date;
                task::spawn(async move {
                    extract_dialog(client_handle, chat_index, dialog, &date).await;
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

async fn extract_dialog(
    client_handle: ClientHandle,
    chat_index: i32,
    dialog: Dialog,
    current_time: &DateTime<Utc>,
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
    let _ = fs::create_dir_all(chat_path);

    let info_file_path = chat_path.join("info.json");

    let mut context = Context::init(chat_path);
    let mut start_loading_time = current_time.clone();

    let in_progress_path = chat_path.join("in_progress");
    if info_file_path.exists() {
        if in_progress_path.exists() {
            // TODO handle unwrap
            let file = BufReader::new(File::open(&in_progress_path).unwrap());
            let in_progress_data: (DateTime<Utc>, i32) = serde_json::from_reader(file).unwrap();
            start_loading_time = in_progress_data.0;
            context.accumulator_counter = in_progress_data.1;
        } else {
            // This loading is finished
            return;
        }
    } else {
        // Create in progress file
        let in_progress_file = File::create(&in_progress_path).unwrap();
        serde_json::to_writer_pretty(
            &in_progress_file,
            &(start_loading_time, context.accumulator_counter),
        )
        .unwrap();
    }

    let info_file = File::create(info_file_path).unwrap();
    serde_json::to_writer_pretty(&info_file, &chat_to_info(chat)).unwrap();

    let mut messages = client_handle
        .iter_messages(chat)
        .offset_date(start_loading_time.timestamp() as i32);
    let mut last_message: Option<(i32, DateTime<Utc>)> = None;
    loop {
        let msg = messages.next().await;
        match msg {
            Ok(Some(mut message)) => {
                last_message = Some((message.id(), message.date()));
                save_message(&mut message, &mut context).await;
                let dropped = context.drop_messages();
                if dropped {
                    let file1 = File::create(&in_progress_path).unwrap();
                    serde_json::to_writer_pretty(
                        &file1,
                        &(last_message.unwrap().1, context.accumulator_counter),
                    )
                    .unwrap();
                }
            }
            Ok(None) => {
                context.force_drop_messages();
                fs::remove_file(in_progress_path).unwrap();
                log::info!("Finish writing data: {}", chat.name());
                return;
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
                    if let Some((id, date)) = last_message {
                        context.errors.push(Error::NotFullLoading(id, date));
                    }
                    break;
                }
            }
            Err(e) => {
                log::error!("Error {}", e);
                if let Some((id, date)) = last_message {
                    context.errors.push(Error::NotFullLoading(id, date));
                }
                break;
            }
        };
    }
    context.save_errors(PATH, chat.id());
    context.force_drop_messages();

    let file1 = File::create(&in_progress_path).unwrap();
    serde_json::to_writer_pretty(
        &file1,
        &(last_message.unwrap().1, context.accumulator_counter),
    )
    .unwrap();
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
}

fn make_path(name: &str, id: i32) -> String {
    return format!("{}/chats/{}.{}", PATH, id, name);
}
