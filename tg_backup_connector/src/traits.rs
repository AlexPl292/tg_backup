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

use std::io;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use grammers_client::client::auth::{AuthorizationError, InvocationError};
use grammers_client::client::messages::MessageIter;
use grammers_client::types::{Chat, Photo};

use tg_backup_types::Member;

use crate::test::TestTg;
use crate::TgError;

pub trait DChat: Send {
    fn id(&self) -> i32;
    fn name(&self) -> String;
    fn chat(&self) -> Chat;
    fn user(&self) -> Option<Member>;
}

pub trait DDialog: Send {
    fn chat(&mut self) -> Box<dyn DChat>;
}

#[async_trait]
pub trait DIter {
    async fn total(&mut self) -> Result<usize, InvocationError>;
    async fn next(&mut self) -> Result<Option<Box<dyn DDialog>>, InvocationError>;
}

#[async_trait]
pub trait DMsgIter: Send {
    async fn total(&mut self) -> Result<usize, InvocationError>;

    fn iter(self: Box<Self>) -> MessageIter;

    async fn next(&mut self) -> Result<Option<Box<dyn DMessage>>, TgError>;
}

pub trait DMessage: Send {
    fn date(&self) -> DateTime<Utc>;
    fn id(&self) -> i32;
    fn text(&self) -> String;
    fn photo(&self) -> Option<Box<dyn DPhoto>>;
    fn document(&self) -> Option<Box<dyn DDocument>>;
    fn edit_date(&self) -> Option<DateTime<Utc>>;
    fn mentioned(&self) -> bool;
    fn outgoing(&self) -> bool;
    fn pinned(&self) -> bool;
    fn sender_id(&self) -> Option<i32>;
    fn sender_name(&self) -> Option<String>;
}

#[async_trait]
pub trait DPhoto: Send {
    fn id(&self) -> Option<i64>;
    fn photo(self: Box<Self>) -> Photo;
    async fn load_largest(&self, path: &PathBuf) -> Result<(), io::Error>;
}

#[async_trait]
pub trait DDocument: Send {
    fn id(&self) -> i64;
    fn name(&self) -> String;
    fn is_round_message(&self) -> bool;
    fn is_voice_message(&self) -> bool;
    async fn download(&mut self, path: &Path) -> Result<(), io::Error>;
}

#[async_trait]
pub trait Tg: Clone + Send {
    async fn create_connection(
        test_data: Option<TestTg>,
        session_file: &Option<String>,
    ) -> Result<Self, AuthorizationError>
    where
        Self: Sized;

    async fn auth(session_file_path: Option<String>, session_file_name: String);

    async fn get_me(&mut self) -> Result<Member, InvocationError>;

    async fn dialogs(&mut self) -> Box<dyn DIter>;

    fn messages(
        &mut self,
        chat: &Box<dyn DChat>,
        offset_date: i32,
        offset_id: Option<i32>,
    ) -> Box<dyn DMsgIter>;
}
