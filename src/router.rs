use actix_web::{web, HttpResponse, Responder};
use crate::configuration::Settings;
use std::sync::Arc;

async fn analyze_data_handler(config: web::Data<Arc<Settings>>) -> impl Responder {
    let llm_type = &config.llm_type;
    let llm_url = &config.llm_url;
    let llm_api_key = &config.llm_api_key;

    // Use the configuration values in your handler logic
    HttpResponse::Ok().body(format!(
        "Using LLM Type: {}, URL: {}, API Key: {}",
        llm_type, llm_url, llm_api_key
    ))
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/analyze")
            .route(web::get().to(analyze_data_handler))
    );
}
