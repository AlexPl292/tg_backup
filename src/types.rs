use grammers_client::types::{Chat, Message};
use serde::{Deserialize, Serialize};

use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize)]
pub struct ChatInfo {
    name: String,
    id: i32,
    pub loaded_up_to: DateTime<Utc>,
    total_messages: usize,
}

#[derive(Serialize, Deserialize)]
pub struct MessageInfo {
    text: String,
    id: i32,
    pub date: DateTime<Utc>,
    attachment: Attachment,
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
    None,
    Error(String),
}

pub fn msg_to_info(data: &Message) -> MessageInfo {
    MessageInfo {
        text: data.text().to_string(),
        id: data.id(),
        date: data.date(),
        attachment: Attachment::None,
    }
}

pub fn msg_to_file_info(data: &Message, attachment: Attachment) -> MessageInfo {
    MessageInfo {
        text: data.text().to_string(),
        id: data.id(),
        date: data.date(),
        attachment,
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
