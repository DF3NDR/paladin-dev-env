use actix_web::{web, HttpResponse, Responder};
use crate::configuration::Settings;
use crate::rss_fetcher::{fetch_rss_feed, normalize_rss_data};
use crate::llm_analyzer::analyze_data;
use std::sync::Arc;

async fn fetch_and_analyze_data(config: web::Data<Arc<Settings>>) -> impl Responder {
    let rss_url = "https://example.com/rss"; // Replace with actual URL
    match fetch_rss_feed(rss_url).await {
        Ok(channel) => {
            let normalized_data = normalize_rss_data(&channel);
            let data_str = serde_json::to_string(&normalized_data).unwrap();
            match analyze_data(&data_str, "Summarize the main points", &config).await {
                Ok(summary) => HttpResponse::Ok().json(summary),
                Err(e) => HttpResponse::InternalServerError().body(format!("Error analyzing data: {:?}", e)),
            }
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("Error fetching RSS feed: {:?}", e)),
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/analyze")
            .route(web::get().to(fetch_and_analyze_data))
    );
}

