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

use grammers_mtproto::mtp::RpcError;
use std::error::Error;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};

use grammers_mtsender::InvocationError;

#[derive(Debug)]
pub enum TgError {
    Rpc { name: String, value: Option<u32> },
    Other(InvocationError),
}

impl Display for TgError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            TgError::Rpc { name, value } => {
                write!(f, "Name{}, value: {:?})", name, value)
            }
            TgError::Other(inv) => fmt::Display::fmt(&inv, f),
        }
    }
}

impl Error for TgError {}

impl From<InvocationError> for TgError {
    fn from(err: InvocationError) -> Self {
        match err {
            InvocationError::Rpc(RpcError {
                code: _,
                name,
                value,
            }) => TgError::Rpc { name, value },
            _ => TgError::Other(err),
        }
    }
}
