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
use std::fs::DirEntry;
use std::path::PathBuf;

pub(crate) fn init_logs(output_dir: &PathBuf, max_size: usize) {
    let log_dir = format!("{}/logs", output_dir.as_path().display().to_string());
    let _ = fs::create_dir(log_dir.as_str());

    let existing_log_files = fs::read_dir(log_dir.as_str());
    if let Ok(res) = existing_log_files {
        let mut files: Vec<DirEntry> = res.map(|x| x.unwrap()).collect();
        if files.len() > max_size - 1 {
            files.sort_by(|x, y| {
                x.metadata()
                    .unwrap()
                    .created()
                    .unwrap()
                    .cmp(&y.metadata().unwrap().created().unwrap())
            });
            files[..(files.len() - (max_size - 1))]
                .iter()
                .for_each(|x| fs::remove_file(x.path()).unwrap());
        }
    }

    let now = chrono::offset::Utc::now();
    let now_formatted = now.format("%Y%m%d-%H%M%S");
    let log_path = format!("{}/tg_backup-{}.log", log_dir, now_formatted);
    simple_logging::log_to_file(log_path, log::LevelFilter::Info).unwrap();
}
