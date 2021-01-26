use grammers_client::types::{Chat, Message};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ChatInfo {
    name: String,
    id: i32,
}

#[derive(Serialize, Deserialize)]
pub struct MessageInfo {
    text: String,
    id: i32,
}

pub fn msg_to_info(data: &Message) -> MessageInfo {
    MessageInfo {
        text: data.text().to_string(),
        id: data.id(),
    }
}

pub fn chat_to_info(data: &Chat) -> ChatInfo {
    ChatInfo {
        name: data.name().to_string(),
        id: data.id(),
    }
}
