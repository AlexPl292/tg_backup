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

use std::fs;

use tempdir::TempDir;

use tg_backup;
use tg_backup::{start_backup, Opts};
use tg_backup_connector::test::{TestDDialog, TestTg};

#[tokio::test]
async fn test_loading() {
    let data = TestTg { dialogs: vec![] };
    let temp_dir = TempDir::new("backup").unwrap();
    let backup_path = temp_dir.path().display().to_string();

    start_backup::<TestTg>(
        Some(data),
        Opts {
            included_chats: vec![],
            excluded_chats: vec![],
            batch_size: 5,
            clean: true,
            session_file: None,
            quiet: false,
            auth: None,
            output: Some(backup_path),
        },
    )
    .await;

    let mut files: Vec<String> = fs::read_dir(temp_dir.path())
        .unwrap()
        .map(|x| x.unwrap().file_name().to_str().unwrap().to_string())
        .collect();
    files.sort();
    assert_eq!(vec!["backup.json", "logs", "me.json"], files);

    let me_data = fs::read_to_string(temp_dir.path().join("me.json")).unwrap();
    let me_expected = r#"{
  "type": "User",
  "id": 0,
  "username": "Usernae",
  "first_name": "anem",
  "last_name": "xx",
  "verified": false,
  "contact": false,
  "mutual_contact": false,
  "deleted": false
}"#;
    assert_eq!(me_expected, me_data)
}

#[tokio::test]
async fn test_loading_with_dialogs() {
    let data = TestTg {
        dialogs: vec![TestDDialog { messages: vec![] }],
    };
    let temp_dir = TempDir::new("backup").unwrap();
    let backup_path = temp_dir.path().display().to_string();

    start_backup::<TestTg>(
        Some(data),
        Opts {
            included_chats: vec![],
            excluded_chats: vec![],
            batch_size: 5,
            clean: true,
            session_file: None,
            quiet: true,
            auth: None,
            output: Some(backup_path),
        },
    )
    .await;

    let mut files: Vec<String> = fs::read_dir(temp_dir.path())
        .unwrap()
        .map(|x| x.unwrap().file_name().to_str().unwrap().to_string())
        .collect();
    files.sort();
    assert_eq!(vec!["backup.json", "chats", "logs", "me.json"], files);

    let chats_dir = temp_dir.path().join("chats");

    let mut files: Vec<String> = fs::read_dir(chats_dir)
        .unwrap()
        .map(|x| x.unwrap().file_name().to_str().unwrap().to_string())
        .collect();
    files.sort();
    assert_eq!(vec!["0.my_chat.Username"], files);
}
