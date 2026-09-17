//! Examples for `docs/src/user-guides/herald-output.md` (Phase 35, MB-24).
//!
//! Every `// ANCHOR:` region below is pulled into the Herald Output user guide via
//! mdBook `{{#include}}`, so a sample in the guide cannot drift from the landed API:
//! `cargo check -p paladin-doc-examples` compiles all of them.
#![allow(unused_variables, unused_imports, dead_code)]

// ANCHOR: custom_herald
use paladin_core::platform::container::herald::{
    BattalionResult, ExecutionMetadata, Herald, HeraldError, PaladinError, PaladinResult,
    StreamChunk,
};

/// A bespoke CSV formatter implementing the full seven-method `Herald` trait — the
/// same seven methods `output-formatting.md` documents, not the three-method stale
/// shape this page previously showed.
pub struct CsvHerald;

/// Escape a field for inclusion in a comma-separated row. This bespoke example
/// deliberately keeps escaping minimal (commas only, no quoting/newline handling) --
/// it is not RFC 4180-complete -- but applies it consistently across every method
/// below so no field can silently break row alignment.
fn csv_escape(field: &str) -> String {
    field.replace(',', ";")
}

impl Herald for CsvHerald {
    fn format_paladin_result(&self, result: &PaladinResult) -> Result<String, HeraldError> {
        Ok(format!(
            "{},{},{},{}\n",
            csv_escape(&result.output),
            result.usage.total_tokens,
            result.execution_time_ms,
            csv_escape(&format!("{:?}", result.stop_reason)),
        ))
    }

    fn format_battalion_result(&self, result: &BattalionResult) -> Result<String, HeraldError> {
        Ok(format!("{}\n", csv_escape(&result.final_output)))
    }

    fn format_stream_chunk(&self, chunk: &StreamChunk) -> Result<Option<String>, HeraldError> {
        Ok(Some(chunk.content.clone()))
    }

    fn finalize_stream(&self, metadata: &ExecutionMetadata) -> Result<String, HeraldError> {
        Ok(format!(
            "# total_tokens={},duration_ms={}\n",
            metadata.token_usage.total_tokens,
            csv_escape(&format!("{:?}", metadata.duration_ms)),
        ))
    }

    fn format_error(&self, error: &PaladinError) -> String {
        format!("error,{}\n", csv_escape(&error.to_string()))
    }

    fn name(&self) -> &str {
        "csv"
    }

    fn mime_type(&self) -> &str {
        "text/csv"
    }
}
// ANCHOR_END: custom_herald
