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

use clap::AppSettings;
use clap::Clap;
use clap::ValueHint;

//#[clap(after_help = "Beware `-d`, dragons be here")]
// We can put something at the end

#[derive(Clap)]
#[clap(author, about, version)]
#[clap(setting = AppSettings::ColorAuto)]
#[clap(setting = AppSettings::ColoredHelp)]
#[clap(setting = AppSettings::HelpRequired)]
// #[clap(setting = AppSettings::DisableVersionForSubcommands)]
pub struct Opts {
    /// Backup output directory
    #[clap(long, short, value_hint = ValueHint::DirPath)]
    pub output: Option<String>,

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

    /// Path to custom session file [default: ~/.tg_backup/tg_backup.session]
    #[clap(long, value_hint = ValueHint::FilePath)]
    pub session_file: Option<String>,

    #[clap(subcommand)]
    pub auth: Option<SubCommand>,
}

#[derive(Clap)]
pub enum SubCommand {
    /// Start authentication process
    Auth(Auth),
}

#[derive(Clap)]
pub struct Auth {
    /// Use this folder to create a session file [default: ~/.tg_backup]
    #[clap(long, value_hint = ValueHint::DirPath)]
    pub session_file_dir: Option<String>,

    /// Custom name for session file
    #[clap(long, default_value = "tg_backup.session")]
    pub session_file_name: String,
}
