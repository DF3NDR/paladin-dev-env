# Sentinel Vision System

The Sentinel Vision System extends Paladin's AI agent framework with multimodal capabilities, enabling Paladins to analyze images and process documents alongside text. This comprehensive guide covers all aspects of vision and document processing in Paladin.

## Table of Contents

- [Introduction](#introduction)
- [Getting Started](#getting-started)
- [Vision Content Types](#vision-content-types)
- [Supported Providers](#supported-providers)
- [Paladin Vision API](#paladin-vision-api)
- [Document Processing](#document-processing)
- [CLI Usage](#cli-usage)
- [YAML Configuration](#yaml-configuration)
- [Security](#security)
- [Battalion Integration](#battalion-integration)
- [Error Handling](#error-handling)
- [Performance Considerations](#performance-considerations)
- [Troubleshooting](#troubleshooting)

## Introduction

The Sentinel Vision System brings multimodal AI capabilities to Paladin, allowing your AI agents to:

- **Analyze Images**: Process photos, screenshots, diagrams, charts, and visual data
- **Extract Text from Documents**: Parse PDFs, extract metadata, and chunk content intelligently
- **Combine Vision and Text**: Create agents that reason about both visual and textual information
- **Orchestrate Vision Workflows**: Use Battalion patterns to coordinate complex vision tasks
- **Secure Processing**: Encrypt sensitive visual data with automatic memory cleanup

### Architecture

Sentinel follows Paladin's hexagonal architecture:

```
┌─────────────────────────────────────────────────┐
│                 Application                      │
│  ┌──────────────────────────────────────────┐   │
│  │         Paladin Vision API               │   │
│  │  (PaladinBuilder::enable_vision)         │   │
│  └──────────────────────────────────────────┘   │
│                      │                           │
│           ┌──────────┴──────────┐               │
│           ▼                     ▼                │
│  ┌─────────────────┐   ┌─────────────────┐     │
│  │ VisionCapableLlm│   │  DocumentPort   │     │
│  │      Port       │   │     Port        │     │
│  └─────────────────┘   └─────────────────┘     │
└─────────────────────────────────────────────────┘
                     │
        ┌────────────┴────────────┐
        ▼                         ▼
┌──────────────┐         ┌──────────────┐
│ OpenAI Vision│         │ DocumentAdapter│
│ Anthropic    │         │ PdfExtractor │
└──────────────┘         └──────────────┘
```

## Getting Started

### Prerequisites

```toml
# Cargo.toml
[dependencies]
paladin-ai = "0.5"
tokio = { version = "1", features = ["full"] }
```

### Quick Example

```rust,ignore
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin_llm::openai::{OpenAIAdapter, OpenAIConfig};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create vision-capable LLM adapter
    let config = OpenAIConfig {
        api_key: std::env::var("OPENAI_API_KEY")?,
        base_url: "https://api.openai.com/v1".to_string(),
        organization: None,
        timeout_seconds: 300,
        max_retries: 3,
    };
    let llm = Arc::new(OpenAIAdapter::new(config)?);

    // 2. Build vision-enabled Paladin (model is set on the builder, not the config)
    let paladin = PaladinBuilder::new(llm)
        .name("ImageAnalyzer")
        .system_prompt("You are an expert image analyst. Describe images in detail.")
        .enable_vision(true)
        .model("gpt-4o")
        .build()?;

    // 3. Analyze an image
    let result = paladin.execute_with_vision(
        "What do you see in this image?",
        vec![VisionContent::ImageFile {
            path: PathBuf::from("./photo.jpg"),
            detail: ImageDetail::Auto,
        }]
    ).await?;

    println!("Analysis: {}", result.output);
    Ok(())
}
```

## Vision Content Types

Sentinel supports three ways to provide images to vision-capable Paladins:

### ImageUrl

Reference images via HTTP/HTTPS URLs:

```rust,ignore
use paladin::core::platform::container::vision::{VisionContent, ImageDetail};

let content = VisionContent::ImageUrl {
    url: "https://example.com/photo.jpg".to_string(),
    detail: ImageDetail::High,
};
```

**Best for**: Publicly accessible images, web scraping, API integrations

### ImageBase64

Embed images as base64-encoded strings:

```rust,ignore
let base64_data = "iVBORw0KGgoAAAANSUhEUg..."; // Base64-encoded image

let content = VisionContent::ImageBase64 {
    data: base64_data.to_string(),
    media_type: "image/png".to_string(),
    detail: ImageDetail::Auto,
};
```

**Best for**: Small images, embedded data, when URLs aren't available

### ImageFile

Load images from the local filesystem:

```rust,ignore
use std::path::PathBuf;

let content = VisionContent::ImageFile {
    path: PathBuf::from("./assets/diagram.png"),
    detail: ImageDetail::Low,
};
```

**Best for**: Local processing, batch operations, development/testing

### Image Detail Levels

Control the resolution and token usage:

```rust,ignore
pub enum ImageDetail {
    Auto,  // Let the model decide (balanced)
    Low,   // Faster, cheaper, less detail (512x512 max)
    High,  // Slower, more expensive, more detail (2048x2048 max)
}
```

**Recommendation**: Start with `Auto`, use `Low` for speed/cost, `High` for precision.

### Supported Formats

- **PNG** (Portable Network Graphics)
- **JPEG** (Joint Photographic Experts Group)
- **GIF** (Graphics Interchange Format) - first frame only
- **WebP** (Web Picture format)

## Supported Providers

### OpenAI Vision

**Models**: `gpt-4o`, `gpt-4o-mini`, `gpt-4-vision-preview`

```rust,ignore
use paladin_llm::openai::{OpenAIAdapter, OpenAIConfig};

// The model ("gpt-4o") is set on the PaladinBuilder, not on OpenAIConfig -- see the Quick Example above.
let config = OpenAIConfig {
    api_key: env::var("OPENAI_API_KEY")?,
    base_url: "https://api.openai.com/v1".to_string(),
    organization: None,
    timeout_seconds: 300,
    max_retries: 3,
};

let llm = Arc::new(OpenAIAdapter::new(config)?);
```

**Features**:
- High-quality image understanding
- Automatic image resizing
- Support for multiple images (up to 10)
- Fast inference

**Token Estimation**:
- Low detail: ~85 tokens per image
- High detail: ~170 tokens per 512x512 tile
- Auto detail: Model decides based on image size

### Anthropic Vision

**Models**: `claude-3-opus-20240229`, `claude-3-sonnet-20240229`, `claude-3-haiku-20240307`

```rust,ignore
use paladin_llm::anthropic::{AnthropicAdapter, AnthropicConfig};

let config = AnthropicConfig {
    api_key: env::var("ANTHROPIC_API_KEY")?,
    model: "claude-3-opus-20240229".to_string(),
    base_url: "https://api.anthropic.com/v1".to_string(),
    max_tokens: 4096,
    timeout_seconds: 300,
};

let llm = Arc::new(AnthropicAdapter::new(config)?);
```

**Features**:
- Excellent OCR and text extraction
- Strong diagram understanding
- Multiple images supported (up to 20)
- Base64 encoding required (automatic conversion)

**Note**: Anthropic models automatically convert ImageUrl to base64 internally.

### Capability Detection

```rust,ignore
let capabilities = llm.get_capabilities();
if capabilities.supports_vision {
    println!("Provider: {}", llm.get_provider_name());
    // Use vision features
} else {
    println!("Vision not supported by this provider");
}
```

## Paladin Vision API

### Building Vision-Enabled Paladins

```rust,ignore
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;

let paladin = PaladinBuilder::new(llm_port)
    .name("VisionPaladin")
    .system_prompt("You are a visual analysis expert")
    .enable_vision(true)          // Enable vision capabilities
    .model("gpt-4o")               // Use vision-capable model
    .temperature(0.7)
    .max_loops(3)
    .build()?;
```

### Executing with Vision

```rust,ignore
use paladin::core::platform::container::vision::VisionContent;

// Single image
let images = vec![VisionContent::ImageFile {
    path: PathBuf::from("photo.jpg"),
    detail: ImageDetail::Auto,
}];

let result = paladin.execute_with_vision(
    "Describe this image in detail",
    images
).await?;

// Multiple images
let images = vec![
    VisionContent::ImageUrl {
        url: "https://example.com/before.jpg".to_string(),
        detail: ImageDetail::High,
    },
    VisionContent::ImageUrl {
        url: "https://example.com/after.jpg".to_string(),
        detail: ImageDetail::High,
    },
];

let result = paladin.execute_with_vision(
    "Compare these two images and identify the differences",
    images
).await?;
```

### With Memory (Garrison)

```rust,ignore
use paladin::infrastructure::adapters::garrison::SqliteGarrison;

let garrison = Arc::new(SqliteGarrison::new("memory.db")?);

let paladin = PaladinBuilder::new(llm_port)
    .enable_vision(true)
    .with_garrison(garrison)
    .build()?;

// Vision analysis is stored in Garrison
// Subsequent calls can reference previous analyses
```

### With RAG (Sanctum)

```rust,ignore
use paladin::infrastructure::adapters::sanctum::QdrantSanctum;
use paladin::application::services::sanctum::rag_retrieval_service::RagRetrievalService;

let sanctum = Arc::new(QdrantSanctum::new(config)?);
let rag_service = Arc::new(RagRetrievalService::new(sanctum));

let paladin = PaladinBuilder::new(llm_port)
    .enable_vision(true)
    .with_rag_retrieval(rag_service)
    .build()?;

// Retrieves relevant context from Sanctum
// Combines with vision analysis
```

## Document Processing

### PDF Text Extraction

```rust,ignore
use paladin::infrastructure::adapters::document::pdf_extractor::PdfExtractor;
use std::path::Path;

let extractor = PdfExtractor::new();

// From file path
let document = extractor.extract(Path::new("report.pdf"))?;

// From bytes
let pdf_bytes = std::fs::read("report.pdf")?;
let document = extractor.extract_bytes(&pdf_bytes)?;

// Access content
println!("Title: {:?}", document.metadata.title);
println!("Pages: {}", document.metadata.page_count);
for page in &document.pages {
    println!("Page {}: {} chars", page.number, page.content.len());
}
```

### DocumentPort Interface

```rust,ignore
use paladin::paladin_ports::input::document_port::{
    DocumentPort, DocumentSource, ChunkConfig
};
use paladin::infrastructure::adapters::document::DocumentAdapter;

let adapter = Arc::new(DocumentAdapter::new());

// Ingest from various sources
let document = adapter.ingest(DocumentSource::File(PathBuf::from("doc.pdf"))).await?;

// Or from bytes
let document = adapter.ingest(DocumentSource::Bytes {
    data: pdf_bytes,
    format: DocumentFormat::Pdf,
}).await?;

// Chunk for RAG
let config = ChunkConfig {
    chunk_size: 1000,
    chunk_overlap: 200,
    separator: "\n\n".to_string(),
};

let chunks = adapter.chunk(&document, config).await;
for chunk in chunks {
    println!("Chunk {}: {} chars", chunk.chunk_index, chunk.content.len());
}
```

### Supported Document Formats

| Format | Extension | Features |
|--------|-----------|----------|
| PDF | `.pdf` | Text extraction, metadata, multi-page |
| Text | `.txt` | Plain text processing |
| Markdown | `.md` | Markdown parsing |

### Document Metadata

```rust,ignore
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub page_count: usize,
    pub creation_date: Option<DateTime<Utc>>,
}
```

### Intelligent Chunking

```rust,ignore
let config = ChunkConfig {
    chunk_size: 500,        // Target chunk size in characters
    chunk_overlap: 100,     // Overlap between chunks
    separator: "\n\n",      // Split on paragraphs
};

let chunks = adapter.chunk(&document, config).await;
```

**Best Practices**:
- **chunk_size**: 500-1500 characters for RAG, 2000-4000 for summarization
- **chunk_overlap**: 10-20% of chunk_size for context preservation
- **separator**: `\n\n` for paragraphs, `\n` for lines, `.` for sentences

## CLI Usage

### Image Analysis

Analyze a single image:

```bash
paladin agent run vision_analyzer --image photo.jpg --task "Describe this image"
```

Multiple images:

```bash
paladin agent run comparator \
  --image before.jpg \
  --image after.jpg \
  --task "Compare these images"
```

### Document Processing

Process a PDF document:

```bash
paladin agent run document_analyzer \
  --document report.pdf \
  --task "Summarize this document"
```

### Combined Vision and Document

```bash
paladin agent run multimodal_agent \
  --image chart.png \
  --document report.pdf \
  --task "Explain the chart in context of the report"
```

### Using Configuration Files

```bash
paladin agent run vision_agent --config vision_config.yaml
```

## YAML Configuration

### Basic Vision Configuration

```yaml
# vision_config.yaml
name: "ImageAnalyzer"
system_prompt: "You are an expert at analyzing images"
model: "gpt-4o"
temperature: 0.7
max_loops: 1
vision_enabled: true

images:
  - "./photos/sample1.jpg"
  - "./photos/sample2.jpg"

task: "Analyze these images and describe what you see"
```

### Advanced Configuration

```yaml
# advanced_vision_config.yaml
name: "AdvancedVisionPaladin"
system_prompt: |
  You are an advanced image analysis system.
  Provide detailed technical descriptions.
model: "gpt-4o"
temperature: 0.3
max_loops: 3
timeout_seconds: 600
vision_enabled: true

# Images to analyze
images:
  - "./data/medical_scan.jpg"
  - "https://example.com/reference.png"

# Documents for context
documents:
  - "./data/medical_guidelines.pdf"

# Memory configuration
garrison:
  type: "sqlite"
  path: "./memory.db"

# RAG configuration
sanctum:
  enabled: true
  collection: "medical_knowledge"

# Security
encryption:
  enabled: true
  data_retention_days: 30
```

### Configuration with Battalion

```yaml
# vision_battalion.yaml
battalion:
  type: "formation"
  name: "ImagePipeline"

paladins:
  - name: "Detector"
    system_prompt: "Detect objects in images"
    model: "gpt-4o"
    vision_enabled: true

  - name: "Classifier"
    system_prompt: "Classify detected objects"
    model: "gpt-4o"
    vision_enabled: true

  - name: "Reporter"
    system_prompt: "Generate analysis report"
    model: "gpt-4"
    vision_enabled: false

images:
  - "./input/image.jpg"
```

### Vision Configuration (Retry & Limits)

Epic 20 introduced comprehensive vision configuration for retry logic and token limits:

```yaml
# config.yml
vision:
  # Retry configuration for failed vision API calls
  retry:
    max_retries: 3                # Maximum retry attempts
    initial_backoff_ms: 1000      # Initial backoff delay (1 second)
    backoff_multiplier: 2.0       # Exponential backoff multiplier

  # Provider-specific limits
  openai:
    max_tokens: 4096              # Maximum tokens for OpenAI vision requests

  anthropic:
    max_tokens: 4096              # Maximum tokens for Anthropic vision requests
```

**Retry Behavior**:
- Automatic retry on transient failures (network errors, rate limits, timeouts)
- Exponential backoff: delay increases as `initial_backoff_ms * (backoff_multiplier ^ attempt)`
- Example delays: 1s → 2s → 4s for 3 retries with 2.0 multiplier
- Non-retryable errors (authentication, invalid format) fail immediately

**Using Configuration in Code**:

```rust,ignore
use paladin::config::application_settings::ApplicationSettings;

let settings = ApplicationSettings::load("config.yml")?;

// Configuration is automatically applied to vision adapters
let openai_adapter = OpenAIAdapter::new_with_vision_config(
    openai_config,
    settings.vision.clone()
)?;

let anthropic_adapter = AnthropicAdapter::new_with_vision_config(
    anthropic_config,
    settings.vision.clone()
)?;
```

**Best Practices**:
- **Development**: Lower `max_retries` (1-2) for faster feedback
- **Production**: Higher `max_retries` (3-5) for reliability
- **High Traffic**: Lower `backoff_multiplier` (1.5) to reduce total wait time
- **Rate Limited APIs**: Higher `backoff_multiplier` (3.0) to respect limits

## Security

### Encryption at Rest

```rust,ignore
use paladin::infrastructure::security::encryption::{EncryptionService, SecureData};

let encryption = EncryptionService::new();

// Encrypt image data
let image_data = std::fs::read("photo.jpg")?;
let encrypted = encryption.encrypt_image_data(&image_data)?;

// Decrypt to secure memory (auto-zeroized on drop)
let decrypted: SecureData<Vec<u8>> = encryption.decrypt_image_data(&encrypted)?;

// Use decrypted data
// Memory is automatically zeroed when SecureData goes out of scope
```

### Data Retention

```rust,ignore
use paladin::infrastructure::security::encryption::DataRetentionPolicy;
use std::time::Duration;

let policy = DataRetentionPolicy {
    ttl: Duration::from_secs(30 * 24 * 60 * 60), // 30 days
    auto_cleanup: true,
};

// Check if data should be retained
let secure_data = encryption.decrypt_image_data(&encrypted)?;
if !policy.should_retain(&secure_data) {
    // Data has expired
}
```

### Audit Logging

```rust,ignore
use paladin::infrastructure::security::audit::AuditLogger;

let audit = AuditLogger::new(true);

// Log file access (no sensitive data)
audit.log_file_access("user123", "photo.jpg", "read", true, None);

// Log LLM API call (no prompts/responses)
audit.log_llm_api_call("user123", "openai", "gpt-4o", true, None);

// Log vision processing (no image data)
audit.log_vision_processing("user123", 3, "analysis_complete", true, None);
```

**Security Features**:
- ✅ ChaCha20-Poly1305 AEAD encryption
- ✅ Automatic memory zeroization
- ✅ Configurable data retention (default: 30 days)
- ✅ Audit logging without sensitive data
- ✅ TLS/HTTPS for all API calls
- ✅ Certificate validation enabled

## Battalion Integration

All Battalion patterns work seamlessly with vision-enabled Paladins. See [BATTALION_VISION_SUPPORT.md](battalion-vision-support.md) for comprehensive examples.

### Formation: Sequential Vision Processing

```rust,ignore
use paladin::application::services::battalion::formation_service::FormationExecutionService;
use paladin::core::platform::container::battalion::formation::Formation;

let detector = create_vision_paladin("object_detector");
let classifier = create_vision_paladin("object_classifier");
let reporter = create_text_paladin("report_generator");

let formation = Formation::new(
    vec![detector, classifier, reporter],
    BattalionConfig::new("vision_pipeline")
)?;

let service = FormationExecutionService::new(paladin_port);
let result = service.execute(&formation, "Analyze image.jpg").await?;
```

### Phalanx: Parallel Vision Processing

```rust,ignore
use paladin::application::services::battalion::phalanx_service::PhalanxExecutionService;
use paladin::core::platform::container::battalion::phalanx::Phalanx;

let paladins = vec![
    create_vision_paladin("object_detector"),
    create_vision_paladin("face_detector"),
    create_vision_paladin("text_detector"),
];

let phalanx = Phalanx::new(paladins, BattalionConfig::new("parallel_analysis"))?
    .with_aggregation(AggregationStrategy::Concatenate);

let service = PhalanxExecutionService::new(paladin_port);
let result = service.execute(&phalanx, "Analyze all aspects of image.jpg").await?;
```

## Error Handling

### VisionError Types

```rust,ignore
use paladin::core::platform::container::vision::VisionError;

match result {
    Err(VisionError::UnsupportedFormat(fmt)) => {
        eprintln!("Unsupported format: {}", fmt);
    }
    Err(VisionError::FileTooLarge { size, max_size }) => {
        eprintln!("File too large: {} bytes (max: {})", size, max_size);
    }
    Err(VisionError::InvalidImage(msg)) => {
        eprintln!("Invalid image: {}", msg);
    }
    Err(VisionError::ModelNotSupported(model)) => {
        eprintln!("Model doesn't support vision: {}", model);
    }
    Err(VisionError::NetworkError(err)) => {
        eprintln!("Network error: {}", err);
    }
    Ok(result) => {
        println!("Success: {}", result);
    }
}
```

### DocumentError Types

```rust,ignore
use paladin::core::platform::container::document::DocumentError;

match document_result {
    Err(DocumentError::UnsupportedFormat(fmt)) => {
        eprintln!("Unsupported document format: {}", fmt);
    }
    Err(DocumentError::EncryptedPdf) => {
        eprintln!("PDF is encrypted and cannot be processed");
    }
    Err(DocumentError::CorruptedFile(msg)) => {
        eprintln!("File is corrupted: {}", msg);
    }
    Err(DocumentError::ExtractionFailed(msg)) => {
        eprintln!("Extraction failed: {}", msg);
    }
    Ok(document) => {
        println!("Extracted {} pages", document.pages.len());
    }
}
```

### PaladinError Integration

```rust,ignore
use paladin::application::services::paladin::error::PaladinError;

match paladin.execute_with_vision(task, images).await {
    Err(PaladinError::ConfigurationError(msg)) => {
        eprintln!("Configuration error: {}", msg);
        // Check vision_enabled flag and model support
    }
    Err(PaladinError::ExecutionError(msg)) => {
        eprintln!("Execution error: {}", msg);
        // Check API keys, network, LLM provider status
    }
    Err(PaladinError::Timeout(secs)) => {
        eprintln!("Timeout after {} seconds", secs);
        // Increase timeout or reduce image size
    }
    Ok(result) => {
        println!("Analysis: {}", result.output);
    }
}
```

## Performance Considerations

### Image Size Optimization

**Provider Image Size Limits**:
- **OpenAI**: Maximum 20MB per image
- **Anthropic**: Maximum 5MB per image (base64-encoded)
- **Recommended**: Keep images under 2MB for optimal performance

**Recommendations**:
- Maximum size: 20MB (OpenAI), 5MB (Anthropic)
- Optimal resolution: 1024x1024 for most tasks
- Use `ImageDetail::Low` for faster processing
- Compress images before upload to reduce latency

```rust,ignore
// Fast processing (low detail)
VisionContent::ImageFile {
    path: PathBuf::from("large_image.jpg"),
    detail: ImageDetail::Low,  // Max 512x512
}

// Detailed analysis (high detail)
VisionContent::ImageFile {
    path: PathBuf::from("diagram.png"),
    detail: ImageDetail::High,  // Up to 2048x2048
}
```

### Batch Processing

Use Phalanx for parallel processing:

```rust,ignore
// Process 100 images in parallel with 10 Paladins
let paladins: Vec<Paladin> = (0..10)
    .map(|i| create_vision_paladin(&format!("processor_{}", i)))
    .collect();

let phalanx = Phalanx::new(paladins, config)?
    .with_max_concurrency(10);  // Limit concurrent requests

// Each Paladin processes ~10 images
let result = service.execute(&phalanx, "Process batch of 100 images").await?;
```

### Token Management

**OpenAI Token Costs**:
- Low detail: ~85 tokens per image
- High detail: ~170 tokens per 512x512 tile
- Text prompt: varies by length

**Anthropic Token Costs**:
- Base64 encoding adds overhead
- Similar token counts to OpenAI

**Optimization**:
1. Use `ImageDetail::Auto` for balanced cost/quality
2. Compress images before processing
3. Cache results in Garrison for repeated analyses
4. Use Formation to build on previous results

### API Rate Limits

```rust,ignore
// Add delays for rate limit compliance
use tokio::time::{sleep, Duration};

for image in images {
    let result = paladin.execute_with_vision(task, vec![image]).await?;
    sleep(Duration::from_millis(1000)).await;  // 1 request/second
}
```

## Troubleshooting

### Vision Not Working

**Symptom**: `ModelNotSupported` error

**Solutions**:
1. Verify vision-capable model:
   ```rust
   .model("gpt-4o")  // ✅ Supports vision
   // Not .model("gpt-4")  // ❌ No vision
   ```

2. Enable vision flag:
   ```rust
   .enable_vision(true)  // Required!
   ```

3. Check provider capabilities:
   ```rust
   let caps = llm.get_capabilities();
   assert!(caps.supports_vision);
   ```

### Image Not Loading

**Symptom**: `InvalidImage` or `FileNotFound` error

**Solutions**:
1. Verify file exists and path is correct
2. Check file format (PNG, JPEG, GIF, WebP only)
3. Verify file size < 20MB
4. For URLs, ensure publicly accessible

### PDF Extraction Fails

**Symptom**: `ExtractionFailed` or `EncryptedPdf` error

**Solutions**:
1. Check if PDF is encrypted:
   ```bash
   pdfinfo document.pdf | grep Encrypted
   ```
2. Decrypt PDF first using external tools
3. Verify PDF is not corrupted
4. Try different PDF version (some v1.7+ features unsupported)

### Out of Memory

**Symptom**: Process killed or OOM error

**Solutions**:
1. Use `ImageDetail::Low` to reduce memory usage
2. Process images sequentially instead of parallel
3. Limit Phalanx concurrency:
   ```rust
   .with_max_concurrency(5)
   ```
4. Enable data retention cleanup

### Slow Performance

**Symptom**: Vision processing takes too long

**Solutions**:
1. Use `ImageDetail::Low` for faster inference
2. Reduce image resolution before processing
3. Use Phalanx for parallel batch processing
4. Cache results in Garrison
5. Check network latency to API endpoints

### Token Limits Exceeded

**Symptom**: API error about context length

**Solutions**:
1. Reduce image detail level
2. Use fewer images per request
3. Shorten text prompts
4. Split into multiple requests

## Examples

See the `examples/` directory for complete working examples:

- **vision_analysis.rs**: Single-image analysis
- **document_processing.rs**: PDF extraction and chunking
- **vision_battalion.rs**: Multi-agent vision workflows

Run examples with:

```bash
cargo run --example vision_analysis
cargo run --example document_processing
cargo run --example vision_battalion
```

## Further Reading

- [Battalion Vision Support](battalion-vision-support.md) - Detailed Battalion integration
- [Paladin Vision API](../api-reference/stable-api.md) - Complete API reference
- [Security Guide](security-scanning.md) - Encryption and data protection
- [Performance Tuning](../operations/performance-tuning.md) - Optimization strategies

## Contributing

See [CONTRIBUTING.md](../contributing/development-setup.md) for guidelines on extending vision capabilities.

---

**Sentinel Vision System** is part of Epic 13 and brings multimodal AI to Paladin's agent framework.
