use std::sync::Arc;
use crate::adapters::primary::{
    http_content_fetcher::HttpContentFetcher,
    llm_content_analyzer::LlmContentAnalyzer,
    content_aggregator::ContentAggregator,
    http_content_deliverer::HttpContentDeliverer,
};
use crate::application::use_cases::{
    fetch_content::FetchContentUseCase,
    analyze_content::AnalyzeContentUseCase,
    aggregate_content::AggregateContentUseCase,
    deliver_content::DeliverContentUseCase,
};

#[derive(Clone)]
pub struct UseCases {
    pub fetch_content: Arc<FetchContentUseCase<HttpContentFetcher>>,
    pub analyze_content: Arc<AnalyzeContentUseCase<LlmContentAnalyzer>>,
    pub aggregate_content: Arc<AggregateContentUseCase<ContentAggregator>>,
    pub deliver_content: Arc<DeliverContentUseCase<HttpContentDeliverer>>,
}

pub fn initialize_use_cases() -> UseCases {
    let fetch_content = Arc::new(FetchContentUseCase::new(HttpContentFetcher {}));
    let analyze_content = Arc::new(AnalyzeContentUseCase::new(LlmContentAnalyzer {}));
    let aggregate_content = Arc::new(AggregateContentUseCase::new(ContentAggregator {}));
    let deliver_content = Arc::new(DeliverContentUseCase::new(HttpContentDeliverer {}));

    UseCases {
        fetch_content,
        analyze_content,
        aggregate_content,
        deliver_content,
    }
}
