use actix_web::{web, HttpResponse, Responder};
use crate::config::Settings;
use crate::fetcher::{api_fetcher::fetch_api_data, rss_fetcher::fetch_rss_feed, scraper::scrape_web_page};
use crate::analyzer::llm_analyzer::analyze_data;
use std::sync::Arc;
use crate::error::FetchError;
use log::info;

async fn fetch_and_analyze_data(
    config: web::Data<Arc<Settings>>, 
    source_type: String, 
    url: String, 
    prompt: String
) -> Result<impl Responder, FetchError> {
    let normalized_data = match source_type.as_str() {
        "rss" => fetch_rss_feed(&url).await?,
        "api" => fetch_api_data(&url).await?,
        "web" => scrape_web_page(&url).await?,
        _ => return Err(FetchError::Custom("Unknown source type".into())),
    };

    info!("Normalized data: {:?}", normalized_data);

    let mut summaries = Vec::new();
    for item in normalized_data {
        let summary = analyze_data(&item.title, &prompt, &config).await?;
        summaries.push(summary);
    }

    info!("Summaries: {:?}", summaries);

    // Serialize the summaries into JSON
    Ok(HttpResponse::Ok().json(summaries))
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/analyze")
            .route(web::get().to(fetch_and_analyze_data))
    );
}

#[cfg(test)]
mod tests {
    use env_logger::Env;
    use actix_web::{test, App, web};
    use std::sync::Arc;
    use crate::test_utils::common::load_test_config;

    #[actix_web::test]
    async fn test_fetch_and_analyze_data() {
        env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();
        let config = Arc::new(load_test_config());
        let data = web::Data::new(config.clone());

        let mut app = test::init_service(
            App::new()
                .app_data(data.clone())
                .configure(crate::delivery::router::configure)
        ).await;

        let req = test::TestRequest::get()
            .uri("/analyze?source_type=rss&prompt=summarize&url=https://caitlin-long.com/feed/")
            .to_request();
        let resp = test::call_service(&mut app, req).await;

        // assert!(resp.status().is_success());

        let summaries: Vec<String> = test::read_body_json(resp).await;
        assert!(!summaries.is_empty());
    }
}
