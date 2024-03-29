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

use regex::Regex;
use std::cmp::Ordering;
use std::fs::{DirEntry, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::thread::sleep;
use std::{env, fs, io};

use chrono::{DateTime, Utc};
use pbr::ProgressBar;
use tokio::task;
use tokio::time::Duration;

use crate::actions::Action;
use crate::companion::ChannelState::{ASK, BACKUP};
use crate::companion::ChannelsStateInfo;
use crate::context::{ChatContext, MainContext, MainMutContext, FILE, PHOTO, ROUND, VOICE};
use crate::ext::{ChatExt, MessageExt};
use crate::in_progress::{InProgress, InProgressInfo};
use crate::logs::init_logs;
use crate::opts::{Opts, SubCommand};
use crate::types::Attachment::{PhotoExpired, TooLarge};
use crate::types::{
    chat_to_info, msg_to_info, Attachment, BackUpInfo, ChatInfo, FileInfo, Member, MessageInfo,
};
use grammers_client::types::photo_sizes::VecExt;
use grammers_client::types::{Dialog, Message};
use grammers_client::{Client, Config, SignInError};
use grammers_mtproto::mtp::RpcError;
use grammers_mtsender::{AuthorizationError, InvocationError};
use grammers_session::Session;
use grammers_tl_types as tl;
use log::info;
use serde_json::Error;
use std::collections::HashSet;
use sysinfo::{AsU32, Pid, System, SystemExt};

const PATH: &'static str = "backup";
const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
const DEFAULT_FILE_NAME: &'static str = "tg_backup.session";

pub async fn start_backup(opts: Opts, long_messages_results: Vec<ChannelsStateInfo>) {
    // Start auth subcommand
    if let Some(auth_command) = opts.auth {
        let SubCommand::Auth(auth_data) = auth_command;
        auth(auth_data.session_file_dir, auth_data.session_file_name).await;
        return;
    }

    let session_file = opts.session_file;

    // Check if authentication is needed
    if need_auth(&session_file) {
        if !opts.quiet {
            println!("Start tg_backup with `auth` command");
        }
        return;
    }

    let output_dir = path_or_default_output(&opts.output);
    // Create backup directory
    if opts.clean {
        let _ = fs::remove_dir_all(output_dir.as_path());
    }
    let _ = fs::create_dir(output_dir.as_path());

    // Check instance uniqueness
    let continue_execution = create_lock_file(output_dir.as_path());
    if !continue_execution {
        if !opts.quiet {
            println!("An instance of tg_backup already running");
        }
        log::info!("An instance of tg_backup already running. Stop following execution.");
        return;
    }

    // Initialize logs
    init_logs(&output_dir, opts.keep_last_n_logs, opts.panic_to_stderr);

    log::info!("Initializing telegram backup.");
    log::info!("Version v{}", VERSION.unwrap_or("Unknown"));

    // Initialize main context
    let main_ctx = save_current_information(
        opts.included_chats,
        opts.excluded_chats,
        opts.batch_size,
        output_dir.as_path(),
        opts.quiet,
        opts.file_limit,
        opts.max_participants,
        opts.test,
    );

    let main_mut_context = Arc::new(RwLock::new(MainMutContext {
        already_finished: vec![],
        amount_of_dialogs: None,
        total_flood_wait: 0,
        long_messages_info: long_messages_results,
    }));

    // Save me
    let mut client = get_connection(&session_file).await;
    save_me(&mut client, &main_ctx).await;
    drop(client);

    // Start backup loop
    let mut finish_loop = false;
    let arc_main_ctx = Arc::new(main_ctx);
    while !finish_loop {
        let client = get_connection(&session_file).await;

        let result = start_iteration(client, arc_main_ctx.clone(), main_mut_context.clone()).await;

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

    delete_lock_file(output_dir.as_path());

    let output_dir = path_or_default_output(&opts.output);
    let path_string = format!("{}/long_messages_result.json", output_dir.display());
    let path = Path::new(path_string.as_str());
    let file = File::create(path).unwrap();
    if let Ok(ctx) = main_mut_context.clone().read() {
        serde_json::to_writer_pretty(&file, &ctx.long_messages_info).unwrap();
    }
}

fn need_auth(session_file: &Option<String>) -> bool {
    let path_result = path_or_default(session_file);
    let path = if let Ok(path) = path_result {
        path
    } else {
        return true;
    };
    !path.exists()
}

async fn auth(session_file_path: Option<String>, session_file_name: String) {
    let api_id = env!("TG_ID").parse().expect("TG_ID invalid");
    let api_hash = env!("TG_HASH").to_string();

    let path_result = make_path(session_file_path, session_file_name);
    let path = if let Ok(path) = path_result {
        path
    } else {
        return;
    };

    log::info!("Connecting to Telegram...");
    let mut client = Client::connect(Config {
        session: Session::load_file_or_create(path.as_path()).unwrap(),
        api_id,
        api_hash: api_hash.clone(),
        params: Default::default(),
    })
    .await
    .unwrap();
    log::info!("Connected!");

    if !client.is_authorized().await.unwrap() {
        log::info!("Signing in...");
        let phone = prompt("Enter your phone number (international format): ").unwrap();
        let token = client
            .request_login_code(&phone, api_id, &api_hash)
            .await
            .unwrap();
        let code = prompt("Enter the code you received: ").unwrap();
        let signed_in = client.sign_in(&token, &code).await;
        match signed_in {
            Err(SignInError::PasswordRequired(password_token)) => {
                // Note: this `prompt` method will echo the password in the console.
                //       Real code might want to use a better way to handle this.
                let hint = password_token.hint().unwrap();
                let prompt_message = format!("Enter the password (hint {}): ", &hint);
                let password = prompt(prompt_message.as_str()).unwrap();

                client
                    .check_password(password_token, password.trim())
                    .await
                    .unwrap();
            }
            Ok(_) => (),
            Err(e) => panic!("{}", e),
        };
        log::info!("Signed in!");
        log::info!("Create session file under {:?}", path.as_path());
        println!("Create session file under {:?}", path.as_path());
        match client.session().save_to_file(path.as_path()) {
            Ok(_) => {}
            Err(e) => {
                log::error!(
                    "NOTE: failed to save the session, will sign out when done: {}",
                    e
                );
            }
        }
    }
}

fn path_or_default(session_file: &Option<String>) -> Result<PathBuf, ()> {
    if let Some(path) = session_file {
        let mut path_buf = PathBuf::new();
        path_buf.push(shellexpand::tilde(path).into_owned());
        Ok(path_buf)
    } else {
        let default_file_path = default_file_path();
        let mut file_path = match default_file_path {
            Ok(path) => path,
            Err(error) => {
                log::error!("{}", error);
                return Err(());
            }
        };

        let _ = fs::create_dir_all(file_path.as_path());

        file_path.push(DEFAULT_FILE_NAME.to_string());
        Ok(file_path)
    }
}

pub fn path_or_default_output(folder: &Option<String>) -> PathBuf {
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

fn default_file_path() -> Result<PathBuf, String> {
    let os = env::consts::OS;
    let mut home = match home::home_dir() {
        Some(home) => home,
        None => {
            return Err(String::from(
                "Please specify session file path using --session-file-path option",
            ));
        }
    };
    let folder = if os == "linux" {
        String::from(".tg_backup")
    } else if os == "macos" {
        String::from(".tg_backup")
    } else if os == "windows" {
        String::from(".tg_backup")
    } else {
        return Err(String::from(
            "Please specify session file path using --session-file-path option",
        ));
    };
    home.push(folder);
    return Ok(home);
}

fn make_path(session_file_path: Option<String>, session_file_name: String) -> Result<PathBuf, ()> {
    let mut file_path = if let Some(file_path) = session_file_path {
        let mut buf = PathBuf::new();
        buf.push(shellexpand::tilde(file_path.as_str()).into_owned());
        buf
    } else {
        let default_file_path = default_file_path();
        match default_file_path {
            Ok(path) => path,
            Err(error) => {
                log::error!("{}", error);
                return Err(());
            }
        }
    };

    let _ = fs::create_dir_all(file_path.as_path());

    file_path.push(session_file_name);
    Ok(file_path)
}

type MyResult<T> = Result<T, Box<dyn std::error::Error>>;

fn prompt(message: &str) -> MyResult<String> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    stdout.write_all(message.as_bytes())?;
    stdout.flush()?;

    let stdin = io::stdin();
    let mut stdin = stdin.lock();

    let mut line = String::new();
    stdin.read_line(&mut line)?;
    Ok(line)
}

fn create_lock_file(output_dir: &Path) -> bool {
    let lock_file_path = output_dir.join("file.lock");
    let lock_file_exists = lock_file_path.exists();
    if lock_file_exists {
        let pid: u32 = fs::read_to_string(lock_file_path.as_path())
            .map(|x| x.parse().unwrap_or(0))
            .unwrap_or(0);
        let all_processes = System::new_all()
            .get_processes()
            .keys()
            .cloned()
            .map(|x: Pid| x.as_u32())
            .collect::<HashSet<u32>>();
        let process_exists = all_processes.contains(&pid);
        if process_exists {
            false
        } else {
            let pid = std::process::id().to_string();
            let _ = fs::write(lock_file_path, pid);
            true
        }
    } else {
        let pid = std::process::id().to_string();
        let lock_file = File::create(lock_file_path.as_path());
        if let Ok(_) = lock_file {
            fs::write(lock_file_path.as_path(), pid).unwrap();
        }
        true
    }
}

fn delete_lock_file(output_dir: &Path) {
    let lock_file_path = output_dir.join("file.lock");
    let _ = fs::remove_file(lock_file_path);
}

async fn get_connection(session_file: &Option<String>) -> Client {
    let mut counter = 0;
    loop {
        let connect = create_connection(session_file).await;
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

async fn create_connection(session_file: &Option<String>) -> Result<Client, AuthorizationError> {
    let api_id = env!("TG_ID").parse().expect("TG_ID invalid");
    let api_hash = env!("TG_HASH").to_string();

    let path_result = path_or_default(session_file);
    let path = path_result.expect("Session file expected to be existed at this moment");

    log::info!("Connecting to Telegram...");
    let client = Client::connect(Config {
        session: Session::load_file(path).unwrap(),
        api_id,
        api_hash: api_hash.clone(),
        params: Default::default(),
    })
    .await?;
    log::info!("Connected!");

    let client_handle = client.clone();

    tokio::spawn(async move { client.run_until_disconnected().await });
    Ok(client_handle)
}

async fn save_me(client: &mut Client, main_context: &MainContext) {
    let me_result: Result<Member, InvocationError> = client.get_me().await.map(|it| it.into());
    match me_result {
        Ok(me) => {
            let path_string = format!("{}/me.json", main_context.output_dir.display().to_string());
            let path = Path::new(path_string.as_str());
            if !path.exists() {
                let file = File::create(path).unwrap();
                serde_json::to_writer_pretty(&file, &me).unwrap();
            } else {
                let file = BufReader::new(File::open(&path).unwrap());
                let existing_data: Member = serde_json::from_reader(file).unwrap();
                if existing_data != me {
                    let members_file = File::create(path).unwrap();
                    serde_json::to_writer_pretty(&members_file, &me).unwrap();
                }
            }
        }
        Err(e) => {
            log::error!("Cannot save information about me: {}", e)
        }
    }
}

pub async fn get_me(opts: &Opts) -> Option<Member> {
    let output_dir = path_or_default_output(&opts.output);
    let path_string = format!("{}/me.json", output_dir.display());
    let path = Path::new(path_string.as_str());
    return File::open(&path)
        .map(|it| BufReader::new(it))
        .ok()
        .and_then(|it| serde_json::from_reader::<BufReader<File>, Member>(it).ok());
}

async fn start_iteration(
    client: Client,
    main_ctx: Arc<MainContext>,
    main_mut_ctx: Arc<RwLock<MainMutContext>>,
) -> Result<(), ()> {
    let mut dialogs_iter = client.iter_dialogs();
    if let Ok(mut ctx) = main_mut_ctx.write() {
        if let None = ctx.amount_of_dialogs {
            let total_dialogs_count = dialogs_iter.total().await;
            if let Ok(dialogs_count) = total_dialogs_count {
                ctx.amount_of_dialogs = Some(dialogs_count);
                log::info!("Saving {} dialogs", dialogs_count);
                if !main_ctx.quite_mode {
                    println!("Saving {} dialogs", dialogs_count);
                }
            }
        }
    }
    loop {
        let dialog_res = dialogs_iter.next().await;
        match dialog_res {
            Ok(Some(dialog)) => {
                let local_client = client.clone();

                // TODO okay, this should be executed in an async manner, but it doesn't work
                //   not sure why. So let's leave it sync.
                let my_main_context = main_ctx.clone();
                let my_main_mut_context = main_mut_ctx.clone();
                let result = task::spawn(async move {
                    extract_dialog(local_client, dialog, my_main_context, my_main_mut_context).await
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
    chats: Vec<i64>,
    excluded: Vec<i64>,
    batch_size: i32,
    output_dir: &Path,
    quite_mode: bool,
    file_limit: Option<i32>,
    max_participants: i32,
    test: bool,
) -> MainContext {
    let loading_chats = if chats.is_empty() { None } else { Some(chats) };
    let mut main_context = MainContext::init(
        loading_chats,
        excluded,
        batch_size,
        output_dir.clone().to_path_buf(),
        quite_mode,
        file_limit.map(|x| x * 1024 * 1024),
        max_participants,
        test,
    );

    let path_string = format!("{}/backup.json", output_dir.display());
    let path = Path::new(path_string.as_str());
    if path.exists() {
        let open_file = File::open(path);
        if let Ok(file) = open_file {
            let file = BufReader::new(file);
            let parsed_file: Result<BackUpInfo, _> = serde_json::from_reader(file);
            if let Ok(data) = parsed_file {
                main_context.date_from = Some(data.date)
            }
        } else {
            let _ = fs::remove_file(path);
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
    client: Client,
    dialog: Dialog,
    main_ctx: Arc<MainContext>,
    main_mut_ctx: Arc<RwLock<MainMutContext>>,
) -> Result<(), ()> {
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

    if chat.skip_backup() {
        return Ok(());
    }

    if let Ok(ctx) = main_mut_ctx.read() {
        let item = ctx
            .long_messages_info
            .iter()
            .filter(|item| item.rec == chat_id)
            .next();
        if let Some(item) = item {
            if item.state != BACKUP {
                log::info!(
                    "Skip chat. name: {} id: {} because it has more than {} participants",
                    chat_name,
                    chat_id,
                    main_ctx.max_participants
                );
                return Ok(());
            }
        }
    }
    let chat_participants = chat.members(&client).await.iter().count() as i32;
    if main_ctx.max_participants >= 0 && chat_participants > main_ctx.max_participants {
        if let Ok(mut ctx) = main_mut_ctx.write() {
            let item = ctx
                .long_messages_info
                .iter()
                .filter(|item| item.rec == chat_id)
                .next();
            match item {
                Some(_) => {}
                None => {
                    ctx.long_messages_info.push(ChannelsStateInfo {
                        rec: chat_id,
                        name: chat_name.to_string(),
                        state: ASK,
                    });
                    info!(
            "Skip chat. name: {} id: {} because it has more than {} participants. Asked for permission to save",
            chat_name,
            chat_id,
            main_ctx.max_participants
        );
                    return Ok(());
                }
            }
        }
    }

    let visual_id = chat.visual_id();

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

    let latest_file = get_last_file(chat_path);
    let existing_data: Vec<MessageInfo> = if let Some(entry) = &latest_file {
        let file = BufReader::new(File::open(&entry.path()).unwrap());
        let result: Result<Vec<MessageInfo>, Error> = serde_json::from_reader(file);
        match result {
            Ok(existing_data) => {
                if existing_data.len() as i32 >= main_ctx.batch_size {
                    vec![]
                } else {
                    existing_data
                }
            }
            Err(err) => {
                log::error!(
                    "Cannot parse previous data from {:?}, error {}",
                    entry.path(),
                    err
                );
                vec![]
            }
        }
    } else {
        vec![]
    };

    let mut chat_ctx = ChatContext::init(chat_path, visual_id.clone(), existing_data);
    chat_ctx.initial_file = latest_file.map(|x| x.path());

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

    let mut iter_messages = client
        .iter_messages(dialog.chat())
        .offset_date(start_loading_time.timestamp() as i32);
    if let Some(id) = last_loaded_id {
        iter_messages = iter_messages.offset_id(id);
    }

    let mut last_message: Option<(i32, DateTime<Utc>)> = None;
    let total_messages = iter_messages.total().await.unwrap_or(0);
    let amount_of_messages_to_load = total_messages - amount_of_already_loaded_messages;

    // Save info file
    let info_file = File::create(info_file_path).unwrap();
    serde_json::to_writer_pretty(
        &info_file,
        &chat_to_info(&chat, global_loading_from, total_messages),
    )
    .unwrap();

    // Save members
    let members_await = chat.members(&client);
    let members = members_await.await;
    let members_folder = chat_path.join("members");
    let _ = fs::create_dir(&members_folder);
    let members_path = members_folder.join("members.json");
    if !members_path.exists() {
        let members_file = File::create(members_path).unwrap();
        serde_json::to_writer_pretty(&members_file, &members).unwrap();
    } else {
        let file = BufReader::new(File::open(&members_path).unwrap());
        let existing_data: Vec<Member> = serde_json::from_reader(file).unwrap();
        if existing_data != members {
            let members_file = File::create(members_path).unwrap();
            serde_json::to_writer_pretty(&members_file, &members).unwrap();
        }
    }

    // Initialize progress bar
    let mut pb = ProgressBar::new(amount_of_messages_to_load as u64);
    pb.message(format!("Loading {} [messages] ", visual_id).as_str());
    if !main_ctx.quite_mode {
        chat_ctx.pb = Some(pb);
    }

    log::info!(
        "Start loading loop. Counter: {}, total_size: {}, from: {}, until: {:?}",
        counter,
        amount_of_messages_to_load,
        start_loading_time,
        end_loading_time,
    );

    let mut pivot_time = Utc::now();

    loop {
        let msg = iter_messages.next().await;
        match msg {
            Ok(Some(message)) => {
                let message_date = message.date();
                let message_id = message.id();
                if let Some(end_time) = end_loading_time {
                    if message_date < end_time {
                        chat_ctx.force_drop_messages(&main_ctx);
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
                let saving_result = save_message(&message, &mut chat_ctx, &main_ctx).await;
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
                    chat_ctx.force_drop_messages(&main_ctx);
                    return Err(());
                }

                let current_time = Utc::now();
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
                chat_ctx.force_drop_messages(&main_ctx);
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
            Err(InvocationError::Rpc(RpcError {
                code: _,
                name,
                value,
                ..
            })) => {
                if name == "FLOOD_WAIT" {
                    let wait_time = value.unwrap();
                    let total_wait = if let Ok(mut ctx) = main_mut_ctx.write() {
                        ctx.total_flood_wait += wait_time;
                        ctx.total_flood_wait
                    } else {
                        0
                    };
                    log::warn!(
                        "Flood wait: {}, total flood wait: {}",
                        wait_time,
                        total_wait
                    );
                    sleep(Duration::from_secs(wait_time as u64))
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

    chat_ctx.force_drop_messages(&main_ctx);

    // if let Some(pb) = chat_ctx.pb.as_mut() {
    //     pb.finish();
    // }
    Ok(())
}

fn get_last_file(chat_path: &Path) -> Option<DirEntry> {
    let messages_path = chat_path.join("messages");
    if !messages_path.exists() {
        return None;
    }
    let latest_dir = fs::read_dir(messages_path)
        .unwrap()
        .max_by(|left, right| {
            compare_by_names(
                left.as_ref()
                    .map_or(String::from(""), |dir| {
                        String::from(dir.file_name().to_str().unwrap())
                    })
                    .as_str(),
                right
                    .as_ref()
                    .map_or(String::from(""), |dir| {
                        String::from(dir.file_name().to_str().unwrap())
                    })
                    .as_str(),
            )
        })?
        .unwrap();
    Some(latest_dir)
}

fn compare_by_names(first_name: &str, second_name: &str) -> Ordering {
    let regex = Regex::new(r"data-\d{8}-(\d{8})(-(\d+))?\.json").expect("Incorrect regex");
    let first_date: i32 = regex
        .captures(first_name)
        .unwrap()
        .get(1)
        .map(|x| x.as_str().parse().unwrap())
        .unwrap();
    let second_date: i32 = regex
        .captures(second_name)
        .unwrap()
        .get(1)
        .map(|x| x.as_str().parse().unwrap())
        .unwrap();
    let cmd_result = first_date.cmp(&second_date);
    if cmd_result == Ordering::Equal {
        let first_shift: i32 = regex
            .captures(first_name)
            .unwrap()
            .get(3)
            .map_or(0, |x| x.as_str().parse().unwrap());
        let second_shift: i32 = regex
            .captures(second_name)
            .unwrap()
            .get(3)
            .map_or(0, |x| x.as_str().parse().unwrap());
        first_shift.cmp(&second_shift)
    } else {
        cmd_result
    }
}

async fn save_message(
    message: &Message,
    chat_ctx: &mut ChatContext,
    main_ctx: &Arc<MainContext>,
) -> Result<(), ()> {
    let message_text = message.text();
    let types = &chat_ctx.types;
    let option_photo = message.photo();
    let option_document = message.document();
    let option_geo = message.geo();
    let option_geo_live = message.geo_live();
    let option_dice = message.dice();
    let option_contact = message.contact();
    let attachment = if let Some(photo) = option_photo {
        if let Some(pb) = chat_ctx.pb.as_mut() {
            pb.message(format!("Loading {} [photo   ] ", chat_ctx.visual_id.as_str()).as_str());
        }
        log::debug!("Loading photo {}", message_text);
        let current_type = types.get(PHOTO).unwrap();
        let photo_id = photo.id();
        if let Some(id) = photo_id {
            let file_name = format!("{}@photo.jpg", id);
            let photos_path = current_type.path().join(file_name.as_str());
            let photo_size = photo.thumbs().largest().unwrap().size();

            if main_ctx.max_attachment_size_in_bytes.is_some()
                && main_ctx.max_attachment_size_in_bytes.unwrap() < photo_size as i32
            {
                Some(TooLarge {
                    size: photo_size as i32,
                })
            } else {
                let downloaded = photo
                    .thumbs()
                    .largest()
                    .unwrap()
                    .download(photos_path)
                    .await;
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
            }
        } else {
            Some(PhotoExpired)
        }
    } else if let Some(mut doc) = option_document {
        if main_ctx.max_attachment_size_in_bytes.is_some()
            && main_ctx.max_attachment_size_in_bytes.unwrap() < doc.size() as i32
        {
            Some(TooLarge {
                size: doc.size() as i32,
            })
        } else {
            let doc_id = doc.id();
            let doc_name = doc.name().to_string();
            let (attachment, file_path) = if doc.is_round_message() {
                if let Some(pb) = chat_ctx.pb.as_mut() {
                    pb.message(
                        format!("Loading {} [round   ] ", chat_ctx.visual_id.as_str()).as_str(),
                    );
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
                    pb.message(
                        format!("Loading {} [voice   ] ", chat_ctx.visual_id.as_str()).as_str(),
                    );
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
                    pb.message(
                        format!("Loading {} [file    ] ", chat_ctx.visual_id.as_str()).as_str(),
                    );
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
        }
    } else if let Some(geo) = option_geo {
        Some(Attachment::Geo(geo))
    } else if let Some(geo) = option_geo_live {
        Some(Attachment::GeoLive(geo))
    } else if let Some(dice) = option_dice {
        Some(Attachment::Dice(dice))
    } else if let Some(contact) = option_contact {
        Some(Attachment::Contact(contact))
    } else {
        None
    };

    let action = get_action(message);
    log::debug!("Loading message {}", message_text);
    let message = msg_to_info(&message, attachment, action);
    chat_ctx.messages_accumulator.push(message);
    Ok(())
}

fn get_action(message: &Message) -> Option<Action> {
    let action: &tl::enums::MessageAction = message.action()?;
    let result = match action {
        tl::enums::MessageAction::PhoneCall(call) => call.into(),
        tl::enums::MessageAction::ChatCreate(chat_create) => chat_create.into(),
        tl::enums::MessageAction::ChatEditTitle(edit_title) => edit_title.into(),
        tl::enums::MessageAction::GroupCall(group_call) => group_call.into(),
        tl::enums::MessageAction::InviteToGroupCall(invite_to_group_call) => {
            invite_to_group_call.into()
        }
        tl::enums::MessageAction::HistoryClear => Action::HistoryClear {},
        _ => Action::UnsupportedByTgBackup(format!("{:?}", action)),
    };
    Some(result)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_less() {
        let ordering =
            compare_by_names("data-20200101-20200101.json", "data-20200101-20200102.json");
        assert_eq!(Ordering::Less, ordering)
    }

    #[test]
    fn test_greater() {
        let ordering =
            compare_by_names("data-20200101-20200102.json", "data-20200101-20200101.json");
        assert_eq!(Ordering::Greater, ordering)
    }

    #[test]
    fn test_eq() {
        let ordering =
            compare_by_names("data-20200101-20200101.json", "data-20200101-20200101.json");
        assert_eq!(Ordering::Equal, ordering)
    }

    #[test]
    fn test_eq_with_adding() {
        let ordering = compare_by_names(
            "data-20200101-20200101.json",
            "data-20200101-20200101-1.json",
        );
        assert_eq!(Ordering::Less, ordering)
    }

    #[test]
    fn test_eq_with_adding_for_both() {
        let ordering = compare_by_names(
            "data-20200101-20200101-1.json",
            "data-20200101-20200101-2.json",
        );
        assert_eq!(Ordering::Less, ordering)
    }

    #[test]
    fn find_mat() {
        let files = vec![
            "data-20200101-20200101-1.json",
            "data-20200101-20200101-2.json",
            "data-20200101-20200102.json",
        ];
        let max = files
            .iter()
            .max_by(|left, right| compare_by_names(left, right))
            .unwrap();
        assert_eq!("data-20200101-20200102.json", *max)
    }
}
