// src/test_utils/common.rs
use crate::config::Settings;

pub fn load_test_config() -> Settings {
    Settings::load_from_file("config.test.yml").expect("Failed to load configuration for tests")
}