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

use clap::Parser;
use clap::ValueHint;

//#[clap(after_help = "Beware `-d`, dragons be here")]
// We can put something at the end

#[derive(Parser, Debug, Clone)]
#[clap(author, about, version)]
pub struct Opts {
    /// Backup output directory
    #[clap(value_parser, long, short, value_hint = ValueHint::DirPath)]
    pub output: Option<String>,

    /// List of chats that are going to be saved. All chats are saved by default.
    ///
    /// If both included-chats and excluded_chats have the same value, the chat will be excluded.
    #[clap(value_parser, short, long)]
    pub included_chats: Vec<i64>,

    /// List of chats that are going to be excluded from saving.
    ///
    /// If both included-chats and excluded_chats have the same value, the chat will be excluded.
    #[clap(value_parser, short, long)]
    pub excluded_chats: Vec<i64>,

    /// Size of batches with messages.
    #[clap(value_parser, long, default_value = "1000")]
    pub batch_size: i32,

    /// If presented, the previous existing backup will be removed
    #[clap(value_parser, short, long)]
    pub clean: bool,

    /// Path to custom session file [default: ~/.tg_backup/tg_backup.session]
    #[clap(value_parser,long, value_hint = ValueHint::FilePath)]
    pub session_file: Option<String>,

    /// Show no output
    #[clap(value_parser, short, long)]
    pub quiet: bool,

    /// Amount of log files that would be kept in the log directory
    #[clap(value_parser, long, default_value = "1000")]
    pub keep_last_n_logs: usize,

    /// By default, panics are saved to log file. Use this option to show panics in stderr.
    ///
    /// If enabled, panics will be printed in stderr and not in logs.
    #[clap(value_parser, long)]
    pub panic_to_stderr: bool,

    /// Maximum size of the attachment in MB.
    #[clap(value_parser, long)]
    pub file_limit: Option<i32>,

    /// Skip saving chat if there is more participants. -1 for no limits
    #[clap(value_parser, long, default_value = "20")]
    pub max_participants: i32,

    #[clap(subcommand)]
    pub auth: Option<SubCommand>,

    /// Run test mode
    #[clap(value_parser, long, hide = true)]
    pub test: bool,
}

#[derive(Parser, Debug, Clone)]
pub enum SubCommand {
    /// Start authentication process
    Auth(Auth),
}

#[derive(Parser, Debug, Clone)]
pub struct Auth {
    /// Use this folder to create a session file [default: ~/.tg_backup]
    #[clap(value_parser,long, value_hint = ValueHint::DirPath)]
    pub session_file_dir: Option<String>,

    /// Custom name for session file
    #[clap(value_parser, long, default_value = "tg_backup.session")]
    pub session_file_name: String,
}
