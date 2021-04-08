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

use crate::traits::{DChat, DDialog, DIter, DMsgIter, Tg};
use async_trait::async_trait;
use grammers_client::client::auth::{AuthorizationError, InvocationError};
use tg_backup_types::Member;

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
        _chat: &Box<dyn DChat>,
        _offset_date: i32,
        _offset_id: Option<i32>,
    ) -> Box<dyn DMsgIter> {
        todo!()
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
pub struct TestDDialog {}

impl DDialog for TestDDialog {
    fn chat(&mut self) -> Box<dyn DChat> {
        todo!()
    }
}
