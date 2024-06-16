// src/aggregator/summarizer.rs
pub fn summarize_content(content: &str, summary_format: &str) -> String {
    // Simple summarization logic
    match summary_format {
        "bullets" => format!("• {}", content.replace("\n", "\n• ")),
        _ => content.to_string(),
    }
}
