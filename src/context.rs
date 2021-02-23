use std::collections::HashMap;
use crate::attachment_type::AttachmentType;
use crate::types::{MessageInfo, Error};
use std::path::Path;
use std::fs::File;

const MESSAGES: &'static str = "messages";
const PHOTO: &'static str = "photo";
const FILE: &'static str = "file";
const ROUND: &'static str = "round";
const VOICE: &'static str = "voice";

const ACCUMULATOR_SIZE: usize = 1_000;

pub struct Context {
    pub(crate) types: HashMap<String, AttachmentType>,
    pub(crate) messages_accumulator: Vec<MessageInfo>,
    accumulator_counter: i32,
    pub(crate) errors: Vec<Error>,
}

impl Context {
    pub fn init(path: &Path) -> Context {
        let mut types = Context::init_types();
        types.values_mut().for_each(|x| x.init_folder(path));
        Context {
            types,
            messages_accumulator: vec![],
            accumulator_counter: 0,
            errors: vec![],
        }
    }

    pub(crate) fn save_errors(&self, path: &Path, id: i32) {
        let errors_path = path.join(format!("errors-{}.json", id));
        let file = File::create(errors_path).unwrap();
        serde_json::to_writer_pretty(file, &self.errors).unwrap();
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

    pub(crate) fn drop_messages(&mut self) {
        if self.messages_accumulator.len() < ACCUMULATOR_SIZE {
            return;
        }
        self.force_drop_messages()
    }

    pub(crate) fn force_drop_messages(&mut self) {
        let data_type = self.types.get(MESSAGES).unwrap();
        let messages_path = data_type.path();
        let file_path = messages_path.join(format!("data-{}.json", self.accumulator_counter));
        let file = File::create(file_path).unwrap();
        serde_json::to_writer_pretty(&file, &self.messages_accumulator).unwrap();

        self.messages_accumulator.clear();
        self.accumulator_counter += 1;
    }
}
