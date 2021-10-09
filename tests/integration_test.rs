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

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::{fs, thread};
use std::time::Duration;
use grammers_client::{Client, Config, InputMessage};
use grammers_session::Session;
use tg_backup::opts::Opts;
use clap::Clap;
use serde_json::Value;

#[tokio::test]
async fn test_add() {
    let api_id = env!("TG_ID").parse().expect("TG_ID invalid");
    let api_hash = env!("TG_HASH").to_string();
    let path = "/Users/Alex.Plate/.tg_backup/tg_backup.session";
    let client = Client::connect(Config {
        session: Session::load_file(path).unwrap(),
        api_id,
        api_hash: api_hash.clone(),
        params: Default::default(),
    })
        .await
        .unwrap();

    fs::remove_dir_all("backup").unwrap();

    let mut dialog_iter = client.iter_dialogs();
    while let Some(dialog) = dialog_iter.next().await.unwrap() {
        if dialog.chat().id() == 1720199897 {
            let mut messages_iter = client.iter_messages(dialog.chat());
            let mut message_ids = vec![];
            while let Some(message) = messages_iter.next().await.unwrap() {
                message_ids.push(message.id());
            }
            client.delete_messages(dialog.chat(), &message_ids).await.unwrap();
            client.send_message(dialog.chat(), InputMessage::text("Hello")).await.unwrap();
            break;
        }
    }

    thread::sleep(Duration::from_millis(1000));

    let opts = Opts::parse_from(&["tg_backup", "--included-chats", "1720199897", "--quiet"]);
    tg_backup::backup::start_backup(opts).await;

    let file = BufReader::new(File::open(&Path::new("backup/chats/1720199897.tg_backup_test_2.tg_backup_test_2_bot/messages/data-20211009-20211009.json")).unwrap());
    let existing_data: Value = serde_json::from_reader(file).unwrap();

    let last_data = &existing_data[1];
    assert_eq!(422281, last_data["sender_id"]);
    assert_eq!("Alex", last_data["sender_name"]);
    assert_eq!("Hello", last_data["text"]);
}
