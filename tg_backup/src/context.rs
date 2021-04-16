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

use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};

use crate::attachment_type::AttachmentType;
use crate::types::MessageInfo;
use chrono::{DateTime, Utc};
use pbr::ProgressBar;
use std::io::Stdout;

pub const MESSAGES: &'static str = "messages";
pub const PHOTO: &'static str = "photo";
pub const FILE: &'static str = "file";
pub const ROUND: &'static str = "round";
pub const VOICE: &'static str = "voice";

pub struct MainMutContext {
    pub(crate) already_finished: Vec<i32>,
    pub(crate) amount_of_dialogs: Option<usize>,
}

pub struct MainContext {
    pub(crate) date: DateTime<Utc>,
    pub(crate) date_from: Option<DateTime<Utc>>,
    pub(crate) batch_size: i32,
    pub(crate) included_chats: Option<Vec<i32>>,
    pub(crate) excluded_chats: Vec<i32>,
    pub(crate) output_dir: PathBuf,
    pub(crate) quite_mode: bool,
}

impl MainContext {
    pub fn init(
        loading_chats: Option<Vec<i32>>,
        excluded_chats: Vec<i32>,
        batch_size: i32,
        output_dir: PathBuf,
        quite_mode: bool,
    ) -> MainContext {
        MainContext {
            date: chrono::offset::Utc::now(),
            date_from: None,
            batch_size,
            included_chats: loading_chats,
            excluded_chats,
            output_dir,
            quite_mode,
        }
    }
}

pub struct ChatContext {
    pub(crate) types: HashMap<String, AttachmentType>,
    pub(crate) messages_accumulator: Vec<MessageInfo>,
    pub(crate) accumulator_counter: i32,
    pub(crate) pb: Option<ProgressBar<Stdout>>,
    pub(crate) visual_id: String,

    pub(crate) file_issue: i64,
    pub(crate) file_issue_count: i32,
}

impl ChatContext {
    pub fn init(path: &Path, visual_id: String) -> ChatContext {
        let mut types = ChatContext::init_types();
        types.values_mut().for_each(|x| x.init_folder(path));
        ChatContext {
            types,
            messages_accumulator: vec![],
            accumulator_counter: 0,
            pb: None,
            visual_id,
            file_issue: 0,
            file_issue_count: 0,
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
            AttachmentType::init("media/photos", PHOTO, Some(".jpg")),
        );
        map.insert(
            FILE.to_string(),
            AttachmentType::init("media/files", FILE, None),
        );
        map.insert(
            ROUND.to_string(),
            AttachmentType::init("media/rounds", ROUND, Some(".mp4")),
        );
        map.insert(
            VOICE.to_string(),
            AttachmentType::init("media/voice_messages", VOICE, Some(".ogg")),
        );
        map
    }

    pub(crate) fn drop_messages(&mut self, main_ctx: &MainContext) -> bool {
        if self.messages_accumulator.len() < main_ctx.batch_size as usize {
            return false;
        }
        self.force_drop_messages();
        return true;
    }

    pub(crate) fn force_drop_messages(&mut self) {
        if self.messages_accumulator.is_empty() {
            return;
        }

        let data_type = self.types.get(MESSAGES).unwrap();
        let messages_path = data_type.path();
        let first_msg = self
            .messages_accumulator
            .first()
            .unwrap()
            .date
            .format("%Y%m%d");
        let last_msg = self
            .messages_accumulator
            .last()
            .unwrap()
            .date
            .format("%Y%m%d");
        let mut file_path = messages_path.join(format!("data-{}-{}.json", first_msg, last_msg));

        let mut counter = 0;
        while file_path.exists() {
            file_path =
                messages_path.join(format!("data-{}-{}-{}.json", first_msg, last_msg, counter));
            counter += 1;
        }

        let file = File::create(file_path).unwrap();
        serde_json::to_writer_pretty(&file, &self.messages_accumulator).unwrap();

        self.messages_accumulator.clear();
        self.accumulator_counter += 1;
    }
}
