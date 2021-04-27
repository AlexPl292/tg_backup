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
use std::str::FromStr;

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

    /// Show no output
    #[clap(short, long)]
    pub quiet: bool,

    /// Amount of log files that would be kept in the log directory
    #[clap(long, default_value = "1000")]
    pub keep_last_n_logs: usize,

    /// Control how much instances of tg_backup are allowed
    ///
    /// possible values: multiple, per-output, single
    ///
    /// multiple: User can start multiple instances of this application.
    ///
    /// per-output: Only a single instance per output is allowed. Additional instances will be
    ///     immediately finished.
    ///
    /// single: Only a single instance is allowed. Additional instances will be immediately finished.
    ///
    #[clap(long, default_value = "per-output")]
    pub instances: SingleInstanceOption,

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

#[derive(Clap)]
pub enum SingleInstanceOption {
    /// User can start multiple instances of this application
    Multiple,

    /// Only a single instance per output is allowed
    ///
    /// Additional instances will be immediately finished
    PerOutput,

    /// Only a single instance is allowed
    ///
    /// Additional instances will be immediately finished
    Single,
}

impl FromStr for SingleInstanceOption {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "multiple" => Ok(SingleInstanceOption::Multiple),
            "per-output" => Ok(SingleInstanceOption::PerOutput),
            "single" => Ok(SingleInstanceOption::Single),
            _ => Err(String::from("Cannot parse")),
        }
    }
}
