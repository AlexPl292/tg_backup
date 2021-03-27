use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
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
use crate::types::{
    chat_to_info, msg_to_file_info, msg_to_info, Attachment, BackUpInfo, ChatInfo, FileInfo,
};

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

    let main_ctx =
        save_current_information(opts.included_chats, opts.excluded_chats, opts.batch_size);
    let arc_main_ctx = Arc::new(main_ctx);

    let mut finish_loop = false;
    while !finish_loop {
        let (client_handle, _main_handle) = get_connection().await;

        let result = start_iteration(client_handle, arc_main_ctx.clone()).await;

        finish_loop = match result {
            Ok(_) => {
                log::info!("Finishing backups");
                true
            }
            Err(_) => {
                log::info!("Start new backup loop");
                false
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

fn save_current_information(chats: Vec<i32>, excluded: Vec<i32>, batch_size: i32) -> MainContext {
    let loading_chats = if chats.is_empty() { None } else { Some(chats) };
    let mut main_context = MainContext::init(loading_chats, excluded, batch_size);

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
        main_context.included_chats.clone(),
        main_context.excluded_chats.clone(),
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

    if let Some(chats) = main_ctx.included_chats.as_ref() {
        if !chats.contains(&dialog.chat.id()) {
            return Ok(());
        }
    }

    if main_ctx.excluded_chats.contains(&dialog.chat.id()) {
        return Ok(());
    }

    if let Chat::User(_) = chat {
    } else {
        // Save only one-to-one dialogs at the moment
        return Ok(());
    }

    log::info!("Saving chat. name: {} id: {}", chat.name(), chat.id());

    let chat_path_string = make_path(chat.id(), chat.name());
    let chat_path = Path::new(chat_path_string.as_str());
    if !chat_path.exists() {
        let _ = fs::create_dir_all(chat_path);
    }

    let info_file_path = chat_path.join("info.json");

    let mut chat_ctx = ChatContext::init(chat_path, chat.name().to_string());
    let mut start_loading_time = main_ctx.date.clone();
    let mut end_loading_time: Option<DateTime<Utc>> = None;
    let mut last_loaded_id: Option<i32> = None;
    let mut counter = chat_ctx.accumulator_counter * main_ctx.batch_size;

    let in_progress = InProgress::create(chat_path);
    if info_file_path.exists() {
        if in_progress.exists() {
            let in_progress_data = in_progress.read_data();
            start_loading_time = in_progress_data.extract_from;
            end_loading_time = in_progress_data.extract_until;
            chat_ctx.accumulator_counter = in_progress_data.accumulator_counter;
            chat_ctx.file_issue = in_progress_data.file_issue;
            chat_ctx.file_issue_count = in_progress_data.file_issue_count;
            counter = in_progress_data.messages_counter;
            last_loaded_id = in_progress_data.last_loaded_id;
        } else {
            let file = BufReader::new(File::open(&info_file_path).unwrap());
            let chat_info: ChatInfo = serde_json::from_reader(file).unwrap();
            end_loading_time = Some(chat_info.loaded_up_to);
            in_progress.write_data(InProgressInfo::create(
                start_loading_time,
                end_loading_time,
                None,
                &chat_ctx,
                &main_ctx,
            ));
        }
    } else {
        // Create in progress file
        in_progress.write_data(InProgressInfo::create(
            start_loading_time,
            end_loading_time,
            None,
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
    if let Some(id) = last_loaded_id {
        messages = messages.offset_id(id);
    }
    let mut last_message: Option<(i32, DateTime<Utc>)> = None;
    let total_messages = messages.total().await.unwrap_or(0);

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
                            pb.message(format!("Finish loading of {}", chat.name()).as_str());
                            println!()
                        }
                        return Ok(());
                    }
                }
                counter += 1;
                let saving_result = save_message(&mut message, &mut chat_ctx).await;
                if let Err(_) = saving_result {
                    if let Some(pb) = chat_ctx.pb.as_mut() {
                        pb.message("Error while loading");
                        println!()
                    }
                    if let Some((id, time)) = last_message {
                        in_progress.write_data(InProgressInfo::create(
                            time,
                            end_loading_time,
                            Some(id),
                            &chat_ctx,
                            &main_ctx,
                        ));
                    } else {
                        in_progress.write_data(InProgressInfo::create(
                            start_loading_time,
                            end_loading_time,
                            last_loaded_id,
                            &chat_ctx,
                            &main_ctx,
                        ));
                    }
                    chat_ctx.force_drop_messages();
                    return Err(());
                }
                last_message = Some((message.id(), message.date()));
                if let Some(pb) = chat_ctx.pb.as_mut() {
                    pb.set(counter as u64);
                    pb.message(format!("Loading {} [messages] ", chat.name()).as_str());
                }
                let dropped = chat_ctx.drop_messages(&main_ctx);
                if dropped {
                    in_progress.write_data(InProgressInfo::create(
                        last_message.unwrap().1,
                        end_loading_time,
                        Some(last_message.unwrap().0),
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
                    pb.finish_println(format!("Finish loading of {}", chat.name()).as_str());
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
            Some(message.0),
            &chat_ctx,
            &main_ctx,
        ));
    }

    chat_ctx.force_drop_messages();

    // if let Some(pb) = chat_ctx.pb.as_mut() {
    //     pb.finish();
    // }
    Ok(())
}

async fn save_message(message: &mut Message, chat_ctx: &mut ChatContext) -> Result<(), ()> {
    let types = &chat_ctx.types;
    let res = if let Some(photo) = message.photo() {
        if let Some(pb) = chat_ctx.pb.as_mut() {
            pb.message(format!("Loading {} [photo   ] ", chat_ctx.chat_name.as_str()).as_str());
        }
        log::debug!("Loading photo {}", message.text());
        let current_type = types.get(PHOTO).unwrap();
        let id = photo.id();
        if let None = id {
            // TODO handle this
            // Photo has been expired
            return Ok(());
        }
        let id = id.unwrap();
        let file_name = format!("photo@{}.jpg", id);
        let photos_path = current_type.path().join(file_name.as_str());
        let thumbs = photo.thumbs();
        let first = thumbs.largest();
        let downloaded = first.unwrap().download(&photos_path).await;
        let photo_path = format!("../{}/{}", current_type.folder, file_name);
        let attachment = Attachment::Photo(FileInfo {
            id,
            path: photo_path,
        });
        if let Err(e) = downloaded {
            if chat_ctx.file_issue == id {
                chat_ctx.file_issue_count += 1;
                if chat_ctx.file_issue_count > 5 {
                    Some(Attachment::Error(format!("Cannot load: {}", e)))
                } else {
                    log::error!("Cannot download photo");
                    return Err(());
                }
            } else {
                chat_ctx.file_issue = id;
                chat_ctx.file_issue_count = 0;
                log::error!("Cannot download photo");
                return Err(());
            }
        } else {
            Some(attachment)
        }
    } else if let Some(mut doc) = message.document() {
        let (attachment, file_path) = if doc.is_round_message() {
            if let Some(pb) = chat_ctx.pb.as_mut() {
                pb.message(format!("Loading {} [round   ] ", chat_ctx.chat_name.as_str()).as_str());
            }
            log::debug!("Round message {}", message.text());
            let current_type = types.get(ROUND).unwrap();
            let mut file_name = doc.name().to_string();
            file_name = format!("{}-{}", doc.id(), file_name);
            let file_name = current_type.format(file_name);
            let file_path = current_type.path().join(file_name.as_str());
            let att_path = format!("../{}/{}", current_type.folder, file_name);
            let attachment = Attachment::Round(FileInfo {
                id: doc.id(),
                path: att_path,
            });
            (attachment, file_path)
        } else if doc.is_voice_message() {
            if let Some(pb) = chat_ctx.pb.as_mut() {
                pb.message(format!("Loading {} [voice   ] ", chat_ctx.chat_name.as_str()).as_str());
            }
            log::debug!("Voice message {}", message.text());
            let current_type = types.get(VOICE).unwrap();
            let mut file_name = doc.name().to_string();
            file_name = format!("{}-{}", doc.id(), file_name);
            let file_name = current_type.format(file_name);
            let file_path = current_type.path().join(file_name.as_str());
            let att_path = format!("../{}/{}", current_type.folder, file_name);
            let attachment = Attachment::Voice(FileInfo {
                id: doc.id(),
                path: att_path,
            });
            (attachment, file_path)
        } else {
            if let Some(pb) = chat_ctx.pb.as_mut() {
                pb.message(format!("Loading {} [file    ] ", chat_ctx.chat_name.as_str()).as_str());
            }
            log::debug!("File {}", message.text());
            let current_type = types.get(FILE).unwrap();
            let mut file_name = doc.name().to_string();
            file_name = format!("{}-{}", doc.id(), file_name);
            let file_name = current_type.format(file_name);
            let file_path = current_type.path().join(file_name.as_str());
            let photo_path = format!("../{}/{}", current_type.folder, file_name);
            let attachment = Attachment::File(FileInfo {
                id: doc.id(),
                path: photo_path,
            });
            (attachment, file_path)
        };

        // TODO handle file migrate
        let downloaded = doc.download(&file_path).await;
        if let Err(e) = downloaded {
            if chat_ctx.file_issue == doc.id() {
                chat_ctx.file_issue_count += 1;
                if chat_ctx.file_issue_count > 5 {
                    Some(Attachment::Error(format!("Cannot load: {}", e)))
                } else {
                    log::error!("Cannot download photo");
                    return Err(());
                }
            } else {
                chat_ctx.file_issue = doc.id();
                chat_ctx.file_issue_count = 0;
                log::error!("Cannot download photo");
                return Err(());
            }
        } else {
            Some(attachment)
        }
    } else {
        None
    };

    if let Some(attachment) = res {
        let message_info = msg_to_file_info(&message, attachment);
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
