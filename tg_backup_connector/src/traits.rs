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

use async_trait::async_trait;
use grammers_client::client::auth::{AuthorizationError, InvocationError};
use grammers_client::types::{Chat, Message};

use tg_backup_types::Member;

use crate::TgError;
use std::any::Any;

#[async_trait]
pub trait DChat: Send {
    fn id(&self) -> i32;
    fn name(&self) -> String;
    fn chat(&self) -> Chat;
    fn as_any(&self) -> &dyn Any;
    async fn members(&self) -> Vec<Member>;
    fn visual_id(&self) -> String;
    fn skip_backup(&self) -> bool;
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

    async fn next(&mut self) -> Result<Option<Message>, TgError>;
}

#[async_trait]
pub trait Tg: Clone + Send {
    async fn create_connection(session_file: &Option<String>) -> Result<Self, AuthorizationError>
    where
        Self: Sized;

    async fn auth(session_file_path: Option<String>, session_file_name: String);

    fn need_auth(session_file: &Option<String>) -> bool;

    async fn get_me(&mut self) -> Result<Member, InvocationError>;

    async fn dialogs(&mut self) -> Box<dyn DIter>;

    fn messages(
        &mut self,
        chat: &Box<dyn DChat>,
        offset_date: i32,
        offset_id: Option<i32>,
    ) -> Box<dyn DMsgIter>;
}
