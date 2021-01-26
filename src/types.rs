use grammers_client::types::{Chat, Message};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ChatInfo {
    name: String,
    id: i32,
}

impl From<Chat> for ChatInfo {
    fn from(data: Chat) -> Self {
        ChatInfo {
            name: data.name().to_string(),
            id: data.id(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct MessageInfo {
    text: String,
    id: i32,
}

impl From<Message> for MessageInfo {
    fn from(data: Message) -> Self {
        MessageInfo {
            text: data.text().to_string(),
            id: data.id(),
        }
    }
}
