pub mod service_runner;

use crate::config::application_settings::Settings;
use std::sync::Arc;

pub async fn setup_and_run(config: Settings) -> Result<(), Box<dyn std::error::Error>> {
    let config = Arc::new(config);

    service_runner::run_services(config.clone()).await?;
    Ok(())
}