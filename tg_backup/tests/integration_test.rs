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

use tg_backup;
use tg_backup::{start_backup, Opts};
use tg_backup_connector::test::TestTg;
use tempdir::TempDir;
use std::fs;

#[tokio::test]
async fn test_loading() {
    let data = TestTg { dialogs: vec![] };
    let temp_dir = TempDir::new("backup").unwrap();
    let backup_path = temp_dir.path().display().to_string();

    start_backup::<TestTg>(
        Some(data),
        Opts {
            included_chats: vec![1707414104, 1720199897],
            excluded_chats: vec![],
            batch_size: 5,
            clean: true,
            session_file: None,
            auth: None,
            output: Some(backup_path),
        },
    )
    .await;

    let files: Vec<String> = fs::read_dir(temp_dir.path()).unwrap().map(|x| x.unwrap().file_name().to_str().unwrap().to_string()).collect();
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
