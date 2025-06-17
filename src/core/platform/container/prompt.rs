/*
Prompt Item Domain Entity

PromptItem represents a piece of prompt in the system. It can be of various types such as text, video, audio, or image.
It contains metadata such as UUID, creation and modification timestamps, prompt type, URL, hash, source information, title, description, tags, author, and publication/modification dates.
It also provides methods to create a new prompt item, update its prompt, and generate a hash for the prompt.
PromptItem is a type of Node in the Hexagonal Architecture, meaning it is a core domain entity that should not have direct knowledge of the repository.

ToDo this is a placeholder for the actual implementation of the PromptItem domain entity. It is a copy of the ContentItem.

*/

use uuid::Uuid;
use url::Url;
use chrono::{DateTime, Utc};
use std::hash::{Hasher, Hash};
use fasthash::{murmur3::Hash128_x64, FastHash};

use thiserror::Error;
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PromptItem {
    pub uuid: Uuid,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub prompt: PromptType,
    pub url: Option<Url>,
    pub hash: Option<String>,
    pub source_id: Option<String>,
    pub source_url: Option<Url>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub source: Option<String>,
    pub author: Option<String>,
    pub pub_date: Option<DateTime<Utc>>,
    pub mod_date: Option<DateTime<Utc>>,
}

impl PromptItem {
    // Being a Domain Entity in Hexagonal Architecture the PromptItem should not have any knowledge of the repository
    pub fn new(
        prompt: PromptType, 
        // repository: &impl PromptRepository
    ) -> Result<Self, PromptItemError> {
        let mut prompt_item = PromptItem {
            uuid: Uuid::new_v4(),
            created: Utc::now(),
            modified: Utc::now(),
            prompt,
            url: None,
            hash: None,
            source_id: None,
            source_url: None,
            title: None,
            description: None,
            tags: None,
            source: None,
            author: None,
            pub_date: None,
            mod_date: None,
        };

        // prompt_item.update_hash(repository)?;

        Ok(prompt_item)
    }

    // Being a Domain Entity in Hexagonal Architecture the PromptItem should not have any knowledge of the repository
    pub fn update_prompt(
        &mut self, new_prompt: PromptType, 
        // repository: &impl PromptRepository
    ) -> Result<(), PromptItemError> {
        self.prompt = new_prompt;
        self.modified = Utc::now();
        // self.update_hash(repository)?;
        Ok(())
    }

    // Being a Domain Entity in Hexagonal Architecture the PromptItem should not have any knowledge of the repository
    fn update_hash(
        &mut self, 
        // repository: &impl PromptRepository
    ) -> Result<(), PromptItemError> {
        if let Some(path) = self.prompt.path() {
            let new_hash = generate_file_hash(&path)?;
            // if let Some(existing_prompt) = repository.get_by_hash(&new_hash)? {
            //     return Err(PromptItemError::HashAlreadyExists(existing_prompt.uuid));
            // }
            self.hash = Some(new_hash);
        } else {
            self.hash = None;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PromptType {
    Text(TextPrompt),
    Video(VideoPrompt),
    Audio(AudioPrompt),
    Image(ImagePrompt),
}

impl PromptType {
    pub fn path(&self) -> Option<&String> {
        match self {
            PromptType::Text(text) => text.path.as_ref(),
            PromptType::Video(video) => video.path.as_ref(),
            PromptType::Audio(audio) => audio.path.as_ref(),
            PromptType::Image(image) => image.path.as_ref(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextPrompt {
    pub path: Option<String>,
    pub prompt: Option<String>,
    pub filesize: u64,
}

impl TextPrompt {
    pub fn new(path: Option<String>, prompt: Option<String>) -> Result<Self, PromptItemError> {
        let filesize = if let Some(ref path) = path {
            let metadata = std::fs::metadata(path)?;
            check_filesize(metadata.len())?;
            metadata.len()
        } else {
            0
        };
        Ok(TextPrompt { path, prompt, filesize })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VideoPrompt {
    pub path: Option<String>,
    pub duration: u64,
    pub filesize: u64,
}

impl VideoPrompt {
    pub fn new(path: Option<String>, duration: u64) -> Result<Self, PromptItemError> {
        let filesize = if let Some(ref path) = path {
            let metadata = std::fs::metadata(path)?;
            check_filesize(metadata.len())?;
            metadata.len()
        } else {
            0
        };
        Ok(VideoPrompt { path, duration, filesize })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AudioPrompt {
    pub path: Option<String>,
    pub duration: u64,
    pub filesize: u64,
}

impl AudioPrompt {
    pub fn new(path: Option<String>, duration: u64) -> Result<Self, PromptItemError> {
        let filesize = if let Some(ref path) = path {
            let metadata = std::fs::metadata(path)?;
            check_filesize(metadata.len())?;
            metadata.len()
        } else {
            0
        };
        Ok(AudioPrompt { path, duration, filesize })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ImagePrompt {
    pub path: Option<String>,
    pub resolution: (u32, u32),
    pub filesize: u64,
}

impl ImagePrompt {
    pub fn new(path: Option<String>, resolution: (u32, u32)) -> Result<Self, PromptItemError> {
        let filesize = if let Some(ref path) = path {
            let metadata = std::fs::metadata(path)?;
            check_filesize(metadata.len())?;
            metadata.len()
        } else {
            0
        };
        Ok(ImagePrompt { path, resolution, filesize })
    }
}

#[derive(Debug, Clone, Error)]
pub enum PromptItemError {
    #[error("File not found")]
    FileNotFound,
    #[error("Failed to read file")]
    FileReadError,
    #[error("File size exceeds the maximum limit")]
    FileSizeLimitExceeded,
    #[error("Prompt hash already exists in the system with UUID: {0}")]
    HashAlreadyExists(Uuid),
    #[error("No prompt item exists for that hash")]
    NoPromptForHash,
}

fn generate_file_hash(path: &str) -> Result<String, PromptItemError> {
    let mut file = std::fs::File::open(path).map_err(|_| PromptItemError::FileNotFound)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).map_err(|_| PromptItemError::FileReadError)?;
    // not sure this will even work. Source for hash is supposed to be bytes not a Vec<u8>
    let hash = Hash128_x64::hash(buffer).to_string();
    Ok(format!("{:x}", hash))
}

fn check_filesize(filesize: u64) -> Result<(), PromptItemError> {
    let max_file_size = crate::config::application_settings::Settings::max_file_size();
    if filesize > max_file_size {
        Err(PromptItemError::FileSizeLimitExceeded)
    } else {
        Ok(())
    }
}
