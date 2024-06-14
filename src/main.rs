use smartcontent_aggregator::*;
use structopt::StructOpt;
use actix_web::web;
use std::sync::Mutex;

#[tokio::main]
async fn main() {
    let args = Cli::from_args();
    let config = load_config(&args.config).unwrap();
    println!("{:?}", config);

    let data = web::Data::new(AppState {
        summaries: Mutex::new(Vec::new()),
    });

    let llm_config = LlmConfig::new(
        std::env::var("LLM_TYPE").expect("LLM_TYPE must be set"),
        std::env::var("LLM_URL").expect("LLM_URL must be set"),
        std::env::var("LLM_API_KEY").expect("LLM_API_KEY must be set"),
    );

    // Example usage of modules
    for source in &config.sources {
        let result = match source.source_type.as_str() {
            "rss" => {
                match fetch_rss_feed(&source.url).await {
                    Ok(channel) => {
                        println!("Title: {}", channel.title());
                        analyze_data(&channel.title(), &source.prompt, &llm_config).await
                    }
                    Err(e) => Err(e.into()),
                }
            }
            "api" => {
                match fetch_api_data(&source.url).await {
                    Ok(data) => {
                        println!("Data: {:?}", data);
                        analyze_data(&data.to_string(), &source.prompt, &llm_config).await
                    }
                    Err(e) => Err(e.into()),
                }
            }
            "web" => {
                match scrape_web_page(&source.url).await {
                    Ok(titles) => {
                        println!("Titles: {:?}", titles);
                        analyze_data(&titles.join(", "), &source.prompt, &llm_config).await
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
    run_server().await.unwrap();
}
