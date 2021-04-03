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

use std::fs;
use std::path::Path;

pub struct AttachmentType {
    pub folder: String,
    pub type_name: String,
    path: Option<Box<Path>>,
    pub extension: Option<String>,
}

impl AttachmentType {
    pub fn init(folder: &str, type_name: &str, extension: Option<&str>) -> AttachmentType {
        AttachmentType {
            folder: folder.to_string(),
            type_name: type_name.to_string(),
            path: None,
            extension: extension.map(|x| x.to_string()),
        }
    }

    pub fn init_folder(&mut self, path: &Path) {
        let photos_path = path.join(self.folder.as_str());
        let _ = fs::create_dir_all(&photos_path);
        self.path = Some(photos_path.into_boxed_path())
    }

    pub fn path(&self) -> &Path {
        self.path.as_ref().unwrap()
    }

    pub fn format(&self, name: String) -> String {
        if let Some(ext) = self.extension.as_ref() {
            if !name.ends_with(ext) {
                return format!("{}{}", name, ext);
            }
        }
        return name;
    }
}
