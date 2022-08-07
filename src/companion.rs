/*
 * tg_backup - backup your messages from the Telegram messenger
 * Copyright 2021-2022 Alex Plate
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

use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use regex::Regex;
use serde::{Deserialize, Serialize};
use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup, UpdateKind};

use crate::backup::{get_me, path_or_default_output};
use crate::companion::ChannelState::{ASK, ASKED};
use crate::opts::Opts;
use crate::types::Member;

pub async fn ask(opts: &Opts) -> Vec<ChannelsStateInfo> {
    let mut current_info = read_current_state(&opts).unwrap_or(vec![]);

    let bot = Bot::from_env().auto_send();

    let updates = bot.get_updates().timeout(0).send();
    let result = updates.await;

    let info: Vec<(i64, bool)> = result
        .unwrap_or(vec![])
        .iter()
        .filter_map(|item| match &item.kind {
            UpdateKind::Message(info) => {
                let regex = Regex::new(r"^(\d+): (yes|no).*").unwrap();
                let text = info.text().unwrap_or("");
                if let Some(cap) = regex.captures(text) {
                    let res = match &cap[2] {
                        "yes" => true,
                        "no" => false,
                        _ => panic!(" ---> {:?}", &cap[2]),
                    };
                    Some((cap[1].parse::<i64>().unwrap(), res))
                } else {
                    None
                }
            }
            _ => None,
        })
        .collect();

    for x in info {
        current_info.iter_mut().for_each(|item| {
            if item.rec == x.0 {
                item.state = if x.1 {
                    ChannelState::BACKUP
                } else {
                    ChannelState::SKIP
                }
            }
        })
    }

    let me = get_me(opts).await;
    if let Some(member) = me {
        if let Member::User { id, .. } = member {
            for item in current_info.iter_mut() {
                if item.state == ASK {
                    bot.send_message(UserId(id as u64), format!("{}, {} ?", item.name, item.rec))
                        .await
                        .expect("TODO: panic message");
                    item.state = ASKED
                }
            }
        }
    }

    return current_info;
}

fn read_current_state(opts: &Opts) -> Option<Vec<ChannelsStateInfo>> {
    let output_dir = path_or_default_output(&opts.output);
    let path_string = format!("{}/long_messages_result.json", output_dir.display());
    let path = Path::new(path_string.as_str());
    if path.exists() {
        let open_file = File::open(path);
        if let Ok(file) = open_file {
            let file = BufReader::new(file);
            let parsed_file: Result<Vec<ChannelsStateInfo>, _> = serde_json::from_reader(file);
            if let Ok(data) = parsed_file {
                return Some(data);
            }
        } else {
            let _ = fs::remove_file(path);
        }
    }
    return None;
}

#[derive(Serialize, Deserialize, Eq, PartialEq)]
pub struct ChannelsStateInfo {
    pub rec: i64,
    pub name: String,
    pub state: ChannelState,
}

#[derive(Serialize, Deserialize, Eq, PartialEq)]
pub enum ChannelState {
    ASK,
    ASKED,
    BACKUP,
    SKIP,
}

fn _make_keyboard() -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    let debian_versions = ["yes", "no"];

    for versions in debian_versions.chunks(3) {
        let row = versions
            .iter()
            .map(|&version| InlineKeyboardButton::callback(version.to_owned(), version.to_owned()))
            .collect();

        keyboard.push(row);
    }

    InlineKeyboardMarkup::new(keyboard)
}
