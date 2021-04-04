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

use grammers_client::types::{Chat, Message};
use serde::{Deserialize, Serialize};

use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize)]
pub struct ChatInfo {
    pub name: String,
    pub id: i32,
    pub loaded_up_to: DateTime<Utc>,
    pub total_messages: usize,
}

#[derive(Serialize, Deserialize)]
pub struct MessageInfo {
    text: String,
    id: i32,
    pub date: DateTime<Utc>,
    attachment: Attachment,
    edit_date: Option<DateTime<Utc>>,
    mentioned: bool,
    outgoing: bool,
    pinned: bool,
    sender_id: Option<i32>,
    sender_name: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct FileInfo {
    pub id: i64,
    pub path: String,
}

#[derive(Serialize, Deserialize)]
pub enum Attachment {
    File(FileInfo),
    Photo(FileInfo),
    Voice(FileInfo),
    Round(FileInfo),
    PhotoExpired,
    None,
    Error(String),
}

pub fn msg_to_info(data: &Message) -> MessageInfo {
    MessageInfo {
        text: data.text().to_string(),
        id: data.id(),
        date: data.date(),
        attachment: Attachment::None,
        edit_date: data.edit_date(),
        mentioned: data.mentioned(),
        outgoing: data.outgoing(),
        pinned: data.pinned(),
        sender_id: data.sender().map(|x| x.id()),
        sender_name: data.sender().map(|x| x.name().to_string()),
    }
}

pub fn msg_to_file_info(data: &Message, attachment: Attachment) -> MessageInfo {
    MessageInfo {
        text: data.text().to_string(),
        id: data.id(),
        date: data.date(),
        attachment,
        edit_date: data.edit_date(),
        mentioned: data.mentioned(),
        outgoing: data.outgoing(),
        pinned: data.pinned(),
        sender_id: data.sender().map(|x| x.id()),
        sender_name: data.sender().map(|x| x.name().to_string()),
    }
}

pub fn chat_to_info(data: &Chat, loaded_up_to: DateTime<Utc>, total_messages: usize) -> ChatInfo {
    ChatInfo {
        name: data.name().to_string(),
        id: data.id(),
        loaded_up_to,
        total_messages,
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BackUpInfo {
    pub date: DateTime<Utc>,
    pub batch_size: i32,
    pub included_chats: Option<Vec<i32>>,
    pub excluded_chats: Vec<i32>,
}

impl BackUpInfo {
    pub fn init(
        date: DateTime<Utc>,
        loading_chats: Option<Vec<i32>>,
        excluded_chats: Vec<i32>,
        batch_size: i32,
    ) -> BackUpInfo {
        BackUpInfo {
            date,
            batch_size,
            included_chats: loading_chats,
            excluded_chats,
        }
    }
}
