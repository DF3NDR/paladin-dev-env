use std::sync::{Arc, Mutex};
use actix_web::{web, App, HttpServer};
use log::info;
use crate::config::Settings;
use crate::use_case_initializer::UseCases;

#[derive(Clone)]
pub struct AppState {
    pub summaries: Arc<Mutex<Vec<String>>>,
    pub use_cases: UseCases,
}

pub async fn run_http_server(config: Arc<Settings>, use_cases: UseCases) -> std::io::Result<()> {
    let app_state = web::Data::new(AppState {
        summaries: Arc::new(Mutex::new(Vec::new())),
        use_cases,
    });

    let server_config = config.server.clone();

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .app_data(web::Data::new(config.clone()))
            .configure(crate::adapters::primary::http_content_deliverer::configure)
    })
    .bind(format!("{}:{}", server_config.host, server_config.port))?
    .run()
    .await
}
