// src/delivery/router.rs
use actix_web::{web, HttpResponse, Responder};
use crate::config::Settings;
use crate::fetcher::fetch_data;
use crate::queue::enqueue_for_analysis;
use std::sync::Arc;
use crate::error::FetchError;

async fn fetch_and_analyze_data(
    config: web::Data<Arc<Settings>>, 
    source_type: String, 
    url: String, 
    prompt: String
) -> Result<impl Responder, FetchError> {
    let normalized_data = fetch_data(&source_type, &url).await?;
    for item in normalized_data {
        enqueue_for_analysis(item);
    }

    Ok(HttpResponse::Ok().json("Fetch task triggered"))
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/analyze")
            .route(web::get().to(fetch_and_analyze_data))
    );
}

#[cfg(test)]
mod tests {
    use actix_web::{test, App, web};
    use std::sync::Arc;
    use crate::test_utils::common::load_test_config;

    #[actix_web::test]
    async fn test_fetch_and_analyze_data() {
        let config = Arc::new(load_test_config());
        let data = web::Data::new(config);

        let mut app = test::init_service(
            App::new()
                .app_data(data.clone())
                .configure(super::configure)
        ).await;

        let req = test::TestRequest::with_uri("/analyze?source_type=rss&url=https://caitlin-long.com/feed/&prompt=Summarize").to_request();
        let resp = test::call_service(&mut app, req).await;

        assert!(resp.status().is_success());
    }
}
