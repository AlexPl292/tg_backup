use grammers_client::types::{Chat, Message, Photo};
use serde::{Deserialize, Serialize};

use chrono::{DateTime, Utc};

#[derive(Serialize, Deserialize)]
pub struct ChatInfo {
    name: String,
    id: i32,
}

#[derive(Serialize, Deserialize)]
pub struct MessageInfo {
    text: String,
    id: i32,
    date: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
pub struct MessageWithFile {
    text: String,
    id: i32,
    date: DateTime<Utc>,
    attachment: FileInfo,
}

#[derive(Serialize, Deserialize)]
pub struct FileInfo {
    pub id: i64,
    pub attachment_type: String,
    pub path: String,
}

pub fn msg_to_info(data: &Message) -> MessageInfo {
    MessageInfo {
        text: data.text().to_string(),
        id: data.id(),
        date: data.date(),
    }
}

pub fn msg_to_file_info(data: &Message, file: FileInfo) -> MessageWithFile {
    MessageWithFile {
        text: data.text().to_string(),
        id: data.id(),
        date: data.date(),
        attachment: file,
    }
}

pub fn chat_to_info(data: &Chat) -> ChatInfo {
    ChatInfo {
        name: data.name().to_string(),
        id: data.id(),
    }
}

#[derive(Serialize, Deserialize)]
pub struct BackUpInfo {
    date: DateTime<Utc>,
}

impl BackUpInfo {
    pub fn current_info() -> BackUpInfo {
        BackUpInfo {
            date: chrono::offset::Utc::now(),
        }
    }
}
