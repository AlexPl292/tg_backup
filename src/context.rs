use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

use crate::attachment_type::AttachmentType;
use crate::types::{BackUpInfo, MessageInfo};

pub const MESSAGES: &'static str = "messages";
pub const PHOTO: &'static str = "photo";
pub const FILE: &'static str = "file";
pub const ROUND: &'static str = "round";
pub const VOICE: &'static str = "voice";

pub struct Context {
    pub(crate) types: HashMap<String, AttachmentType>,
    pub(crate) messages_accumulator: Vec<MessageInfo>,
    pub(crate) accumulator_counter: i32,
}

impl Context {
    pub fn init(path: &Path) -> Context {
        let mut types = Context::init_types();
        types.values_mut().for_each(|x| x.init_folder(path));
        Context {
            types,
            messages_accumulator: vec![],
            accumulator_counter: 0,
        }
    }

    fn init_types() -> HashMap<String, AttachmentType> {
        let mut map = HashMap::new();
        map.insert(
            MESSAGES.to_string(),
            AttachmentType::init("messages", MESSAGES, None),
        );
        map.insert(
            PHOTO.to_string(),
            AttachmentType::init("photos", PHOTO, Some(".jpg")),
        );
        map.insert(FILE.to_string(), AttachmentType::init("files", FILE, None));
        map.insert(
            ROUND.to_string(),
            AttachmentType::init("rounds", ROUND, Some(".mp4")),
        );
        map.insert(
            VOICE.to_string(),
            AttachmentType::init("voice_messages", VOICE, Some(".ogg")),
        );
        map
    }

    pub(crate) fn drop_messages(&mut self, backup_info: &BackUpInfo) -> bool {
        if self.messages_accumulator.len() < backup_info.batch_size as usize {
            return false;
        }
        self.force_drop_messages();
        return true;
    }

    pub(crate) fn force_drop_messages(&mut self) {
        if self.messages_accumulator.is_empty() {
            return;
        }

        let data_type = self.types.get(MESSAGES).unwrap();
        let messages_path = data_type.path();
        let first_msg = self.messages_accumulator.first().unwrap().date.format("%Y%m%d");
        let last_msg = self.messages_accumulator.last().unwrap().date.format("%Y%m%d");
        let mut file_path = messages_path.join(format!("data-{}-{}.json", first_msg, last_msg));

        let mut counter = 0;
        while file_path.exists() {
            file_path = messages_path.join(format!("data-{}-{}-{}.json", first_msg, last_msg, counter));
            counter+=1;
        }

        let file = File::create(file_path).unwrap();
        serde_json::to_writer_pretty(&file, &self.messages_accumulator).unwrap();

        self.messages_accumulator.clear();
        self.accumulator_counter += 1;
    }
}
