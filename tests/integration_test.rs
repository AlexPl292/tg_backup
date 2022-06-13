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

use clap::Parser;
use grammers_client::types::Dialog;
use grammers_client::{Client, Config, InputMessage};
use grammers_session::Session;
use serde_json::value::Value::Null;
use serde_json::Value;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::Duration;
use std::{fs, thread};
use tg_backup::opts::Opts;

#[tokio::test]
#[ignore]
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

    let _ = fs::remove_dir_all("backup");

    let main_dialog = get_dialog(&client, 1720199897).await.unwrap();
    let second_dialog = get_dialog(&client, 1707414104).await.unwrap();

    cleanup(&client, &main_dialog).await;

    // Tests
    send_hello(&client, &main_dialog).await;
    forward(&client, &main_dialog, second_dialog).await;
    send_dice(&client, &main_dialog).await;

    thread::sleep(Duration::from_millis(1000));

    let opts = Opts::parse_from(&[
        "tg_backup",
        "--included-chats",
        "1720199897",
        "--quiet",
        "--panic-to-stderr",
        "--test",
    ]);
    tg_backup::backup::start_backup(opts).await;

    let file = BufReader::new(
        File::open(&Path::new(
            "backup/chats/1720199897.tg_backup_test_2.tg_backup_test_2_bot/messages/data.json",
        ))
        .unwrap(),
    );
    let existing_data: Value = serde_json::from_reader(file).unwrap();

    // Checks
    let last_data = &existing_data[3];
    assert_eq!(422281, last_data["sender_id"]);
    assert_eq!("Alex", last_data["sender_name"]);
    assert_eq!("Hello", last_data["text"]);

    let last_data = &existing_data[2];
    assert_eq!(422281, last_data["sender_id"]);
    assert_eq!("Alex", last_data["sender_name"]);
    assert_eq!("Test msg 0", last_data["text"]);
    assert_eq!(422281, last_data["forwarded_from"]["from_id"]);
    assert_eq!(Null, last_data["forwarded_from"]["from_name"]); // IDK why

    let last_data = &existing_data[1];
    assert_eq!(422281, last_data["sender_id"]);
    assert_eq!("Alex", last_data["sender_name"]);
    assert_eq!("", last_data["text"]);
    assert!(last_data["attachment"]["Dice"]["value"].is_number());
    assert_eq!("🎲", last_data["attachment"]["Dice"]["emoticon"]);

    let last_data = &existing_data[0];
    assert_eq!(422281, last_data["sender_id"]);
    assert_eq!("Alex", last_data["sender_name"]);
    assert_eq!("", last_data["text"]);
    assert_eq!("HistoryClear", last_data["action"]);
}

async fn forward(client: &Client, main_dialog: &Dialog, second_dialog: Dialog) {
    client
        .forward_messages(main_dialog.chat(), &[214743], second_dialog.chat)
        .await
        .unwrap();
}

async fn cleanup(client: &Client, dialog: &Dialog) {
    let mut messages_iter = client.iter_messages(dialog.chat());
    let mut message_ids = vec![];
    while let Some(message) = messages_iter.next().await.unwrap() {
        message_ids.push(message.id());
    }
    client
        .delete_messages(dialog.chat(), &message_ids)
        .await
        .unwrap();
}

async fn send_hello(client: &Client, dialog: &Dialog) {
    client
        .send_message(dialog.chat(), InputMessage::text("Hello"))
        .await
        .unwrap();
}

async fn send_dice(client: &Client, dialog: &Dialog) {
    client
        .send_message(
            dialog.chat(),
            InputMessage::text("").dice(String::from("🎲")),
        )
        .await
        .unwrap();
}

async fn get_dialog(client: &Client, id: i32) -> Result<Dialog, ()> {
    let mut dialog_iter = client.iter_dialogs();
    while let Some(dialog) = dialog_iter.next().await.unwrap() {
        if dialog.chat().id() == id {
            return Ok(dialog);
        }
    }
    return Err(());
}
