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

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::actions::Action;
use crate::ext::MessageExt;
use grammers_client::types::media::GeoPoint;
use grammers_client::types::{Chat, Message, User};

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
    attachment: Option<Attachment>,
    edit_date: Option<DateTime<Utc>>,
    mentioned: bool,
    outgoing: bool,
    pinned: bool,
    sender_id: Option<i32>,
    sender_name: Option<String>,
    forwarded_from: Option<ForwardInfo>,
    reply_to: Option<ReplyInfo>,
    action: Option<Action>,
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
    Geo(GeoInfo),
    GeoLive(GeoLiveInfo),
    Contact(ContactInfo),
    PhotoExpired,
    TooLarge { size: i32 },
    Error(String),
}

#[derive(Serialize, Deserialize)]
pub enum PhoneCallDiscardReason {
    PhoneCallDiscardReasonMissed,
    PhoneCallDiscardReasonDisconnect,
    PhoneCallDiscardReasonHangup,
    PhoneCallDiscardReasonBusy,
}

pub fn msg_to_info(
    data: &Message,
    attachment: Option<Attachment>,
    action: Option<Action>,
) -> MessageInfo {
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
        forwarded_from: data.fwd_from(),
        reply_to: data.reply_to(),
        action,
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

#[derive(Serialize, Deserialize, Eq, PartialEq)]
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
    IdOnly {
        id: i32,
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

#[derive(Serialize, Deserialize)]
pub struct ForwardInfo {
    pub from_id: Option<i32>,
    pub from_name: Option<String>,
    pub date: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
pub struct ReplyInfo {
    pub to_message_id: i32,
}

#[derive(Serialize, Deserialize)]
pub struct GeoInfo {
    pub longitude: f64,
    pub latitude: f64,
    pub accuracy_radius: Option<i32>,
}

impl From<GeoPoint> for GeoInfo {
    fn from(point: GeoPoint) -> Self {
        Self {
            latitude: point.latitude,
            longitude: point.longitude,
            accuracy_radius: point.accuracy_radius,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct GeoLiveInfo {
    pub point: Option<GeoInfo>,
    pub period: i32,
    pub heading: Option<i32>,
    pub proximity_notification_radius: Option<i32>,
}

#[derive(Serialize, Deserialize)]
pub struct ContactInfo {
    pub phone_number: String,
    pub first_name: String,
    pub last_name: String,
    pub vcard: String,
}
