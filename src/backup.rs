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
use pbr::ProgressBar;
use tokio::task;
use tokio::task::JoinHandle;
use tokio::time::Duration;

use crate::connector;
use crate::context::{ChatContext, MainContext, FILE, PHOTO, ROUND, VOICE};
use crate::in_progress::{InProgress, InProgressInfo};
use crate::opts::Opts;
use crate::types::{chat_to_info, msg_to_file_info, msg_to_info, BackUpInfo, ChatInfo, FileInfo};
use std::io::BufReader;
use std::sync::Arc;

const PATH: &'static str = "backup";
const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

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

    let log_dir = format!("{}/logs", PATH);
    let _ = fs::create_dir(log_dir.as_str());
    let log_path = format!("{}/tg_backup.log", log_dir);
    simple_logging::log_to_file(log_path, log::LevelFilter::Info).unwrap();

    log::info!("Initializing telegram backup.");
    log::info!("Version v{}", VERSION.unwrap_or("Unknown"));

    let main_ctx = save_current_information(opts.included_chats, opts.batch_size);
    let arc_main_ctx = Arc::new(main_ctx);

    let mut finish_loop = false;
    while !finish_loop {
        let (client_handle, _main_handle) = get_connection().await;

        let result = start_iteration(client_handle, arc_main_ctx.clone()).await;

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

async fn start_iteration(
    client_handle: ClientHandle,
    main_ctx: Arc<MainContext>,
) -> Result<(), ()> {
    let mut dialogs = client_handle.iter_dialogs();
    loop {
        let dialog_res = dialogs.next().await;
        match dialog_res {
            Ok(Some(dialog)) => {
                let client_handle = client_handle.clone();

                // TODO okay, this should be executed in an async manner, but it doesn't work
                //   not sure why. So let's leave it sync.
                let my_main_context = main_ctx.clone();
                let result = task::spawn(async move {
                    extract_dialog(client_handle, dialog, my_main_context).await
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

fn save_current_information(chats: Vec<i32>, batch_size: i32) -> MainContext {
    let loading_chats = if chats.is_empty() { None } else { Some(chats) };
    let mut main_context = MainContext::init(loading_chats, batch_size);

    let path_string = format!("{}/backup.json", PATH);
    let path = Path::new(path_string.as_str());
    if path.exists() {
        let file = BufReader::new(File::open(path).unwrap());
        let parsed_file: Result<BackUpInfo, _> = serde_json::from_reader(file);
        if let Ok(data) = parsed_file {
            main_context.date_from = Some(data.date)
        }
    }

    let back_up_info = BackUpInfo::init(
        main_context.date.clone(),
        main_context.loading_chats.clone(),
        main_context.batch_size,
    );
    let file = File::create(path).unwrap();
    serde_json::to_writer_pretty(&file, &back_up_info).unwrap();
    main_context
}

async fn extract_dialog(
    client_handle: ClientHandle,
    dialog: Dialog,
    main_ctx: Arc<MainContext>,
) -> Result<(), ()> {
    let chat = dialog.chat();

    if let Some(chats) = main_ctx.loading_chats.as_ref() {
        if !chats.contains(&dialog.chat.id()) {
            return Ok(());
        }
    }

    log::info!("Saving chat name: {} id: {}", chat.name(), chat.id());

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

    let mut chat_ctx = ChatContext::init(chat_path, chat.name().to_string());
    let mut start_loading_time = main_ctx.date.clone();
    let mut end_loading_time: Option<DateTime<Utc>> = None;

    let in_progress = InProgress::create(chat_path);
    if info_file_path.exists() {
        if in_progress.exists() {
            // TODO handle unwrap
            let in_progress_data = in_progress.read_data();
            start_loading_time = in_progress_data.extract_from;
            end_loading_time = in_progress_data.extract_until;
            chat_ctx.accumulator_counter = in_progress_data.accumulator_counter;
        } else {
            let file = BufReader::new(File::open(&info_file_path).unwrap());
            let chat_info: ChatInfo = serde_json::from_reader(file).unwrap();
            end_loading_time = Some(chat_info.loaded_up_to);
            in_progress.write_data(InProgressInfo::create(
                start_loading_time,
                end_loading_time,
                &chat_ctx,
                &main_ctx,
            ));
        }
    } else {
        // Create in progress file
        in_progress.write_data(InProgressInfo::create(
            start_loading_time,
            end_loading_time,
            &chat_ctx,
            &main_ctx,
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
    let mut counter = chat_ctx.accumulator_counter * main_ctx.batch_size;

    let mut pb = ProgressBar::new(total_messages as u64);
    pb.message(format!("Loading {} [messages] ", chat.name()).as_str());
    chat_ctx.pb = Some(pb);

    loop {
        let msg = messages.next().await;
        match msg {
            Ok(Some(mut message)) => {
                if let Some(end_time) = end_loading_time {
                    if message.date() < end_time {
                        chat_ctx.force_drop_messages();
                        in_progress.remove_file();
                        log::info!("Finish writing data: {}", chat.name());
                        if let Some(pb) = chat_ctx.pb.as_mut() {
                            pb.finish_print(format!("Finish loading of {}", chat.name()).as_str());
                        }
                        return Ok(());
                    }
                }
                counter += 1;
                last_message = Some((message.id(), message.date()));
                let saving_result = save_message(&mut message, &mut chat_ctx).await;
                if let Err(_) = saving_result {
                    if let Some(pb) = chat_ctx.pb.as_mut() {
                        pb.finish();
                    }
                    return Err(());
                }
                if let Some(pb) = chat_ctx.pb.as_mut() {
                    pb.set(counter as u64);
                    pb.message(format!("Loading {} [messages] ", chat.name()).as_str());
                }
                let dropped = chat_ctx.drop_messages(&main_ctx);
                if dropped {
                    in_progress.write_data(InProgressInfo::create(
                        last_message.unwrap().1,
                        end_loading_time,
                        &chat_ctx,
                        &main_ctx,
                    ));
                }
            }
            Ok(None) => {
                chat_ctx.force_drop_messages();
                in_progress.remove_file();
                log::info!("Finish writing data: {}", chat.name());
                if let Some(pb) = chat_ctx.pb.as_mut() {
                    pb.finish_print(format!("Finish loading of {}", chat.name()).as_str());
                }
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
            &chat_ctx,
            &main_ctx,
        ));
    }

    chat_ctx.force_drop_messages();

    if let Some(pb) = chat_ctx.pb.as_mut() {
        pb.finish();
    }
    Ok(())
}

async fn save_message(message: &mut Message, chat_ctx: &mut ChatContext) -> Result<(), ()> {
    let types = &chat_ctx.types;
    let res = if let Some(photo) = message.photo() {
        if let Some(pb) = chat_ctx.pb.as_mut() {
            pb.message(format!("Loading {} [photo   ] ", chat_ctx.chat_name.as_str()).as_str());
        }
        log::debug!("Loading photo {}", message.text());
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
            Some((att_type, file_name, id))
        }
    } else if let Some(mut doc) = message.document() {
        let current_type = if doc.is_round_message() {
            if let Some(pb) = chat_ctx.pb.as_mut() {
                pb.message(format!("Loading {} [round   ] ", chat_ctx.chat_name.as_str()).as_str());
            }
            log::debug!("Round message {}", message.text());
            types.get(ROUND).unwrap()
        } else if doc.is_voice_message() {
            if let Some(pb) = chat_ctx.pb.as_mut() {
                pb.message(format!("Loading {} [voice   ] ", chat_ctx.chat_name.as_str()).as_str());
            }
            log::debug!("Voice message {}", message.text());
            types.get(VOICE).unwrap()
        } else {
            if let Some(pb) = chat_ctx.pb.as_mut() {
                pb.message(format!("Loading {} [file    ] ", chat_ctx.chat_name.as_str()).as_str());
            }
            log::debug!("File {}", message.text());
            types.get(FILE).unwrap()
        };

        let mut file_name = doc.name().to_string();
        if file_name.is_empty() {
            file_name = doc.id().to_string();
        }
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
        chat_ctx.messages_accumulator.push(message_info);
        Ok(())
    } else {
        log::debug!("Loading message {}", message.text());
        chat_ctx.messages_accumulator.push(msg_to_info(message));
        Ok(())
    }
}

fn make_path(id: i32, name: &str) -> String {
    return format!("{}/chats/{}.{}", PATH, id, name);
}
