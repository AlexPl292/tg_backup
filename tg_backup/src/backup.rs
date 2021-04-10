/*
 * tg_backup - backup your messages from the Telegram messenger
 * Copyright 2021-2021 Alex Plate
 *
 * This file is part of tg_backup.
 *
 * tg_backup is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * tg_backup is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with tg_backup.  If not, see <https://www.gnu.org/licenses/>.
 */

use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::thread::sleep;

use chrono::{DateTime, Utc};
use pbr::ProgressBar;
use tokio::task;
use tokio::time::Duration;

use tg_backup_connector::TgError;
use tg_backup_types::Member;

use crate::context::{ChatContext, MainContext, MainMutContext, FILE, PHOTO, ROUND, VOICE};
use crate::in_progress::{InProgress, InProgressInfo};
use crate::opts::{Opts, SubCommand};
use crate::types::Attachment::PhotoExpired;
use crate::types::{
    chat_to_info, msg_to_file_info, msg_to_info, Attachment, BackUpInfo, ChatInfo, FileInfo,
};
use tg_backup_connector::test::TestTg;
use tg_backup_connector::traits::{DDialog, DMessage, Tg};

const PATH: &'static str = "backup";
const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

pub async fn start_backup<T>(test_data: Option<TestTg>, opts: Opts)
where
    T: Tg + 'static,
{
    // Start auth subcommand
    if let Some(auth) = opts.auth {
        let SubCommand::Auth(auth_data) = auth;
        T::auth(auth_data.session_file_dir, auth_data.session_file_name).await;
        return;
    }

    let session_file = opts.session_file;

    // Check if authentication is needed
    if T::need_auth(&session_file) {
        println!("Start tg_backup with `auth` command");
        return;
    }

    let output_dir = path_or_default(&opts.output);
    // Create backup directory
    if opts.clean {
        let _ = fs::remove_dir_all(output_dir.as_path());
    }
    let _ = fs::create_dir(output_dir.as_path());

    // Initialize logs
    let log_dir = format!("{}/logs", output_dir.as_path().display().to_string());
    let _ = fs::create_dir(log_dir.as_str());
    let log_path = format!("{}/tg_backup.log", log_dir);
    simple_logging::log_to_file(log_path, log::LevelFilter::Info).unwrap();

    log::info!("Initializing telegram backup.");
    log::info!("Version v{}", VERSION.unwrap_or("Unknown"));

    // Initialize main context
    let main_ctx = save_current_information(
        opts.included_chats,
        opts.excluded_chats,
        opts.batch_size,
        output_dir,
    );

    let main_mut_context = Arc::new(RwLock::new(MainMutContext {
        already_finished: vec![],
        amount_of_dialogs: None,
    }));

    // Save me
    let mut tg: T = get_connection::<T>(test_data.clone(), &session_file).await;
    save_me(&mut tg, &main_ctx).await;
    drop(tg);

    // Start backup loop
    let mut finish_loop = false;
    let arc_main_ctx = Arc::new(main_ctx);
    while !finish_loop {
        let tg: T = get_connection::<T>(test_data.clone(), &session_file).await;

        let result = start_iteration(tg, arc_main_ctx.clone(), main_mut_context.clone()).await;

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

fn path_or_default(folder: &Option<String>) -> PathBuf {
    if let Some(path) = folder {
        let mut path_buf = PathBuf::new();
        path_buf.push(shellexpand::tilde(path).into_owned());
        path_buf
    } else {
        let mut dif_path = PathBuf::new();
        dif_path.push(PATH);
        dif_path
    }
}

async fn get_connection<T>(test_data: Option<TestTg>, session_file: &Option<String>) -> T
where
    T: Tg,
{
    let mut counter = 0;
    loop {
        let connect = T::create_connection(test_data.clone(), session_file).await;
        if let Ok(tg) = connect {
            return tg;
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

async fn save_me<T>(tg: &mut T, main_context: &MainContext)
where
    T: Tg,
{
    let me_result = tg.get_me().await;
    match me_result {
        Ok(me) => {
            let path_string = format!("{}/me.json", main_context.output_dir.display().to_string());
            let path = Path::new(path_string.as_str());
            let file = File::create(path).unwrap();
            serde_json::to_writer_pretty(&file, &me).unwrap();
        }
        Err(e) => {
            log::error!("Cannot save information about me: {}", e)
        }
    }
}

async fn start_iteration<T>(
    mut tg: T,
    main_ctx: Arc<MainContext>,
    main_mut_ctx: Arc<RwLock<MainMutContext>>,
) -> Result<(), ()>
where
    T: Tg + 'static,
{
    let mut dialogs_iter = tg.dialogs().await;
    if let Ok(mut ctx) = main_mut_ctx.write() {
        if let None = ctx.amount_of_dialogs {
            let total_dialogs_count = dialogs_iter.total().await;
            if let Ok(dialogs_count) = total_dialogs_count {
                ctx.amount_of_dialogs = Some(dialogs_count);
                log::info!("Saving {} dialogs", dialogs_count);
                println!("Saving {} dialogs", dialogs_count);
            }
        }
    }
    loop {
        let dialog_res = dialogs_iter.next().await;
        match dialog_res {
            Ok(Some(dialog)) => {
                let tg = tg.clone();

                // TODO okay, this should be executed in an async manner, but it doesn't work
                //   not sure why. So let's leave it sync.
                let my_main_context = main_ctx.clone();
                let my_main_mut_context = main_mut_ctx.clone();
                let result = task::spawn(async move {
                    extract_dialog(tg, dialog, my_main_context, my_main_mut_context).await
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

fn save_current_information(
    chats: Vec<i32>,
    excluded: Vec<i32>,
    batch_size: i32,
    output_dir: PathBuf,
) -> MainContext {
    let loading_chats = if chats.is_empty() { None } else { Some(chats) };
    let mut main_context =
        MainContext::init(loading_chats, excluded, batch_size, output_dir.clone());

    let path_string = format!("{}/backup.json", output_dir.as_path().display());
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

async fn extract_dialog<T>(
    mut tg: T,
    mut dialog: Box<dyn DDialog>,
    main_ctx: Arc<MainContext>,
    main_mut_ctx: Arc<RwLock<MainMutContext>>,
) -> Result<(), ()>
where
    T: Tg,
{
    let chat = dialog.chat();

    let chat_id = chat.id();
    let chat_name = chat.name();

    if let Some(chats) = main_ctx.included_chats.as_ref() {
        if !chats.contains(&chat_id) {
            return Ok(());
        }
    }

    if main_ctx.excluded_chats.contains(&chat_id) {
        return Ok(());
    }

    if let Ok(ctx) = main_mut_ctx.read() {
        if ctx.already_finished.contains(&chat_id) {
            return Ok(());
        }
    }

    let member = if let Some(member) = chat.user() {
        member
    } else {
        // Save only one-to-one dialogs at the moment
        return Ok(());
    };

    let username = if let Member::User { username, .. } = &member {
        username
    } else {
        panic!()
    };
    let visual_id = format!(
        "{}.{}",
        chat_name,
        username.as_ref().unwrap_or(&String::from("NO_USERNAME"))
    );

    log::info!("Saving chat. name: {} id: {}", chat_name, chat_id);

    let chat_path_string = format!(
        "{}/chats/{}.{}",
        main_ctx.output_dir.as_path().display(),
        chat_id,
        visual_id.as_str()
    );
    let chat_path = Path::new(chat_path_string.as_str());
    if !chat_path.exists() {
        let _ = fs::create_dir_all(chat_path);
    }

    let info_file_path = chat_path.join("info.json");

    let mut chat_ctx = ChatContext::init(chat_path, visual_id.clone());
    let mut start_loading_time = main_ctx.date.clone();
    let mut end_loading_time: Option<DateTime<Utc>> = None;
    let mut last_loaded_id: Option<i32> = None;
    let mut counter = chat_ctx.accumulator_counter * main_ctx.batch_size;
    let mut global_loading_from = main_ctx.date.clone();
    let mut amount_of_already_loaded_messages: usize = 0;

    let in_progress = InProgress::create(chat_path);
    if info_file_path.exists() {
        let file = BufReader::new(File::open(&info_file_path).unwrap());
        let chat_info: ChatInfo = serde_json::from_reader(file).unwrap();
        if in_progress.exists() {
            log::info!("Loading data from in_progress file");
            let in_progress_data = in_progress.read_data();
            start_loading_time = in_progress_data.extract_from;
            end_loading_time = in_progress_data.extract_until;
            chat_ctx.accumulator_counter = in_progress_data.accumulator_counter;
            chat_ctx.file_issue = in_progress_data.file_issue;
            chat_ctx.file_issue_count = in_progress_data.file_issue_count;
            counter = in_progress_data.messages_counter;
            last_loaded_id = in_progress_data.last_loaded_id;
            global_loading_from = chat_info.loaded_up_to;
        } else {
            end_loading_time = Some(chat_info.loaded_up_to);
            amount_of_already_loaded_messages = chat_info.total_messages;
            let info = InProgressInfo::create(
                start_loading_time,
                end_loading_time,
                None,
                counter,
                &chat_ctx,
            );
            in_progress.write_data(&info);
        }
    } else {
        // Create in progress file
        let info = InProgressInfo::create(
            start_loading_time,
            end_loading_time,
            None,
            counter,
            &chat_ctx,
        );
        in_progress.write_data(&info);
    }

    let mut messages = tg.messages(&chat, start_loading_time.timestamp() as i32, last_loaded_id);
    let mut last_message: Option<(i32, DateTime<Utc>)> = None;
    let total_messages = messages.total().await.unwrap_or(0);
    let amount_of_messages_to_load = total_messages - amount_of_already_loaded_messages;

    // Save info file
    let info_file = File::create(info_file_path).unwrap();
    serde_json::to_writer_pretty(
        &info_file,
        &chat_to_info(chat, global_loading_from, total_messages),
    )
    .unwrap();

    // Save members
    let members = vec![Member::Me, member];
    let members_folder = chat_path.join("members");
    let _ = fs::create_dir(&members_folder);
    let members_path = members_folder.join("members.json");
    let members_file = File::create(members_path).unwrap();
    serde_json::to_writer_pretty(&members_file, &members).unwrap();

    // Initialize progress bar
    let mut pb = ProgressBar::new(amount_of_messages_to_load as u64);
    pb.message(format!("Loading {} [messages] ", visual_id).as_str());
    chat_ctx.pb = Some(pb);

    log::info!(
        "Start loading loop. Counter: {}, total_size: {}, from: {}, until: {:?}",
        counter,
        amount_of_messages_to_load,
        start_loading_time,
        end_loading_time,
    );

    let mut pivot_time = chrono::offset::Utc::now();

    loop {
        let msg = messages.next().await;
        match msg {
            Ok(Some(message)) => {
                let message_date = message.date();
                let message_id = message.id();
                if let Some(end_time) = end_loading_time {
                    if message_date < end_time {
                        chat_ctx.force_drop_messages();
                        in_progress.remove_file();
                        if let Ok(mut ctx) = main_mut_ctx.write() {
                            ctx.already_finished.push(chat_id);
                        }
                        log::info!("Finish writing data: {}", chat_name);
                        if let Some(pb) = chat_ctx.pb.as_mut() {
                            pb.finish_println(format!("Finish loading of {}", chat_name).as_str());
                        }
                        return Ok(());
                    }
                }
                let saving_result = save_message(message, &mut chat_ctx).await;
                if let Err(_) = saving_result {
                    log::error!("Error while loading");
                    if let Some(pb) = chat_ctx.pb.as_mut() {
                        pb.message("Error while loading");
                        print!(".")
                    }
                    let info = if let Some((id, time)) = last_message {
                        InProgressInfo::create(time, end_loading_time, Some(id), counter, &chat_ctx)
                    } else {
                        InProgressInfo::create(
                            start_loading_time,
                            end_loading_time,
                            last_loaded_id,
                            counter,
                            &chat_ctx,
                        )
                    };
                    in_progress.write_data(&info);
                    log::info!("Force drop messages. Counter: {}", info.messages_counter);
                    chat_ctx.force_drop_messages();
                    return Err(());
                }

                let current_time = chrono::offset::Utc::now();
                let diff = current_time - pivot_time;
                if diff > chrono::Duration::seconds(10) {
                    log::info!(
                        "Loading messages... {}/{}",
                        counter,
                        amount_of_messages_to_load
                    );
                    pivot_time = current_time;
                }

                last_message = Some((message_id, message_date));
                if let Some(pb) = chat_ctx.pb.as_mut() {
                    pb.set(counter as u64);
                    pb.message(format!("Loading {} [messages] ", visual_id).as_str());
                }
                let dropped = chat_ctx.drop_messages(&main_ctx);
                if dropped {
                    let info = InProgressInfo::create(
                        last_message.unwrap().1,
                        end_loading_time,
                        Some(last_message.unwrap().0),
                        counter,
                        &chat_ctx,
                    );
                    in_progress.write_data(&info);
                }
                counter += 1;
            }
            Ok(None) => {
                chat_ctx.force_drop_messages();
                in_progress.remove_file();
                if let Ok(mut ctx) = main_mut_ctx.write() {
                    ctx.already_finished.push(chat_id);
                }
                log::info!("Finish writing data: {}", chat_name);
                if let Some(pb) = chat_ctx.pb.as_mut() {
                    pb.finish_println(format!("Finish loading of {}", chat_name).as_str());
                }
                return Ok(());
            }
            Err(TgError::Rpc { name, value }) => {
                if name == "FLOOD_WAIT" {
                    log::warn!("Flood wait: {}", value.unwrap());
                    sleep(Duration::from_secs(value.unwrap() as u64))
                } else if name == "FILE_MIGRATE" {
                    log::warn!("File migrate: {}", value.unwrap());
                } else {
                    log::error!("Error {}, {:?}", name, value)
                }
            }
            Err(e) => {
                log::error!("Error {}", e);
                break;
            }
        };
    }

    if let Some(message) = last_message {
        let info = InProgressInfo::create(
            message.1,
            end_loading_time,
            Some(message.0),
            counter,
            &chat_ctx,
        );
        in_progress.write_data(&info);
    }

    chat_ctx.force_drop_messages();

    // if let Some(pb) = chat_ctx.pb.as_mut() {
    //     pb.finish();
    // }
    Ok(())
}

async fn save_message(message_x: Box<dyn DMessage>, chat_ctx: &mut ChatContext) -> Result<(), ()> {
    let message_text = message_x.text();
    let types = &chat_ctx.types;
    let option_photo = message_x.photo();
    let option_document = message_x.document();
    let res = if let Some(photo) = option_photo {
        if let Some(pb) = chat_ctx.pb.as_mut() {
            pb.message(format!("Loading {} [photo   ] ", chat_ctx.visual_id.as_str()).as_str());
        }
        log::debug!("Loading photo {}", message_text);
        let current_type = types.get(PHOTO).unwrap();
        let photo_id = photo.id();
        if let Some(id) = photo_id {
            let file_name = format!("{}@photo.jpg", id);
            let photos_path = current_type.path().join(file_name.as_str());
            let data_corut = photo.load_largest(&photos_path);
            let downloaded = data_corut.await;
            let photo_path = format!("../{}/{}", current_type.folder, file_name);
            let attachment = Attachment::Photo(FileInfo {
                id,
                path: photo_path,
            });
            if let Err(e) = downloaded {
                if chat_ctx.file_issue == id {
                    chat_ctx.file_issue_count += 1;
                    if chat_ctx.file_issue_count > 5 {
                        log::error!("Cannot download photo, no more attempts {}", e);
                        Some(Attachment::Error(format!("Cannot load: {}", e)))
                    } else {
                        log::warn!(
                            "Cannot download photo, attempt {}, error: {}",
                            chat_ctx.file_issue_count,
                            e
                        );
                        return Err(());
                    }
                } else {
                    chat_ctx.file_issue = id;
                    chat_ctx.file_issue_count = 0;
                    log::warn!("Cannot download photo, first attempt: {}", e);
                    return Err(());
                }
            } else {
                Some(attachment)
            }
        } else {
            Some(PhotoExpired)
        }
    } else if let Some(mut doc) = option_document {
        let doc_id = doc.id();
        let doc_name = doc.name();
        let (attachment, file_path) = if doc.is_round_message() {
            if let Some(pb) = chat_ctx.pb.as_mut() {
                pb.message(format!("Loading {} [round   ] ", chat_ctx.visual_id.as_str()).as_str());
            }
            log::debug!("Round message {}", message_text);
            let current_type = types.get(ROUND).unwrap();
            let mut file_name = doc_name;
            file_name = format!("{}@{}", doc_id, file_name);
            let file_name = current_type.format(file_name);
            let file_path = current_type.path().join(file_name.as_str());
            let att_path = format!("../{}/{}", current_type.folder, file_name);
            let attachment = Attachment::Round(FileInfo {
                id: doc_id,
                path: att_path,
            });
            (attachment, file_path)
        } else if doc.is_voice_message() {
            if let Some(pb) = chat_ctx.pb.as_mut() {
                pb.message(format!("Loading {} [voice   ] ", chat_ctx.visual_id.as_str()).as_str());
            }
            log::debug!("Voice message {}", message_text);
            let current_type = types.get(VOICE).unwrap();
            let mut file_name = doc_name;
            file_name = format!("{}@{}", doc_id, file_name);
            let file_name = current_type.format(file_name);
            let file_path = current_type.path().join(file_name.as_str());
            let att_path = format!("../{}/{}", current_type.folder, file_name);
            let attachment = Attachment::Voice(FileInfo {
                id: doc_id,
                path: att_path,
            });
            (attachment, file_path)
        } else {
            if let Some(pb) = chat_ctx.pb.as_mut() {
                pb.message(format!("Loading {} [file    ] ", chat_ctx.visual_id.as_str()).as_str());
            }
            log::debug!("File {}", message_text);
            let current_type = types.get(FILE).unwrap();
            let mut file_name = doc_name;
            file_name = format!("{}@{}", doc_id, file_name);
            let file_name = current_type.format(file_name);
            let file_path = current_type.path().join(file_name.as_str());
            let photo_path = format!("../{}/{}", current_type.folder, file_name);
            let attachment = Attachment::File(FileInfo {
                id: doc_id,
                path: photo_path,
            });
            (attachment, file_path)
        };

        // TODO handle file migrate
        let downloaded = doc.download(&file_path).await;
        if let Err(e) = downloaded {
            if chat_ctx.file_issue == doc_id {
                chat_ctx.file_issue_count += 1;
                if chat_ctx.file_issue_count > 5 {
                    Some(Attachment::Error(format!("Cannot load: {}", e)))
                } else {
                    log::error!("Cannot download photo");
                    return Err(());
                }
            } else {
                chat_ctx.file_issue = doc_id;
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
        let message_info = msg_to_file_info(message_x, attachment);
        chat_ctx.messages_accumulator.push(message_info);
        Ok(())
    } else {
        log::debug!("Loading message {}", message_text);
        chat_ctx.messages_accumulator.push(msg_to_info(message_x));
        Ok(())
    }
}
