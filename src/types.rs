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
pub struct MessageWithPhoto {
    text: String,
    id: i32,
    date: DateTime<Utc>,
    photo: PhotoInfo,
}

#[derive(Serialize, Deserialize)]
pub struct PhotoInfo {
    pub id: i64,
    pub path: String,
}

#[derive(Serialize, Deserialize)]
pub struct MessageWithFile {
    text: String,
    id: i32,
    date: DateTime<Utc>,
    file: i64,
}

pub fn msg_to_info(data: &Message) -> MessageInfo {
    MessageInfo {
        text: data.text().to_string(),
        id: data.id(),
        date: data.date(),
    }
}

pub fn msg_to_photo_info(data: &Message, photo: PhotoInfo) -> MessageWithPhoto {
    MessageWithPhoto {
        text: data.text().to_string(),
        id: data.id(),
        date: data.date(),
        photo,
    }
}

pub fn msg_to_file_info(data: &Message, uploaded: i64) -> MessageWithFile {
    MessageWithFile {
        text: data.text().to_string(),
        id: data.id(),
        date: data.date(),
        file: uploaded,
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
