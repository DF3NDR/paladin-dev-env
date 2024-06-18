pub mod http_server;
pub mod service_runner;
pub mod use_case_initializer;

use crate::config::Settings;
use std::sync::Arc;

pub async fn setup_and_run(config: Settings) -> std::io::Result<()> {
    let config = Arc::new(config);
    let use_cases = use_case_initializer::initialize_use_cases();

    service_runner::run_services(config.clone(), use_cases.clone()).await;
    http_server::run_http_server(config, use_cases).await
}
