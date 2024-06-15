use actix_web::{web, HttpResponse, Responder};
use crate::configuration::Settings;
use crate::rss_fetcher::fetch_rss_feed;
use crate::api_integrator::fetch_api_data;
use crate::web_scraper::scrape_web_page;
use crate::llm_analyzer::analyze_data;
use std::sync::Arc;
use crate::error::FetchError;

async fn fetch_and_analyze_data(config: web::Data<Arc<Settings>>, source_type: String, url: String, prompt: String) -> Result<impl Responder, FetchError> {
    let normalized_data = match source_type.as_str() {
        "rss" => fetch_rss_feed(&url).await?,
        "api" => fetch_api_data(&url).await?,
        "web" => scrape_web_page(&url).await?,
        _ => return Err(FetchError::Custom("Unknown source type".into())),
    };

    let mut summaries = Vec::new();
    for item in normalized_data {
        let summary = analyze_data(&item.title, &prompt, &config).await?;
        summaries.push(summary);
    }

    Ok(HttpResponse::Ok().json(summaries))
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/analyze")
            .route(web::get().to(fetch_and_analyze_data))
    );
}
