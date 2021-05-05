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

use crate::traits::{DChat, DDialog, DDocument, DIter, DMessage, DMsgIter, DPhoto, Tg};
use crate::TgError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use grammers_client::client::auth::{AuthorizationError, InvocationError};
use grammers_client::types::Chat;
use std::any::Any;
use tg_backup_types::{ForwardInfo, Member, ReplyInfo};

#[derive(Clone)]
pub struct TestTg {
    pub dialogs: Vec<TestDDialog>,
}

#[async_trait]
impl Tg for TestTg {
    async fn create_connection(
        test_data: Option<TestTg>,
        _session_file: &Option<String>,
    ) -> Result<Self, AuthorizationError>
    where
        Self: Sized,
    {
        Ok(test_data.unwrap())
    }

    async fn auth(_session_file_path: Option<String>, _session_file_name: String) {}

    fn need_auth(_session_file: &Option<String>) -> bool {
        false
    }

    async fn get_me(&mut self) -> Result<Member, InvocationError> {
        Ok(Member::User {
            id: 0,
            deleted: false,
            mutual_contact: false,
            contact: false,
            verified: false,
            last_name: Some(String::from("xx")),
            first_name: String::from("anem"),
            username: Some(String::from("Usernae")),
        })
    }

    async fn dialogs(&mut self) -> Box<dyn DIter> {
        Box::new(TestDIter {
            dialogs: self.dialogs.clone(),
            counter: 0,
        })
    }

    fn messages(
        &mut self,
        chat: &Box<dyn DChat>,
        _offset_date: i32,
        _offset_id: Option<i32>,
    ) -> Box<dyn DMsgIter> {
        let test_chat = chat.as_any().downcast_ref::<TestDChat>().unwrap();
        let msgs = test_chat.messages.clone();
        Box::new(TestDMsgIter {
            messages: msgs,
            counter: 0,
        })
    }
}

#[derive(Clone)]
pub struct TestDIter {
    pub dialogs: Vec<TestDDialog>,
    counter: usize,
}

#[async_trait]
impl DIter for TestDIter {
    async fn total(&mut self) -> Result<usize, InvocationError> {
        Ok(self.dialogs.len())
    }

    async fn next(&mut self) -> Result<Option<Box<dyn DDialog>>, InvocationError> {
        let dialog = Ok(self
            .dialogs
            .get(self.counter)
            .map(|x| Box::new(x.clone()) as Box<dyn DDialog>));
        self.counter += 1;
        dialog
    }
}

#[derive(Clone)]
pub struct TestDMsgIter {
    pub messages: Vec<TestDMessage>,
    counter: usize,
}

#[async_trait]
impl DMsgIter for TestDMsgIter {
    async fn total(&mut self) -> Result<usize, InvocationError> {
        Ok(self.messages.len())
    }

    async fn next(&mut self) -> Result<Option<Box<dyn DMessage>>, TgError> {
        let message = self
            .messages
            .get(self.counter)
            .map(|x| Box::new(x.clone()) as Box<dyn DMessage>);
        self.counter += 1;
        Ok(message)
    }
}

#[derive(Clone)]
pub struct TestDMessage {}

impl DMessage for TestDMessage {
    fn date(&self) -> DateTime<Utc> {
        todo!()
    }

    fn id(&self) -> i32 {
        todo!()
    }

    fn text(&self) -> String {
        todo!()
    }

    fn photo(&self) -> Option<Box<dyn DPhoto>> {
        todo!()
    }

    fn document(&self) -> Option<Box<dyn DDocument>> {
        todo!()
    }

    fn edit_date(&self) -> Option<DateTime<Utc>> {
        todo!()
    }

    fn mentioned(&self) -> bool {
        todo!()
    }

    fn outgoing(&self) -> bool {
        todo!()
    }

    fn pinned(&self) -> bool {
        todo!()
    }

    fn sender_id(&self) -> Option<i32> {
        todo!()
    }

    fn sender_name(&self) -> Option<String> {
        todo!()
    }

    fn fwd_from(&self) -> Option<ForwardInfo> {
        todo!()
    }

    fn reply_to(&self) -> Option<ReplyInfo> {
        todo!()
    }
}

#[derive(Clone)]
pub struct TestDDialog {
    pub messages: Vec<TestDMessage>,
}

impl DDialog for TestDDialog {
    fn chat(&mut self) -> Box<dyn DChat> {
        Box::new(TestDChat {
            messages: self.messages.clone(),
        })
    }
}

#[derive(Clone)]
pub struct TestDChat {
    messages: Vec<TestDMessage>,
}

#[async_trait]
impl DChat for TestDChat {
    fn id(&self) -> i32 {
        0
    }

    fn name(&self) -> String {
        String::from("my_chat")
    }

    fn chat(&self) -> Chat {
        todo!()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn members(&self) -> Vec<Member> {
        todo!()
    }

    fn visual_id(&self) -> String {
        String::from("test")
    }

    fn skip_backup(&self) -> bool {
        false
    }
}
