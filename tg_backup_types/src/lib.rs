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

use grammers_client::types::User;
use serde::{Deserialize, Serialize};

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
