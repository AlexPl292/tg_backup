use grammers_client::types::{Chat, Message, User};
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

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Member {
    Me,
    User {
        id: i32,
        username: Option<String>,
        first_name: String,
        last_name: Option<String>,
        verified: bool,
        contact: bool,
        mutual_contact: bool,
        deleted: bool,
    },
}

impl From<User> for Member {
    fn from(user: User) -> Self {
        Member::User {
            id: user.id(),
            username: user.username().map(|x| x.to_string()),
            first_name: user.first_name().to_string(),
            last_name: user.last_name().map(|x| x.to_string()),
            verified: user.verified(),
            contact: user.contact(),
            mutual_contact: user.mutual_contact(),
            deleted: user.deleted(),
        }
    }
}

// TODO Omg rust, I don't know how to do it better
impl From<&User> for Member {
    fn from(user: &User) -> Self {
        Member::User {
            id: user.id(),
            username: user.username().map(|x| x.to_string()),
            first_name: user.first_name().to_string(),
            last_name: user.last_name().map(|x| x.to_string()),
            verified: user.verified(),
            contact: user.contact(),
            mutual_contact: user.mutual_contact(),
            deleted: user.deleted(),
        }
    }
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
