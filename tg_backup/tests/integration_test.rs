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

#[tokio::test]
async fn test_loading() {
    let data = TestTg { dialogs: vec![] };

    start_backup::<TestTg>(
        Some(data),
        Opts {
            included_chats: vec![1707414104, 1720199897],
            excluded_chats: vec![],
            batch_size: 5,
            clean: true,
            session_file: None,
            auth: None,
        },
    )
    .await
}
