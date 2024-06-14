use smartcontent_aggregator::test_utils::common::{setup_test_config, get_llm_config};
use smartcontent_aggregator::{rss_fetcher, llm_analyzer, error::FetchError};

#[tokio::test]
#[ignore] // Ignoring the test as it requires an external API that costs money
async fn test_fetch_and_analyze_rss() -> Result<(), FetchError> {
    let config = setup_test_config();
    let source = &config.sources[0];
    let prompt = &source.prompt;

    let channel = rss_fetcher::fetch_rss_feed(&source.url).await?;
    let llm_config = get_llm_config();
    let summary = llm_analyzer::analyze_data(&channel.title(), prompt, &llm_config).await?;

    assert!(summary.is_object()); // Assuming the LLM returns a JSON object.
    Ok(())
}
