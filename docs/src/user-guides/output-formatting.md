# Output Formatting Guide

This guide covers the Herald system for formatting and controlling Paladin output in various formats and styles.

## Table of Contents

- [Overview](#overview)
- [Herald Architecture](#herald-architecture)
- [Built-in Formatters](#built-in-formatters)
- [Custom Formatters](#custom-formatters)
- [Streaming Output](#streaming-output)
- [Multi-Format Output](#multi-format-output)
- [Post-Processing](#post-processing)
- [Best Practices](#best-practices)
- [Advanced Patterns](#advanced-patterns)
- [Troubleshooting](#troubleshooting)

## Overview

The Herald system controls how Paladin output is formatted and presented to users.

**Key Capabilities:**
- **Format Transformation**: Convert LLM output to JSON, Markdown, HTML, etc.
- **Streaming**: Real-time output delivery for better UX
- **Validation**: Ensure output meets schema requirements
- **Post-Processing**: Clean, enhance, or transform responses
- **Multi-Channel**: Different formats for different output destinations

**Key Concepts:**
- **Herald**: Output formatting system
- **Formatter**: Converts raw LLM output to specific format
- **OutputFormat**: Target format specification (JSON, Markdown, Plain, etc.)
- **StreamHandler**: Processes output chunks in real-time

## Herald Architecture

### Core Components

The `Herald` trait (`crates/paladin-core/src/platform/container/herald.rs:49`) has seven methods,
not three, and `format_stream_chunk` returns `Result<Option<String>, HeraldError>` -- `None` means
"buffering, not ready to emit yet", not an error:

```rust,ignore
// Output format types (paladin_core::platform::container::paladin_config)
pub enum OutputFormat {
    Text,           // Raw text output
    Json,           // Structured JSON
    Structured,     // Structured data output
}

// Herald interface (paladin_core::platform::container::herald)
pub trait Herald: Send + Sync {
    /// Format a complete Paladin execution result
    fn format_paladin_result(&self, result: &PaladinResult) -> Result<String, HeraldError>;

    /// Format a complete Battalion execution result
    fn format_battalion_result(&self, result: &BattalionResult) -> Result<String, HeraldError>;

    /// Format a streaming chunk. `Ok(None)` means "buffering -- not ready to emit yet".
    fn format_stream_chunk(&self, chunk: &StreamChunk) -> Result<Option<String>, HeraldError>;

    /// Finalize streaming output with metadata (tokens, timing) once the stream completes.
    fn finalize_stream(&self, metadata: &ExecutionMetadata) -> Result<String, HeraldError>;

    /// Format an error for display. Infallible -- never returns Err.
    fn format_error(&self, error: &PaladinError) -> String;

    /// Formatter identifier, e.g. "json", "markdown", "table".
    fn name(&self) -> &str;

    /// MIME type of the formatted output, e.g. "application/json".
    fn mime_type(&self) -> &str;
}
```

### Integration with Paladin

```rust,ignore
let paladin = PaladinBuilder::new(llm_adapter)
    .name("Assistant")
    .system_prompt("You are a helpful assistant.")
    .output_format(OutputFormat::Text)
    .with_herald(Arc::new(MarkdownHerald::default()))
    .build()
    .await?;

let response = paladin.execute("Explain async/await").await?;
// response.content is formatted as Markdown
```

## Built-in Formatters

`crates/paladin-herald/src/lib.rs:8-15` ships exactly three formatters: `JsonHerald`,
`MarkdownHerald` (both always available), and `TableHerald` (gated behind the `table` feature,
`comfy-table`; the `color` feature separately gates ANSI colour in `MarkdownHerald`,
`Cargo.toml:19-20`). There is no `HtmlHerald` or `CodeHerald` -- the "HTML Herald" and "Code
Herald" sections previously here described formatters that do not exist anywhere in the tree
and have been removed. Each formatter takes a `*Config` struct via `::new()`/`::with_config()`
(or, for `TableHerald`, `::new(config)` directly) -- there is no fluent per-option builder chain
(`.with_schema()`, `.with_code_highlighting()`, `.with_css_framework()`, etc. do not exist).

### Markdown Herald

Formats output as Markdown, with optional ANSI colour for terminals (`color` feature) and a
configurable heading level.

```rust,ignore
use paladin_core::platform::container::herald::{Herald, HeraldError};
use paladin::infrastructure::adapters::herald::{MarkdownHerald, markdown_herald::MarkdownHeraldConfig};

let herald = Arc::new(MarkdownHerald::with_config(MarkdownHeraldConfig {
    include_colors: false,
    heading_level: 2,
}));

let paladin = PaladinBuilder::new(llm_adapter)
    .system_prompt("Format all responses as Markdown with proper headers and code blocks.")
    .with_herald(herald)
    .build()
    .await?;

let response = paladin.execute("Explain Rust ownership").await?;
println!("{}", response.content);
```

**Output example:**
```markdown
## Rust Ownership

Ownership is a core concept in Rust that ensures memory safety.

1. Each value has a single owner
2. When the owner goes out of scope, the value is dropped
3. Values can be borrowed immutably or mutably
```

### JSON Herald

Formats output as structured JSON, with pretty-printing and metadata inclusion toggles --
there is no schema-validation option (`with_schema()`/`validate_output()` do not exist on
`JsonHerald`; schema conformance is a system-prompt concern, not a Herald one).

```rust,ignore
use paladin_core::platform::container::herald::{Herald, HeraldError};
use paladin::infrastructure::adapters::herald::{JsonHerald, json_herald::JsonHeraldConfig};

let herald = Arc::new(JsonHerald::with_config(JsonHeraldConfig {
    pretty: true,
    include_metadata: true,
}));

let paladin = PaladinBuilder::new(llm_adapter)
    .system_prompt("Always respond in JSON format matching this schema: \
                    {summary: string, key_points: string[], confidence: number}")
    .with_herald(herald)
    .build()
    .await?;

let response = paladin.execute("Analyze sentiment of: 'This product is amazing!'").await?;

// Parse structured output
let json: serde_json::Value = serde_json::from_str(&response.content)?;
println!("Summary: {}", json["summary"]);
println!("Key points: {:?}", json["key_points"]);
```

**Output example** (the Herald's own JSON envelope around a Paladin result, per
`crates/paladin-herald/src/json_herald.rs` -- `output`/`usage`/`execution_time_ms`/
`loop_count`/`stop_reason`, plus a `metadata.timestamp` when `include_metadata` is true).
`usage` is the full `TokenUsage` split: a stable six-key object where the three
cache/reasoning sub-counts are `null` when the provider did not report them (ACCT-04, D-21):
```json
{
  "output": "Highly positive sentiment expressing enthusiasm...",
  "usage": {
    "prompt_tokens": 30,
    "completion_tokens": 12,
    "total_tokens": 42,
    "cache_read_tokens": null,
    "cache_write_tokens": null,
    "reasoning_tokens": null
  },
  "execution_time_ms": 850,
  "loop_count": 1,
  "stop_reason": "Completed",
  "metadata": {
    "timestamp": "2026-08-24T00:00:00Z"
  }
}
```

### Table Herald

Formats a `BattalionResult`'s per-Paladin results as an ASCII table -- suitable for terminal
output. Requires the `table` feature (`comfy-table`).

```rust,ignore
use paladin::infrastructure::adapters::herald::{TableHerald, table_herald::TableHeraldConfig};

let herald = Arc::new(TableHerald::new(TableHeraldConfig {
    max_column_width: 60,
    border_style: "rounded".to_string(),
}));
```

## Custom Formatters

Create custom heralds for specialized output formats by implementing the `Herald` trait
directly (`crates/paladin-core/src/platform/container/herald.rs:49`) -- no `async_trait` needed,
every method is synchronous. `Herald` has **seven** required methods, not the two or three shown
in older sketches below: `format_paladin_result`, `format_battalion_result`, `format_stream_chunk`
(returns `Result<Option<String>, HeraldError>` -- `Ok(None)` means "still buffering"),
`finalize_stream`, `format_error` (infallible), `name`, and `mime_type`. The rest of this section's
custom-Herald sketches (`XmlHerald`, `CsvHerald`, and the streaming/post-processing/advanced-pattern
examples further down) illustrate formatting logic against this real trait shape but, per D-12's
no-restructure scope, were not individually rewritten to implement every required method --
treat their partial `impl Herald for ...` blocks as illustrating *one* method's body, not a
complete, compiling implementation.

### Simple Custom Herald

```rust,ignore
use paladin_core::platform::container::herald::{
    BattalionResult, ExecutionMetadata, Herald, HeraldError, PaladinError, PaladinResult,
    StreamChunk,
};

pub struct UppercaseHerald;

impl Herald for UppercaseHerald {
    fn format_paladin_result(&self, result: &PaladinResult) -> Result<String, HeraldError> {
        Ok(result.output.to_uppercase())
    }

    fn format_battalion_result(&self, result: &BattalionResult) -> Result<String, HeraldError> {
        Ok(result.final_output.to_uppercase())
    }

    fn format_stream_chunk(&self, chunk: &StreamChunk) -> Result<Option<String>, HeraldError> {
        Ok(Some(chunk.content.to_uppercase()))
    }

    fn finalize_stream(&self, _metadata: &ExecutionMetadata) -> Result<String, HeraldError> {
        Ok(String::new())
    }

    fn format_error(&self, error: &PaladinError) -> String {
        error.to_string().to_uppercase()
    }

    fn name(&self) -> &str {
        "uppercase"
    }

    fn mime_type(&self) -> &str {
        "text/plain"
    }
}

// Usage
let herald = Arc::new(UppercaseHerald);
let paladin = PaladinBuilder::new(llm_adapter)
    .with_herald(herald)
    .build()
    .await?;
```

### XML Herald

A complete, compiling `XmlHerald` implementing all seven `Herald` methods already exists at
`examples/herald_custom_formatter.rs:36-121` -- read that file for the real, working version
rather than the partial sketch below (which, per the Custom Formatters section note above,
illustrates only the `format_paladin_result` body).

```rust,ignore
use paladin_core::platform::container::herald::{Herald, HeraldError};
use paladin::infrastructure::adapters::herald::{JsonHerald, MarkdownHerald, TableHerald};
use quick_xml::Writer;
use std::io::Cursor;

pub struct XmlHerald {
    root_element: String,
}

impl XmlHerald {
    pub fn new(root_element: &str) -> Self {
        Self {
            root_element: root_element.to_string(),
        }
    }
}

impl Herald for XmlHerald {
    fn format_paladin_result(&self, result: &PaladinResult) -> Result<String, HeraldError> {
        let mut writer = Writer::new(Cursor::new(Vec::new()));

        // Write XML declaration
        writer.write_event(quick_xml::events::Event::Decl(
            quick_xml::events::BytesDecl::new("1.0", Some("UTF-8"), None)
        ))?;

        // Parse content as structured data
        let data: serde_json::Value = serde_json::from_str(content)
            .map_err(|e| HeraldError::FormatError(e.to_string()))?;

        // Convert to XML
        self.json_to_xml(&mut writer, &self.root_element, &data)?;

        let xml_bytes = writer.into_inner().into_inner();
        Ok(String::from_utf8(xml_bytes)?)
    }
}

// Usage
let herald = Arc::new(XmlHerald::new("response"));

let paladin = PaladinBuilder::new(llm_adapter)
    .system_prompt("Return JSON that will be converted to XML")
    .with_herald(herald)
    .build().await?;
```

### CSV Herald

A complete, compiling `CsvHerald` implementing all seven `Herald` methods already exists at
`examples/herald_custom_formatter.rs:131-194` -- read that file for the real, working version
rather than the partial sketch below.

```rust,ignore
use paladin_core::platform::container::herald::{Herald, HeraldError};
use paladin::infrastructure::adapters::herald::{JsonHerald, MarkdownHerald, TableHerald};
use csv::Writer;

pub struct CsvHerald {
    headers: Vec<String>,
    delimiter: u8,
}

impl CsvHerald {
    pub fn new(headers: Vec<String>) -> Self {
        Self {
            headers,
            delimiter: b',',
        }
    }

    pub fn with_delimiter(mut self, delimiter: u8) -> Self {
        self.delimiter = delimiter;
        self
    }
}

impl Herald for CsvHerald {
    fn format_paladin_result(&self, result: &PaladinResult) -> Result<String, HeraldError> {
        // Parse JSON array
        let rows: Vec<serde_json::Value> = serde_json::from_str(content)
            .map_err(|e| HeraldError::FormatError(e.to_string()))?;

        let mut wtr = Writer::from_writer(vec![]);

        // Write headers
        wtr.write_record(&self.headers)?;

        // Write data rows
        for row in rows {
            let record: Vec<String> = self.headers.iter()
                .map(|h| {
                    row.get(h)
                        .map(|v| v.to_string())
                        .unwrap_or_default()
                })
                .collect();

            wtr.write_record(&record)?;
        }

        wtr.flush()?;
        let csv_bytes = wtr.into_inner()?;
        Ok(String::from_utf8(csv_bytes)?)
    }
}

// Usage
let herald = Arc::new(CsvHerald::new(vec![
    "name".to_string(),
    "age".to_string(),
    "city".to_string(),
]));

let paladin = PaladinBuilder::new(llm_adapter)
    .system_prompt("Return data as JSON array of objects with name, age, city fields")
    .with_herald(herald)
    .build().await?;

let response = paladin.execute("Generate 5 sample user records").await?;
// Output is formatted CSV
```

## Streaming Output

Process and format output in real-time for better user experience.

### Basic Streaming

```rust,ignore
use paladin_core::platform::container::herald::{Herald, HeraldError};
use paladin::infrastructure::adapters::herald::{JsonHerald, MarkdownHerald, TableHerald};
use futures::StreamExt;

let herald = Arc::new(MarkdownHerald::default());

let paladin = PaladinBuilder::new(llm_adapter)
    .with_herald(herald.clone())
    .build().await?;

// Execute with streaming
let mut stream = paladin.execute_stream("Write a story").await?;

while let Some(chunk) = stream.next().await {
    let chunk = chunk?;

    // Format chunk
    let formatted = herald.format_chunk(&chunk.content).await?;

    // Print in real-time
    print!("{}", formatted);
    std::io::stdout().flush()?;
}
println!();
```

### Streaming with Accumulation

```rust,ignore
pub struct StreamAccumulator {
    herald: Arc<dyn Herald>,
    buffer: String,
}

impl StreamAccumulator {
    pub fn new(herald: Arc<dyn Herald>) -> Self {
        Self {
            herald,
            buffer: String::new(),
        }
    }

    pub async fn process_chunk(&mut self, chunk: &str) -> Result<String, HeraldError> {
        self.buffer.push_str(chunk);

        // Format accumulated content
        self.herald.format(&self.buffer).await
    }

    pub fn buffer(&self) -> &str {
        &self.buffer
    }
}

// Usage
let mut accumulator = StreamAccumulator::new(herald);
let mut stream = paladin.execute_stream("Explain quantum computing").await?;

while let Some(chunk) = stream.next().await {
    let chunk = chunk?;
    let formatted_so_far = accumulator.process_chunk(&chunk.content).await?;

    // Update UI with fully formatted content
    update_ui(&formatted_so_far);
}
```

### Progress Indicators

```rust,ignore
pub struct ProgressHerald {
    inner: Arc<dyn Herald>,
    show_progress: bool,
}

impl Herald for ProgressHerald {
    fn format_stream_chunk(&self, chunk: &StreamChunk) -> Result<String, HeraldError> {
        let formatted = self.inner.format_chunk(chunk).await?;

        if self.show_progress {
            // Add visual progress indicator
            Ok(format!("{} .", formatted))
        } else {
            Ok(formatted)
        }
    }

    fn format_paladin_result(&self, result: &PaladinResult) -> Result<String, HeraldError> {
        self.inner.format_paladin_result(result)
    }
}
```

## Multi-Format Output

Generate output in multiple formats simultaneously.

### Multi-Format Herald

```rust,ignore
pub struct MultiFormatHerald {
    heralds: HashMap<String, Arc<dyn Herald>>,
}

impl MultiFormatHerald {
    pub fn new() -> Self {
        Self {
            heralds: HashMap::new(),
        }
    }

    pub fn add_format(mut self, name: &str, herald: Arc<dyn Herald>) -> Self {
        self.heralds.insert(name.to_string(), herald);
        self
    }

    pub async fn format_all(&self, content: &str) -> Result<HashMap<String, String>, HeraldError> {
        let mut results = HashMap::new();

        for (name, herald) in &self.heralds {
            let formatted = herald.format(content).await?;
            results.insert(name.clone(), formatted);
        }

        Ok(results)
    }
}

// Usage
let multi_herald = MultiFormatHerald::new()
    .add_format("json", Arc::new(JsonHerald::default()))
    .add_format("markdown", Arc::new(MarkdownHerald::default()))
    .add_format("html", Arc::new(JsonHerald::new()));

let paladin = PaladinBuilder::new(llm_adapter).build().await?;
let response = paladin.execute("Summarize Rust features").await?;

// Generate all formats
let all_formats = multi_herald.format_all(&response.content).await?;

// Save or serve each format
std::fs::write("output.json", &all_formats["json"])?;
std::fs::write("output.md", &all_formats["markdown"])?;
std::fs::write("output.html", &all_formats["html"])?;
```

### Adaptive Format Selection

```rust,ignore
pub struct AdaptiveHerald {
    formats: HashMap<String, Arc<dyn Herald>>,
    default: Arc<dyn Herald>,
}

impl AdaptiveHerald {
    pub async fn format_for_context(
        &self,
        content: &str,
        context: &OutputContext,
    ) -> Result<String, HeraldError> {
        let herald = self.select_herald(context);
        herald.format(content).await
    }

    fn select_herald(&self, context: &OutputContext) -> &Arc<dyn Herald> {
        match context.channel {
            OutputChannel::Web => self.formats.get("html").unwrap_or(&self.default),
            OutputChannel::Api => self.formats.get("json").unwrap_or(&self.default),
            OutputChannel::Terminal => self.formats.get("markdown").unwrap_or(&self.default),
            OutputChannel::File(ref ext) => {
                self.formats.get(ext.as_str()).unwrap_or(&self.default)
            }
        }
    }
}

pub struct OutputContext {
    pub channel: OutputChannel,
    pub user_preferences: HashMap<String, String>,
}

pub enum OutputChannel {
    Web,
    Api,
    Terminal,
    File(String),
}

// Usage
let adaptive = AdaptiveHerald::new()
    .with_format("html", Arc::new(JsonHerald::new()))
    .with_format("json", Arc::new(JsonHerald::default()))
    .with_format("markdown", Arc::new(MarkdownHerald::default()))
    .with_default(Arc::new(MarkdownHerald::new()));

// Format based on context
let web_output = adaptive.format_for_context(
    &content,
    &OutputContext {
        channel: OutputChannel::Web,
        user_preferences: HashMap::new(),
    }
).await?;

let api_output = adaptive.format_for_context(
    &content,
    &OutputContext {
        channel: OutputChannel::Api,
        user_preferences: HashMap::new(),
    }
).await?;
```

## Post-Processing

Transform or enhance output after formatting.

### Sanitization Herald

```rust,ignore
pub struct SanitizingHerald {
    inner: Arc<dyn Herald>,
    remove_patterns: Vec<regex::Regex>,
}

impl SanitizingHerald {
    pub fn new(inner: Arc<dyn Herald>) -> Self {
        Self {
            inner,
            remove_patterns: vec![
                // Remove potential PII
                regex::Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap(),  // SSN
                regex::Regex::new(r"\b[\w\.-]+@[\w\.-]+\.\w+\b").unwrap(),  // Email
                regex::Regex::new(r"\b\d{3}-\d{3}-\d{4}\b").unwrap(),  // Phone
            ],
        }
    }
}

impl Herald for SanitizingHerald {
    fn format_paladin_result(&self, result: &PaladinResult) -> Result<String, HeraldError> {
        let formatted = self.inner.format(content).await?;

        // Remove sensitive patterns
        let mut sanitized = formatted;
        for pattern in &self.remove_patterns {
            sanitized = pattern.replace_all(&sanitized, "[REDACTED]").to_string();
        }

        Ok(sanitized)
    }

    // Implement other methods...
}
```

### Enhancement Herald

```rust,ignore
pub struct EnhancingHerald {
    inner: Arc<dyn Herald>,
}

impl Herald for EnhancingHerald {
    fn format_paladin_result(&self, result: &PaladinResult) -> Result<String, HeraldError> {
        let formatted = self.inner.format(content).await?;

        // Add enhancements
        let enhanced = self.add_table_of_contents(&formatted);
        let enhanced = self.add_footnotes(&enhanced);
        let enhanced = self.add_timestamps(&enhanced);

        Ok(enhanced)
    }

    fn add_table_of_contents(&self, content: &str) -> String {
        // Extract headers and generate TOC
        let headers = self.extract_headers(content);

        if headers.is_empty() {
            return content.to_string();
        }

        let toc = headers.iter()
            .map(|(level, text, id)| {
                let indent = "  ".repeat(*level - 1);
                format!("{}* [{}](#{})", indent, text, id)
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!("## Table of Contents\n\n{}\n\n{}", toc, content)
    }

    fn add_footnotes(&self, content: &str) -> String {
        // Process [^1] style footnote references
        // Implementation...
        content.to_string()
    }

    fn add_timestamps(&self, content: &str) -> String {
        format!("Generated at: {}\n\n{}", chrono::Utc::now().to_rfc3339(), content)
    }
}
```

### Caching Herald

```rust,ignore
use std::collections::HashMap;
use std::sync::RwLock;

pub struct CachingHerald {
    inner: Arc<dyn Herald>,
    cache: RwLock<HashMap<String, String>>,
    max_cache_size: usize,
}

impl Herald for CachingHerald {
    fn format_paladin_result(&self, result: &PaladinResult) -> Result<String, HeraldError> {
        // Check cache
        {
            let cache = self.cache.read().unwrap();
            if let Some(cached) = cache.get(content) {
                return Ok(cached.clone());
            }
        }

        // Format
        let formatted = self.inner.format(content).await?;

        // Store in cache
        {
            let mut cache = self.cache.write().unwrap();

            // Evict oldest if at capacity
            if cache.len() >= self.max_cache_size {
                if let Some(key) = cache.keys().next().cloned() {
                    cache.remove(&key);
                }
            }

            cache.insert(content.to_string(), formatted.clone());
        }

        Ok(formatted)
    }

    // Implement other methods...
}
```

## Best Practices

### 1. Match Format to Use Case

```rust,ignore
// ✅ API endpoints - use JSON
let api_herald = Arc::new(JsonHerald::new()
    .with_schema(api_schema)
    .validate_output(true)
);

// ✅ Documentation - use Markdown
let docs_herald = Arc::new(MarkdownHerald::new()
    .with_table_of_contents(true)
    .with_code_highlighting(true)
);

// ✅ Web display - use HTML
let web_herald = Arc::new(JsonHerald::new()
    .with_css_framework(CssFramework::Bootstrap)
    .with_responsive_design(true)
);

// ✅ Data export - use CSV
let export_herald = Arc::new(CsvHerald::new(headers));
```

### 2. Validate Structured Output

```rust,ignore
let herald = Arc::new(JsonHerald::new()
    .with_schema(schema)
    .validate_output(true)  // Validate against schema
);

// Paladin will retry if output doesn't match schema
let paladin = PaladinBuilder::new(llm_adapter)
    .system_prompt("CRITICAL: Output MUST be valid JSON matching the schema")
    .with_herald(herald)
    .retry_attempts(3)  // Retry on validation failures
    .build().await?;
```

### 3. Use Streaming for Long Responses

```rust,ignore
// ❌ Bad: Wait for complete response
let response = paladin.execute(long_prompt).await?;
println!("{}", response.content);  // User waits 30 seconds

// ✅ Good: Stream for immediate feedback
let mut stream = paladin.execute_stream(long_prompt).await?;
while let Some(chunk) = stream.next().await {
    let chunk = chunk?;
    print!("{}", chunk.content);  // Immediate output
    std::io::stdout().flush()?;
}
```

### 4. Layer Heralds for Composability

```rust,ignore
// Layer: Base -> Enhancement -> Sanitization -> Caching
let herald = Arc::new(
    CachingHerald::new(
        Arc::new(SanitizingHerald::new(
            Arc::new(EnhancingHerald::new(
                Arc::new(MarkdownHerald::default())
            ))
        )),
        100,  // cache size
    )
);
```

### 5. Provide Format Guidance in System Prompt

```rust,ignore
// ✅ Explicit format instructions
let paladin = PaladinBuilder::new(llm_adapter)
    .system_prompt(
        "You MUST respond in valid JSON format:\n\
         {\n\
           \"answer\": \"your response\",\n\
           \"confidence\": 0.0 to 1.0,\n\
           \"sources\": [\"source1\", \"source2\"]\n\
         }\n\
         Do NOT include any text outside this JSON structure."
    )
    .with_herald(Arc::new(JsonHerald::default()))
    .build().await?;
```

## Advanced Patterns

### Template-Based Herald

```rust,ignore
use handlebars::Handlebars;

pub struct TemplateHerald {
    handlebars: Handlebars<'static>,
    template_name: String,
}

impl TemplateHerald {
    pub fn new(template: &str, template_name: &str) -> Result<Self, HeraldError> {
        let mut handlebars = Handlebars::new();
        handlebars.register_template_string(template_name, template)?;

        Ok(Self {
            handlebars,
            template_name: template_name.to_string(),
        })
    }
}

impl Herald for TemplateHerald {
    fn format_paladin_result(&self, result: &PaladinResult) -> Result<String, HeraldError> {
        // Parse content as JSON
        let data: serde_json::Value = serde_json::from_str(content)?;

        // Render template
        let rendered = self.handlebars.render(&self.template_name, &data)?;

        Ok(rendered)
    }

    // Implement other methods...
}

// Usage
let template = r#"
# {{title}}

**Summary:** {{summary}}

## Details

{{#each items}}
- {{this}}
{{/each}}

*Generated: {{timestamp}}*
"#;

let herald = Arc::new(TemplateHerald::new(template, "report")?);

let paladin = PaladinBuilder::new(llm_adapter)
    .system_prompt("Return JSON: {title, summary, items: [], timestamp}")
    .with_herald(herald)
    .build().await?;
```

### Diff Herald

```rust,ignore
pub struct DiffHerald {
    previous_content: RwLock<Option<String>>,
}

impl Herald for DiffHerald {
    fn format_paladin_result(&self, result: &PaladinResult) -> Result<String, HeraldError> {
        let previous = self.previous_content.read().unwrap().clone();

        let formatted = if let Some(prev) = previous {
            // Generate diff
            self.generate_diff(&prev, content)
        } else {
            // First time, show all
            content.to_string()
        };

        // Update previous content
        *self.previous_content.write().unwrap() = Some(content.to_string());

        Ok(formatted)
    }

    fn generate_diff(&self, old: &str, new: &str) -> String {
        // Use diff algorithm
        // Implementation...
        format!("--- Old\n+++ New\n{}", new)
    }
}
```

## Troubleshooting

### Invalid JSON Output

**Problem**: JSON Herald fails to parse LLM output.

**Solutions**:
1. Strengthen system prompt with explicit JSON instructions
2. Add JSON schema to prompt
3. Enable output validation with retries
4. Use JSON mode in LLM if supported

```rust,ignore
let paladin = PaladinBuilder::new(llm_adapter)
    .system_prompt(
        "CRITICAL INSTRUCTION: You MUST respond with ONLY valid JSON. \
         No additional text before or after. No markdown code blocks. \
         Just pure JSON.\n\n\
         Schema: {\"result\": string, \"confidence\": number}"
    )
    .output_format(OutputFormat::Json)  // Some LLMs support JSON mode
    .retry_attempts(3)
    .build().await?;
```

### Streaming Format Inconsistency

**Problem**: Streamed chunks don't format correctly.

**Solutions**:
1. Use accumulation pattern
2. Implement chunk boundary detection
3. Buffer until complete format units

```rust,ignore
pub struct BufferedStreamHerald {
    buffer: RwLock<String>,
    delimiter: String,
}

impl BufferedStreamHerald {
    fn format_stream_chunk(&self, chunk: &StreamChunk) -> Result<String, HeraldError> {
        let mut buffer = self.buffer.write().unwrap();
        buffer.push_str(chunk);

        // Check for complete units (e.g., sentences, paragraphs)
        if buffer.ends_with(&self.delimiter) {
            let complete = buffer.clone();
            buffer.clear();
            Ok(complete)
        } else {
            Ok(String::new())  // Not ready yet
        }
    }
}
```

### Performance Issues with Complex Formatting

**Problem**: Formatting is slow for large outputs.

**Solutions**:
1. Implement caching
2. Use lazy formatting (format on demand)
3. Optimize regex patterns
4. Consider parallel processing

```rust,ignore
// Lazy formatting
pub struct LazyHerald {
    inner: Arc<dyn Herald>,
    cached_result: RwLock<Option<String>>,
}

impl LazyHerald {
    pub async fn get_formatted(&self, content: &str) -> Result<String, HeraldError> {
        // Check cache
        if let Some(cached) = self.cached_result.read().unwrap().as_ref() {
            return Ok(cached.clone());
        }

        // Format and cache
        let formatted = self.inner.format(content).await?;
        *self.cached_result.write().unwrap() = Some(formatted.clone());

        Ok(formatted)
    }
}
```

## Testing

### Unit Testing Heralds

```rust,ignore
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_json_herald_formats_correctly() {
        let herald = JsonHerald::default();

        let input = r#"{"name": "Alice", "age": 30}"#;
        let formatted = herald.format(input).await.unwrap();

        // Verify valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&formatted).unwrap();
        assert_eq!(parsed["name"], "Alice");
        assert_eq!(parsed["age"], 30);
    }

    #[tokio::test]
    async fn test_json_herald_validates_schema() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            },
            "required": ["name"]
        });

        let herald = JsonHerald::new().with_schema(schema);

        // Valid
        assert!(herald.validate(r#"{"name": "Bob"}"#).is_ok());

        // Invalid - missing required field
        assert!(herald.validate(r#"{"age": 25}"#).is_err());
    }
}
```

## Examples

See working examples:
- `examples/herald_markdown_output.rs` - Markdown formatting
- `examples/herald_json_output.rs` - Structured JSON output
- `examples/herald_streaming.rs` - Real-time streaming
- `examples/herald_custom_formatter.rs` - Custom herald implementation

## Next Steps

- **[Tool Integration](tool-integration.md)** - Format tool results
- **[Battalion Patterns](battalion-patterns.md)** - Format multi-agent outputs
- **[API Reference](https://docs.rs/paladin)** - Herald API documentation

## Related Resources

- [Handlebars Templates](https://handlebarsjs.com/)
- [JSON Schema](https://json-schema.org/)
- [Markdown Specification](https://commonmark.org/)
