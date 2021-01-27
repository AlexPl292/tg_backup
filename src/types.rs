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
    photo: i64,
}

pub fn msg_to_info(data: &Message) -> MessageInfo {
    MessageInfo {
        text: data.text().to_string(),
        id: data.id(),
        date: data.date(),
    }
}

pub fn msg_to_photo_info(data: &Message, photo: &Photo) -> MessageWithPhoto {
    MessageWithPhoto {
        text: data.text().to_string(),
        id: data.id(),
        date: data.date(),
        photo: photo.id(),
    }
}

pub fn chat_to_info(data: &Chat) -> ChatInfo {
    ChatInfo {
        name: data.name().to_string(),
        id: data.id(),
    }
}
