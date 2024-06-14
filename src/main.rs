mod rss_fetcher;
mod api_integrator;
mod web_scraper;
mod llm_analyzer;
mod api_server;
mod config;
mod cli;
mod error;

use structopt::StructOpt;
use crate::error::FetchError;

#[tokio::main]
async fn main() {
    let args = cli::Cli::from_args();
    let config = config::load_config(&args.config).unwrap();
    println!("{:?}", config);

    // Example usage of modules
    if let Some(first_source) = config.sources.get(0) {
        let result = match first_source.source_type.as_str() {
            "rss" => {
                match rss_fetcher::fetch_rss_feed(&first_source.url).await {
                    Ok(channel) => {
                        println!("Title: {}", channel.title());
                        llm_analyzer::analyze_data(&channel.title(), &first_source.prompt).await
                    }
                    Err(e) => Err(e.into()),
                }
            }
            "api" => {
                match api_integrator::fetch_api_data(&first_source.url).await {
                    Ok(data) => {
                        println!("Data: {:?}", data);
                        llm_analyzer::analyze_data(&data.to_string(), &first_source.prompt).await
                    }
                    Err(e) => Err(e.into()),
                }
            }
            "web" => {
                match web_scraper::scrape_web_page(&first_source.url).await {
                    Ok(titles) => {
                        println!("Titles: {:?}", titles);
                        llm_analyzer::analyze_data(&titles.join(", "), &first_source.prompt).await
                    }
                    Err(e) => Err(e.into()),
                }
            }
            _ => Err(FetchError::Custom("Unknown source type".into())),
        };

        match result {
            Ok(summary) => println!("Summary: {:?}", summary),
            Err(e) => eprintln!("Error processing data: {}", e),
        }
    }

    // Start the API server
    api_server::run_server().await.unwrap();
}
