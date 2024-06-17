use crate::config::Settings;
use std::fs::File;
use std::io::{self, BufReader};
use serde_yaml::from_reader;

pub struct FileSourceRepository;

impl FileSourceRepository {
    pub fn load_sources(filename: &str) -> io::Result<Settings> {
        let file = File::open(filename)?;
        let reader = BufReader::new(file);
        let config: Settings = from_reader(reader)?;
        Ok(config)
    }
}
