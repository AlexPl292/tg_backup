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

use crate::context::ChatContext;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

const FILE_NAME: &'static str = "in_progress.json";

#[derive(Serialize, Deserialize)]
pub struct InProgressInfo {
    pub extract_from: DateTime<Utc>,
    pub extract_until: Option<DateTime<Utc>>,
    pub last_loaded_id: Option<i32>,
    pub accumulator_counter: i32,
    pub messages_counter: i32,

    pub file_issue: i64,
    pub file_issue_count: i32,
}

impl InProgressInfo {
    pub fn create(
        extract_from: DateTime<Utc>,
        extract_until: Option<DateTime<Utc>>,
        last_loaded_id: Option<i32>,
        messages_counter: i32,
        chat_ctx: &ChatContext,
    ) -> InProgressInfo {
        InProgressInfo {
            extract_from,
            extract_until,
            last_loaded_id,
            accumulator_counter: chat_ctx.accumulator_counter,
            messages_counter,

            file_issue: chat_ctx.file_issue,
            file_issue_count: chat_ctx.file_issue_count,
        }
    }
}

pub struct InProgress {
    path: PathBuf,
}

impl InProgress {
    pub fn create(path: &Path) -> InProgress {
        InProgress {
            path: path.join(FILE_NAME),
        }
    }

    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    pub fn read_data(&self) -> InProgressInfo {
        let file = BufReader::new(File::open(&self.path).unwrap());
        return serde_json::from_reader(file).unwrap();
    }

    pub fn write_data(&self, data: &InProgressInfo) {
        let in_progress_file = File::create(&self.path).unwrap();
        serde_json::to_writer_pretty(&in_progress_file, data).unwrap();
    }

    pub fn remove_file(&self) {
        fs::remove_file(&self.path).unwrap()
    }
}
