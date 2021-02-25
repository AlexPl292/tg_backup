use std::fs;
use std::path::Path;

pub struct AttachmentType {
    pub folder: String,
    pub type_name: String,
    path: Option<Box<Path>>,
    pub extension: Option<String>,
}

impl AttachmentType {
    pub fn init(folder: &str, type_name: &str, extension: Option<&str>) -> AttachmentType {
        AttachmentType {
            folder: folder.to_string(),
            type_name: type_name.to_string(),
            path: None,
            extension: extension.map(|x| x.to_string()),
        }
    }

    pub fn init_folder(&mut self, path: &Path) {
        let photos_path = path.join(self.folder.as_str());
        let _ = fs::create_dir(&photos_path);
        self.path = Some(photos_path.into_boxed_path())
    }

    pub fn path(&self) -> &Path {
        self.path.as_ref().unwrap()
    }

    pub fn format(&self, name: String) -> String {
        format!(
            "{}{}",
            name,
            self.extension.as_ref().unwrap_or(&String::new())
        )
    }
}
