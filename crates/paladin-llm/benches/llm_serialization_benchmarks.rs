use chrono::Utc;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use paladin_core::platform::container::prompt::{PromptItem, PromptType, UserPrompt};
use paladin_ports::output::llm_port::{FinishReason, LlmRequest, LlmResponse, TokenUsage};
use std::collections::HashMap;
use uuid::Uuid;

fn sample_request() -> LlmRequest {
    let prompt = PromptItem::new(PromptType::User(UserPrompt {
        query: "Summarize benchmark migration status".to_string(),
        context: Some("Benchmark suite modernization".to_string()),
    }))
    .expect("prompt item");

    let mut metadata = HashMap::new();
    metadata.insert("temperature".to_string(), "0.2".to_string());
    metadata.insert("max_tokens".to_string(), "1024".to_string());

    LlmRequest::new("mock-llm-model", prompt).with_metadata(metadata)
}

fn sample_response(request_id: Uuid) -> LlmResponse {
    LlmResponse {
        id: Uuid::new_v4(),
        request_id,
        model: "mock-llm-model".to_string(),
        content: "All benchmark migration checks completed successfully.".to_string(),
        finish_reason: FinishReason::Stop,
        usage: TokenUsage {
            prompt_tokens: 58,
            completion_tokens: 24,
            total_tokens: 82,
        },
        created_at: Utc::now(),
        metadata: HashMap::new(),
        function_call: None,
    }
}

fn benchmark_request_serialization(c: &mut Criterion) {
    let request = sample_request();

    c.bench_function("llm/serialize_request", |b| {
        b.iter(|| {
            let _ = serde_json::to_string(black_box(&request)).expect("serialize request");
        });
    });
}

fn benchmark_response_deserialization(c: &mut Criterion) {
    let request = sample_request();
    let response = sample_response(request.id);
    let json = serde_json::to_string(&response).expect("serialize seed response");

    c.bench_function("llm/deserialize_response", |b| {
        b.iter(|| {
            let _: LlmResponse = serde_json::from_str(black_box(&json)).expect("deserialize");
        });
    });
}

fn benchmark_response_roundtrip(c: &mut Criterion) {
    let request = sample_request();
    let response = sample_response(request.id);

    c.bench_function("llm/response_roundtrip", |b| {
        b.iter(|| {
            let json = serde_json::to_string(black_box(&response)).expect("serialize response");
            let _: LlmResponse = serde_json::from_str(black_box(&json)).expect("deserialize");
        });
    });
}

criterion_group!(
    llm_serialization_benches,
    benchmark_request_serialization,
    benchmark_response_deserialization,
    benchmark_response_roundtrip
);
criterion_main!(llm_serialization_benches);
