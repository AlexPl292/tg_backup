use crate::context::{ChatContext, MainContext};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

const FILE_NAME: &'static str = "in_progress.txt";

#[derive(Serialize, Deserialize)]
pub struct InProgressInfo {
    pub extract_from: DateTime<Utc>,
    pub extract_until: Option<DateTime<Utc>>,
    pub accumulator_counter: i32,
    pub messages_counter: i32,
}

impl InProgressInfo {
    pub fn create(
        extract_from: DateTime<Utc>,
        extract_until: Option<DateTime<Utc>>,
        context: &ChatContext,
        main_context: &MainContext,
    ) -> InProgressInfo {
        InProgressInfo {
            extract_from,
            extract_until,
            accumulator_counter: context.accumulator_counter,
            messages_counter: context.accumulator_counter * main_context.batch_size
                + (context.messages_accumulator.len() as i32),
        }
    }
}

pub struct InProgress {
    path: PathBuf,
}

impl InProgress {
    pub fn create(path: &Path) -> InProgress {
        InProgress {
            path: path.join(FILE_NAME),
        }
    }

    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    pub fn read_data(&self) -> InProgressInfo {
        let file = BufReader::new(File::open(&self.path).unwrap());
        return serde_json::from_reader(file).unwrap();
    }

    pub fn write_data(&self, data: InProgressInfo) {
        let in_progress_file = File::create(&self.path).unwrap();
        serde_json::to_writer_pretty(&in_progress_file, &data).unwrap();
    }

    pub fn remove_file(&self) {
        fs::remove_file(&self.path).unwrap()
    }
}
