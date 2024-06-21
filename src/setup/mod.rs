pub mod service_runner;

use crate::config::Settings;
use std::sync::Arc;

pub async fn setup_and_run(config: Settings) -> std::io::Result<()> {
    let config = Arc::new(config);

    service_runner::run_services(config.clone()).await;
    Ok(())
}
