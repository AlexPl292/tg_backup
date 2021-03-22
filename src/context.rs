use crate::attachment_type::AttachmentType;
use crate::types::{Error, MessageInfo};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::path::Path;

pub const MESSAGES: &'static str = "messages";
pub const PHOTO: &'static str = "photo";
pub const FILE: &'static str = "file";
pub const ROUND: &'static str = "round";
pub const VOICE: &'static str = "voice";

pub(crate) const ACCUMULATOR_SIZE: usize = 1_000;

pub struct Context {
    pub(crate) types: HashMap<String, AttachmentType>,
    pub(crate) messages_accumulator: Vec<MessageInfo>,
    pub(crate) accumulator_counter: i32,
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

    pub(crate) fn save_errors(&self, path: &str, id: i32) {
        let errors_path_string = format!("{}/errors", path);
        let error_path = Path::new(errors_path_string.as_str());
        let _ = fs::create_dir(error_path);

        let errors_path = error_path.join(format!("errors-{}.json", id));
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

    pub(crate) fn drop_messages(&mut self) -> bool {
        if self.messages_accumulator.len() < ACCUMULATOR_SIZE {
            return false;
        }
        self.force_drop_messages();
        return true;
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
