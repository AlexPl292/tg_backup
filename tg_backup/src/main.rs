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

use crate::backup::start_backup;
use crate::opts::Opts;
use clap::Clap;
use tg_backup_connector::production::ProductionTg;

mod attachment_type;
mod backup;
mod connector;
mod context;
mod in_progress;
mod opts;
mod types;

#[tokio::main]
async fn main() {
    let opts: Opts = Opts::parse();

    start_backup::<ProductionTg>(opts).await;
}
