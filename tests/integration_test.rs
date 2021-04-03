/*
 * tg_backup - software to backup data from Telegram messenger
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
use std::io::Read;
use std::path::Path;
use tg_backup;
use tg_backup::{start_backup, Opts};

#[tokio::test]
async fn test_loading() {
    start_backup(Opts {
        included_chats: vec![1707414104, 1720199897],
        batch_size: 5,
        clean: true,
        auth: None,
    })
    .await;

    let backup_path = Path::new("backup");
    assert!(backup_path.exists());

    let paths = backup_path.read_dir().unwrap();
    let path_list: Vec<_> = paths
        .into_iter()
        .map(|x| x.unwrap().file_name().to_str().unwrap().to_string())
        .collect();
    assert_eq!(["backup.json", "chats"].to_vec(), path_list);

    let chats_path = backup_path.join("chats");
    let paths = chats_path.read_dir().unwrap();
    let path_list: Vec<_> = paths
        .into_iter()
        .map(|x| x.unwrap().file_name().to_str().unwrap().to_string())
        .collect();
    assert_eq!(["1707414104.tg_backup_test"].to_vec(), path_list);

    let chat_path = chats_path.join("1707414104.tg_backup_test");
    let paths = chat_path.read_dir().unwrap();
    let path_list: Vec<_> = paths
        .into_iter()
        .map(|x| x.unwrap().file_name().to_str().unwrap().to_string())
        .collect();
    assert_eq!(
        [
            "messages",
            "files",
            "rounds",
            "info.json",
            "photos",
            "voice_messages"
        ]
        .to_vec(),
        path_list
    );

    let mut res = String::new();
    File::open(chat_path.join("info.json"))
        .unwrap()
        .read_to_string(&mut res)
        .unwrap();
    assert_eq!(
        res,
        r#"{
  "name": "tg_backup_test",
  "id": 1707414104
}"#
    );

    let messages_path = chat_path.join("messages");
    let paths = messages_path.read_dir().unwrap();
    let path_list: Vec<_> = paths
        .into_iter()
        .map(|x| x.unwrap().file_name().to_str().unwrap().to_string())
        .collect();
    assert_eq!(
        ["data-2.json", "data-0.json", "data-1.json"].to_vec(),
        path_list
    );

    let mut res = String::new();
    File::open(messages_path.join("data-0.json"))
        .unwrap()
        .read_to_string(&mut res)
        .unwrap();
    assert_eq!(
        res,
        r#"[
  {
    "text": "Test msg 9",
    "id": 214752,
    "date": "2021-03-23T11:35:41Z",
    "attachment": null
  },
  {
    "text": "Test msg 8",
    "id": 214751,
    "date": "2021-03-23T11:35:41Z",
    "attachment": null
  },
  {
    "text": "Test msg 7",
    "id": 214750,
    "date": "2021-03-23T11:35:41Z",
    "attachment": null
  },
  {
    "text": "Test msg 6",
    "id": 214749,
    "date": "2021-03-23T11:35:41Z",
    "attachment": null
  },
  {
    "text": "Test msg 5",
    "id": 214748,
    "date": "2021-03-23T11:35:41Z",
    "attachment": null
  }
]"#
    );

    let mut res = String::new();
    File::open(messages_path.join("data-1.json"))
        .unwrap()
        .read_to_string(&mut res)
        .unwrap();
    assert_eq!(
        res,
        r#"[
  {
    "text": "Test msg 4",
    "id": 214747,
    "date": "2021-03-23T11:35:40Z",
    "attachment": null
  },
  {
    "text": "Test msg 3",
    "id": 214746,
    "date": "2021-03-23T11:35:40Z",
    "attachment": null
  },
  {
    "text": "Test msg 2",
    "id": 214745,
    "date": "2021-03-23T11:35:40Z",
    "attachment": null
  },
  {
    "text": "Test msg 1",
    "id": 214744,
    "date": "2021-03-23T11:35:40Z",
    "attachment": null
  },
  {
    "text": "Test msg 0",
    "id": 214743,
    "date": "2021-03-23T11:35:40Z",
    "attachment": null
  }
]"#
    );

    let mut res = String::new();
    File::open(messages_path.join("data-2.json"))
        .unwrap()
        .read_to_string(&mut res)
        .unwrap();
    assert_eq!(
        res,
        r#"[
  {
    "text": "/start",
    "id": 214742,
    "date": "2021-03-23T11:25:51Z",
    "attachment": null
  }
]"#
    );
}
