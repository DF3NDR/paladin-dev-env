use crate::configuration::Settings;

pub fn load_test_config() -> Settings {
    Settings::load_from_file("config.yml").expect("Failed to load configuration for tests")
}
