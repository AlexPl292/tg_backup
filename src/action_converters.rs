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
use crate::types::{Action, Member, PhoneCallDiscardReason};
use grammers_tl_types as tl;

impl From<&tl::types::MessageActionPhoneCall> for Action {
    fn from(data: &tl::types::MessageActionPhoneCall) -> Self {
        let reason = data.reason.as_ref().map(|it| match it {
            tl::enums::PhoneCallDiscardReason::Missed => {
                PhoneCallDiscardReason::PhoneCallDiscardReasonMissed
            }
            tl::enums::PhoneCallDiscardReason::Disconnect => {
                PhoneCallDiscardReason::PhoneCallDiscardReasonDisconnect
            }
            tl::enums::PhoneCallDiscardReason::Hangup => {
                PhoneCallDiscardReason::PhoneCallDiscardReasonHangup
            }
            tl::enums::PhoneCallDiscardReason::Busy => {
                PhoneCallDiscardReason::PhoneCallDiscardReasonBusy
            }
        });
        Action::PhoneCall {
            is_video: data.video,
            call_id: data.call_id,
            reason,
            duration: data.duration.unwrap_or(-1),
        }
    }
}

impl From<&tl::types::MessageActionChatCreate> for Action {
    fn from(data: &tl::types::MessageActionChatCreate) -> Self {
        Action::ChatCreate {
            title: data.title.to_string(),
        }
    }
}

impl From<&tl::types::MessageActionChatEditTitle> for Action {
    fn from(data: &tl::types::MessageActionChatEditTitle) -> Self {
        Action::ChatEditTitle {
            new_title: data.title.to_string(),
        }
    }
}

impl From<&tl::types::MessageActionGroupCall> for Action {
    fn from(data: &tl::types::MessageActionGroupCall) -> Self {
        let tl::enums::InputGroupCall::Call(call_info) = &data.call;
        Action::GroupCall {
            duration: data.duration,
            id: call_info.id,
            access_hash: call_info.access_hash,
        }
    }
}

impl From<&tl::types::MessageActionInviteToGroupCall> for Action {
    fn from(data: &tl::types::MessageActionInviteToGroupCall) -> Self {
        let tl::enums::InputGroupCall::Call(group_call) = &data.call;
        let mut invites = vec![];
        for user in &data.users {
            // TODO convert to member
            invites.push(Member::IdOnly { id: *user })
        }
        Action::InviteToGroupCall {
            id: group_call.id,
            access_hash: group_call.access_hash,
            invites,
        }
    }
}
