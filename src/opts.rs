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

use clap::Clap;

#[derive(Clap)]
#[clap(author = "Alex Plate <AlexPl292@gmail.com>", version = "0.1.0")]
pub struct Opts {
    /// List of chats that are going to be saved. All chats are saved by default.
    ///
    /// If both included-chats and excluded_chats have the same value, the chat will be excluded.
    #[clap(short, long)]
    pub included_chats: Vec<i32>,

    /// List of chats that are going to be excluded from saving.
    ///
    /// If both included-chats and excluded_chats have the same value, the chat will be excluded.
    #[clap(short, long)]
    pub excluded_chats: Vec<i32>,

    /// Size of batches with messages.
    #[clap(long, default_value = "1000")]
    pub batch_size: i32,

    /// If presented, the previous existing backup will be removed
    #[clap(short, long)]
    pub clean: bool,

    #[clap(long)]
    pub session_file: Option<String>,

    #[clap(subcommand)]
    pub auth: Option<SubCommand>,
}

#[derive(Clap)]
pub enum SubCommand {
    /// Start authentication process
    #[clap(version = "0.1.0", author = "Alex Plate <AlexPl292@gmail.com>")]
    Auth(Auth),
}

#[derive(Clap)]
pub struct Auth {
    #[clap(long)]
    pub session_file_path: Option<String>,

    #[clap(long)]
    pub session_file_name: Option<String>,
}
