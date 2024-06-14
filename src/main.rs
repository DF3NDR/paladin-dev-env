mod rss_fetcher;
mod api_integrator;
mod web_scraper;
mod llm_analyzer;
mod api_server;
mod config;
mod cli;
mod error;

use structopt::StructOpt;
use actix_web::web;
use std::sync::Mutex;
use crate::api_server::AppState;
use crate::error::FetchError;

#[tokio::main]
async fn main() {
    let args = cli::Cli::from_args();
    let config = config::load_config(&args.config).unwrap();
    println!("{:?}", config);

    let data = web::Data::new(AppState {
        summaries: Mutex::new(Vec::new()),
    });

    // Example usage of modules
    for source in &config.sources {
        let result = match source.source_type.as_str() {
            "rss" => {
                match rss_fetcher::fetch_rss_feed(&source.url).await {
                    Ok(channel) => {
                        println!("Title: {}", channel.title());
                        llm_analyzer::analyze_data(&channel.title(), &source.prompt).await
                    }
                    Err(e) => Err(e.into()),
                }
            }
            "api" => {
                match api_integrator::fetch_api_data(&source.url).await {
                    Ok(data) => {
                        println!("Data: {:?}", data);
                        llm_analyzer::analyze_data(&data.to_string(), &source.prompt).await
                    }
                    Err(e) => Err(e.into()),
                }
            }
            "web" => {
                match web_scraper::scrape_web_page(&source.url).await {
                    Ok(titles) => {
                        println!("Titles: {:?}", titles);
                        llm_analyzer::analyze_data(&titles.join(", "), &source.prompt).await
                    }
                    Err(e) => Err(e.into()),
                }
            }
            _ => Err(FetchError::Custom("Unknown source type".into())),
        };

        match result {
            Ok(summary) => {
                println!("Summary: {:?}", summary);
                data.summaries.lock().unwrap().push(summary.to_string());
            }
            Err(e) => eprintln!("Error processing data: {}", e),
        }
    }

    // Start the API server
    api_server::run_server().await.unwrap();
}
