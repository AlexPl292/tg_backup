use std::fs;
use std::fs::File;
use std::path::Path;
use std::thread::sleep;

use chrono::{DateTime, Utc};
use grammers_client::types::photo_sizes::VecExt;
use grammers_client::types::{Chat, Dialog, Message};
use grammers_client::ClientHandle;
use grammers_mtproto::mtp::RpcError;
use grammers_mtsender::{InvocationError, ReadError};
use tokio::task;
use tokio::task::JoinHandle;
use tokio::time::Duration;
use pbr::ProgressBar;

use crate::connector;
use crate::context::{Context, FILE, PHOTO, ROUND, VOICE};
use crate::in_progress::{InProgress, InProgressInfo};
use crate::opts::Opts;
use crate::types::{chat_to_info, msg_to_file_info, msg_to_info, BackUpInfo, ChatInfo, FileInfo};
use std::io::BufReader;

const PATH: &'static str = "backup";

pub async fn start_backup(opts: Opts) {
    if let Some(_) = opts.auth {
        connector::auth().await;
        return;
    }

    if connector::need_auth() {
        println!("Start tg_backup with `auth` command");
        return;
    }

    if opts.clean {
        let _ = fs::remove_dir_all(PATH);
    }
    let _ = fs::create_dir(PATH);

    let backup_info = save_current_information(opts.included_chats, opts.batch_size);

    let mut finish_loop = false;
    while !finish_loop {
        let (client_handle, _main_handle) = get_connection().await;

        let result = start_iteration(client_handle, &backup_info).await;

        match result {
            Ok(_) => {
                println!("Finish");
                finish_loop = true
            }
            Err(_) => {
                println!("Continue");
                finish_loop = false
            }
        }
    }
}

async fn get_connection() -> (ClientHandle, JoinHandle<Result<(), ReadError>>) {
    let mut counter = 0;
    loop {
        let connect = connector::create_connection().await;
        if let Ok((handle, main_loop)) = connect {
            return (handle, main_loop);
        }
        counter += 1;
        let time_sec = if counter < 5 {
            counter * 5
        } else if counter < 10 {
            counter * 10
        } else {
            panic!("Cannot connect to telegram")
        };
        sleep(Duration::from_secs(time_sec))
    }
}

async fn start_iteration(client_handle: ClientHandle, backup_info: &BackUpInfo) -> Result<(), ()> {
    let mut dialogs = client_handle.iter_dialogs();
    loop {
        let dialog_res = dialogs.next().await;
        match dialog_res {
            Ok(Some(dialog)) => {
                let client_handle = client_handle.clone();

                // TODO okay, this should be executed in an async manner, but it doesn't work
                //   not sure why. So let's leave it sync.
                let my_backup_info = (*backup_info).clone();
                let result = task::spawn(async move {
                    extract_dialog(client_handle, dialog, my_backup_info).await
                })
                .await
                .unwrap();
                if let Err(_) = result {
                    return Err(());
                }
            }
            Ok(None) => return Ok(()),
            Err(e) => {
                log::error!("{}", e);
                return Err(());
            }
        };
    }
}

fn save_current_information(chats: Vec<i32>, batch_size: i32) -> BackUpInfo {
    let loading_chats = if chats.is_empty() { None } else { Some(chats) };
    let mut current_backup_info = BackUpInfo::load_info(loading_chats, batch_size);

    let path_string = format!("{}/backup.json", PATH);
    let path = Path::new(path_string.as_str());
    if path.exists() {
        let file = BufReader::new(File::open(path).unwrap());
        let parsed_file: Result<BackUpInfo, _> = serde_json::from_reader(file);
        if let Ok(data) = parsed_file {
            current_backup_info.date_from = Some(data.date)
        }
    }

    let file = File::create(path).unwrap();
    serde_json::to_writer_pretty(&file, &current_backup_info).unwrap();
    current_backup_info
}

async fn extract_dialog(
    client_handle: ClientHandle,
    dialog: Dialog,
    backup_info: BackUpInfo,
) -> Result<(), ()> {
    let chat = dialog.chat();

    // println!("{}/{}", dialog.chat.name(), dialog.chat.id());
    if let Some(chats) = backup_info.loading_chats.as_ref() {
        if !chats.contains(&dialog.chat.id()) {
            return Ok(());
        }
    }

    if let Chat::User(_) = chat {
    } else {
        // Save only one-to-one dialogs at the moment
        return Ok(());
    }

    let chat_path_string = make_path(chat.id(), chat.name());
    let chat_path = Path::new(chat_path_string.as_str());
    if !chat_path.exists() {
        let _ = fs::create_dir_all(chat_path);
    }

    let info_file_path = chat_path.join("info.json");

    let mut context = Context::init(chat_path);
    let mut start_loading_time = backup_info.date.clone();
    let mut end_loading_time: Option<DateTime<Utc>> = None;

    let in_progress = InProgress::create(chat_path);
    if info_file_path.exists() {
        if in_progress.exists() {
            // TODO handle unwrap
            let in_progress_data = in_progress.read_data();
            start_loading_time = in_progress_data.extract_from;
            end_loading_time = in_progress_data.extract_until;
            context.accumulator_counter = in_progress_data.accumulator_counter;
        } else {
            let file = BufReader::new(File::open(&info_file_path).unwrap());
            let chat_info: ChatInfo = serde_json::from_reader(file).unwrap();
            end_loading_time = Some(chat_info.loaded_up_to);
            in_progress.write_data(InProgressInfo::create(
                start_loading_time,
                end_loading_time,
                &context,
                &backup_info,
            ));
        }
    } else {
        // Create in progress file
        in_progress.write_data(InProgressInfo::create(
            start_loading_time,
            end_loading_time,
            &context,
            &backup_info,
        ));
    }

    let info_file = File::create(info_file_path).unwrap();
    serde_json::to_writer_pretty(&info_file, &chat_to_info(chat, start_loading_time.clone()))
        .unwrap();

    let mut messages = client_handle
        .iter_messages(chat)
        .offset_date(start_loading_time.timestamp() as i32);
    let mut last_message: Option<(i32, DateTime<Utc>)> = None;
    let total_messages = messages.total().await.unwrap_or(0);
    let mut counter = context.accumulator_counter * backup_info.batch_size;

    let mut pb = ProgressBar::new(total_messages as u64);
    pb.message(format!("Loading {} ", chat.name()).as_str());

    loop {
        let msg = messages.next().await;
        match msg {
            Ok(Some(mut message)) => {
                if let Some(end_time) = end_loading_time {
                    if message.date() < end_time {
                        context.force_drop_messages();
                        in_progress.remove_file();
                        log::info!("Finish writing data: {}", chat.name());
                        pb.finish_print(format!("Finish loading of {}", chat.name()).as_str());
                        return Ok(());
                    }
                }
                counter += 1;
                last_message = Some((message.id(), message.date()));
                let saving_result = save_message(&mut message, &mut context).await;
                if let Err(_) = saving_result {
                    pb.finish();
                    return Err(());
                }
                pb.set(counter as u64);
                let dropped = context.drop_messages(&backup_info);
                if dropped {
                    in_progress.write_data(InProgressInfo::create(
                        last_message.unwrap().1,
                        end_loading_time,
                        &context,
                        &backup_info,
                    ));
                }
            }
            Ok(None) => {
                context.force_drop_messages();
                in_progress.remove_file();
                log::info!("Finish writing data: {}", chat.name());
                pb.finish_print(format!("Finish loading of {}", chat.name()).as_str());
                return Ok(());
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

    if let Some(message) = last_message {
        in_progress.write_data(InProgressInfo::create(
            message.1,
            end_loading_time,
            &context,
            &backup_info,
        ));
    }

    context.force_drop_messages();
    pb.finish();
    Ok(())
}

async fn save_message(message: &mut Message, context: &mut Context) -> Result<(), ()> {
    let types = &context.types;
    let res = if let Some(photo) = message.photo() {
        log::info!("Loading photo {}", message.text());
        let att_type = types.get(PHOTO).unwrap();
        let id = photo.id();
        let file_name = format!("photo@{}.jpg", id);
        let photos_path = att_type.path().join(file_name.as_str());
        let thumbs = photo.thumbs();
        let first = thumbs.largest();
        let downloaded = first.unwrap().download(&photos_path).await;
        if let Err(_) = downloaded {
            log::error!("Cannot download photo");
            return Err(());
        } else {
            log::info!("Loaded");
            Some((att_type, file_name, id))
        }
    } else if let Some(mut doc) = message.document() {
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

        let mut file_name = doc.name().to_string();
        if file_name.is_empty() {
            file_name = doc.id().to_string();
        }
        let file_name = current_type.format(file_name);
        let file_path = current_type.path().join(file_name.as_str());
        doc.download(&file_path).await;
        log::info!("Loaded");
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
        Ok(())
    } else {
        // log::info!("Loading no message {}", message.text());
        context.messages_accumulator.push(msg_to_info(message));
        Ok(())
    }
}

fn make_path(id: i32, name: &str) -> String {
    return format!("{}/chats/{}.{}", PATH, id, name);
}
