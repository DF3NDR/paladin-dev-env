use actix_web::{test, App};
use smartcontent_aggregator::test_utils::common::load_test_config;
use smartcontent_aggregator::{api_server, error::FetchError, rss_fetcher, web_scraper, api_integrator, llm_analyzer};
use std::sync::Mutex;

#[actix_web::test]
#[ignore] // Ignoring the test as it requires an external API that costs money
async fn test_end_to_end() -> Result<(), FetchError> {
    let config = load_test_config();

    let data = actix_web::web::Data::new(api_server::AppState {
        summaries: Mutex::new(Vec::new()),
    });

    // Simulate fetching and processing data
    for source in &config.sources {
        let result = match source.source_type.as_str() {
            "rss" => {
                let items = rss_fetcher::fetch_rss_feed(&source.url).await?;
                let mut summaries = Vec::new();
                for item in items {
                    let summary = llm_analyzer::analyze_data(&item.title, &source.prompt, &config).await?;
                    summaries.push(summary);
                }
                Ok(summaries)
            }
            "api" => {
                let data = api_integrator::fetch_api_data(&source.url).await?;
                let titles: Vec<String> = data.iter().map(|item| item.title.clone()).collect();
                let summary = llm_analyzer::analyze_data(&titles.join(", "), &source.prompt, &config).await?;
                Ok(vec![summary])
            }
            "web" => {
                let items = web_scraper::scrape_web_page(&source.url).await?;
                let titles: Vec<String> = items.iter().map(|item| item.title.clone()).collect();
                let summary = llm_analyzer::analyze_data(&titles.join(", "), &source.prompt, &config).await?;
                Ok(vec![summary])
            }
            _ => Err(FetchError::Custom("Unknown source type".into())),
        };

        if let Ok(summaries) = result {
            for summary in summaries {
                data.summaries.lock().unwrap().push(summary.to_string());
            }
        }
    }

    // Set up Actix web server for testing
    let mut app = test::init_service(App::new().app_data(data.clone()).route("/content", actix_web::web::get().to(api_server::get_content))).await;
    let req = test::TestRequest::with_uri("/content").to_request();
    let resp: api_server::SummaryResponse = test::call_and_read_body_json(&mut app, req).await;

    assert!(!resp.summaries.is_empty());
    Ok(())
}
