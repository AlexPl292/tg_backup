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

use crate::types::{ContactInfo, ForwardInfo, GeoInfo, GeoLiveInfo, Member, ReplyInfo};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDateTime, Utc};
use grammers_client::client::auth::InvocationError;
use grammers_client::types::{Chat, Media, Message};
use grammers_client::Client;
use grammers_mtproto::mtp::RpcError;
use grammers_tl_types as tl;
use std::thread::sleep;
use std::time::Duration;

pub trait MessageExt {
    fn geo(&self) -> Option<GeoInfo>;
    fn geo_live(&self) -> Option<GeoLiveInfo>;
    fn contact(&self) -> Option<ContactInfo>;
    fn fwd_from(&self) -> Option<ForwardInfo>;
    fn reply_to(&self) -> Option<ReplyInfo>;
}

impl MessageExt for Message {
    fn geo(&self) -> Option<GeoInfo> {
        let media = self.media();
        if let Some(Media::Geo(geo)) = media {
            geo.point().map(|it| it.into())
        } else {
            None
        }
    }

    fn geo_live(&self) -> Option<GeoLiveInfo> {
        let media = self.media();
        if let Some(Media::GeoLive(geo)) = media {
            Some(GeoLiveInfo {
                point: geo.point().map(|it| it.into()),
                heading: geo.heading(),
                period: geo.period(),
                proximity_notification_radius: geo.proximity_notification_radius(),
            })
        } else {
            None
        }
    }

    fn contact(&self) -> Option<ContactInfo> {
        let media = self.media();
        if let Some(Media::Contact(contact)) = media {
            Some(ContactInfo {
                first_name: contact.first_name().to_string(),
                last_name: contact.last_name().to_string(),
                phone_number: contact.phone_number().to_string(),
                vcard: contact.vcard().to_string(),
            })
        } else {
            None
        }
    }
    fn fwd_from(&self) -> Option<ForwardInfo> {
        let tl::enums::MessageFwdHeader::Header(data) = self.forward_header()?;
        let date =
            DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(data.date as i64, 0), Utc);
        let from_id = if let Some(from_id) = data.from_id {
            if let tl::enums::Peer::User(user) = from_id {
                Some(user.user_id)
            } else {
                None
            }
        } else {
            None
        };
        Some(ForwardInfo {
            from_id,
            from_name: data.from_name.clone(),
            date,
        })
    }

    fn reply_to(&self) -> Option<ReplyInfo> {
        self.reply_to_message_id()
            .map(|to_message_id| ReplyInfo { to_message_id })
    }
}

#[async_trait]
pub trait ChatExt: Send {
    fn id(&self) -> i32;
    fn name(&self) -> String;
    async fn members(&self, client: &Client) -> Vec<Member>;
    fn visual_id(&self) -> String;
    fn skip_backup(&self) -> bool;
}

#[async_trait]
impl ChatExt for Chat {
    fn id(&self) -> i32 {
        self.id()
    }

    fn name(&self) -> String {
        self.name().to_string()
    }

    async fn members(&self, client: &Client) -> Vec<Member> {
        let mut res = vec![];
        if let Chat::User(user) = &self {
            res.push(Member::Me);
            res.push(user.into());
        } else {
            let mut participant_iter = client.iter_participants(&self);
            loop {
                let next = participant_iter.next().await;
                match next {
                    Ok(Some(next_one)) => {
                        let member = next_one.user.into();
                        res.push(member);
                    }
                    Ok(None) => break,
                    Err(InvocationError::Rpc(RpcError {
                        name,
                        code: _,
                        value,
                        ..
                    })) => {
                        if name == "FLOOD_WAIT" {
                            log::warn!("Flood wait: {}", value.unwrap());
                            sleep(Duration::from_secs(value.unwrap() as u64))
                        } else if name == "FILE_MIGRATE" {
                            log::warn!("File migrate: {}", value.unwrap());
                        } else {
                            log::error!("Error {}, {:?}", name, value)
                        }
                    }
                    Err(e) => panic!("{}", e),
                }
            }
        }
        res
    }

    fn visual_id(&self) -> String {
        if let Chat::User(user) = &self {
            let username = user.username().unwrap_or("NO_USERNAME");
            format!("{}.{}", &self.name(), username)
        } else {
            format!("{}", &self.name())
        }
    }

    fn skip_backup(&self) -> bool {
        match self {
            Chat::User(_) => false,
            Chat::Group(_) => false,
            Chat::Channel(_) => true,
        }
    }
}
