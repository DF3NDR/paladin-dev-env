# Paladin Project Plan: Advanced Capabilities Expansion

## Document Information
- **Version:** 1.0
- **Created:** January 29, 2026
- **Status:** Draft
- **Total Estimated Duration:** 14-18 weeks
- **Total New Epics:** 8 (Epics 11-18)

---

## Executive Summary

This project plan defines the work required to expand Paladin into a fully-featured enterprise multi-agent orchestration framework. The plan introduces 8 new epics covering long-term memory, multi-modal processing, advanced orchestration patterns, agent autonomy features, and CLI enhancements.

### Timeline Overview

```
Week 1-2:   Epic 11 (Sanctum Memory Foundation)
Week 3-4:   Epic 12 (Sanctum RAG Integration)
Week 5-6:   Epic 13 (Sentinel Vision System)
Week 7-8:   Epic 14 (Autonomous Agent Features)
Week 9-10:  Epic 15 (Conclave Expert Synthesis)
Week 11-12: Epic 16 (Advanced Battalion Patterns)
Week 13-14: Epic 17 (Tactical Flow DSL)
Week 15-16: Epic 18 (Armory CLI Enhancement)
Week 17-18: Integration Testing & Documentation
```

---

## Paladin Component Naming Convention

| Component | Name | Description |
|-----------|------|-------------|
| AI Agent | **Paladin** | Autonomous AI agent entity |
| Multi-Agent System | **Battalion** | Coordinated group of Paladins |
| Sequential Execution | **Formation** | Paladins execute in ordered sequence |
| Parallel Execution | **Phalanx** | Paladins execute simultaneously |
| DAG Execution | **Campaign** | Graph-based conditional routing |
| Hierarchical Delegation | **Chain of Command** | Leader delegates to specialists |
| Expert Synthesis | **Conclave** | Experts advise, Loremaster concludes |
| Group Discussion | **Council** | Paladins deliberate collaboratively |
| Expertise Routing | **Grove** | Guild-based specialist selection |
| Flow-Based Orchestration | **Maneuver** | Tactical movement patterns |
| Universal Orchestrator | **Commander** | Strategic decision maker |
| Long-Term Memory | **Sanctum** | Sacred repository of knowledge |
| Short-Term Memory | **Garrison** | Active conversation history |
| Tool System | **Arsenal** | Collection of external capabilities |
| Individual Tool | **Armament** | Single tool or weapon |
| Output Formatting | **Herald** | Announces results in proper form |
| State Persistence | **Citadel** | Fortified state storage |
| CLI Interface | **Armory** | Workshop for Paladin management |
| Document/PDF | **Scroll** | Written intelligence to process |
| Delegate Agent | **Squire** | Specialist receiving delegated tasks |
| Aggregator Agent | **Loremaster** | Synthesizes expert knowledge |
| Moderator Agent | **Herald** | Facilitates council discussions |
| Agent Group | **Guild** | Collection of related specialists |
| System Prompt | **Oath** | Sacred vow defining Paladin purpose |

---

## Epic 11: Sanctum Memory Foundation

**Theme:** Vector Embedding Infrastructure  
**Duration:** 2 weeks  
**Priority:** Critical  
**Dependencies:** None  

### Description
Establish the foundational infrastructure for vector-based long-term memory, including embedding generation ports, vector storage traits, and the core data structures needed for semantic search. The Sanctum represents the sacred repository where Paladin knowledge is preserved across sessions.

### User Stories

#### US-11.1: Embedding Port Definition
**As a** framework developer  
**I want** a standardized port for generating vector embeddings  
**So that** I can plug in different embedding providers

**Acceptance Criteria:**
- [ ] `EmbeddingPort` trait defined in `src/application/ports/output/embedding_port.rs`
- [ ] Trait includes `embed_text(&str) -> Result<Vec<f32>, EmbeddingError>`
- [ ] Trait includes `embed_batch(&[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError>`
- [ ] Trait includes `dimension() -> usize` for vector dimension
- [ ] `EmbeddingError` enum covers: NetworkError, RateLimited, InvalidInput, ProviderError
- [ ] Unit tests for error handling

**Definition of Done:**
```rust
#[async_trait]
pub trait EmbeddingPort: Send + Sync {
    async fn embed_text(&self, text: &str) -> Result<Embedding, EmbeddingError>;
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>, EmbeddingError>;
    fn dimension(&self) -> usize;
    fn model_name(&self) -> &str;
}

pub struct Embedding {
    pub vector: Vec<f32>,
    pub model: String,
    pub dimension: usize,
    pub token_count: Option<u32>,
}
```

---

#### US-11.2: OpenAI Embedding Adapter
**As a** developer  
**I want** to generate embeddings using OpenAI's API  
**So that** I can use industry-standard embedding models

**Acceptance Criteria:**
- [ ] `OpenAIEmbeddingAdapter` implements `EmbeddingPort`
- [ ] Supports `text-embedding-3-small` (1536 dimensions)
- [ ] Supports `text-embedding-3-large` (3072 dimensions)
- [ ] Supports `text-embedding-ada-002` (1536 dimensions, legacy)
- [ ] Configurable via `OpenAIEmbeddingConfig` struct
- [ ] Implements retry with exponential backoff
- [ ] Batch processing respects API limits (max 2048 inputs)
- [ ] Integration test with mocked responses

**Definition of Done:**
```rust
pub struct OpenAIEmbeddingAdapter {
    client: reqwest::Client,
    config: OpenAIEmbeddingConfig,
}

pub struct OpenAIEmbeddingConfig {
    pub api_key: String,
    pub model: String,  // "text-embedding-3-small" default
    pub base_url: String,
    pub max_retries: u32,
    pub timeout_seconds: u64,
}
```

---

#### US-11.3: Sanctum Port Definition
**As a** framework developer  
**I want** a standardized port for vector storage operations  
**So that** I can plug in different vector databases

**Acceptance Criteria:**
- [ ] `SanctumPort` trait defined in `src/application/ports/output/sanctum_port.rs`
- [ ] Supports `store(id, vector, metadata)` operation
- [ ] Supports `search(vector, top_k, filter)` operation
- [ ] Supports `delete(id)` operation
- [ ] Supports `update(id, vector, metadata)` operation
- [ ] Filter supports metadata-based filtering
- [ ] Returns `SanctumSearchResult` with score and metadata

**Definition of Done:**
```rust
#[async_trait]
pub trait SanctumPort: Send + Sync {
    async fn store(&self, entry: SanctumEntry) -> Result<(), SanctumError>;
    async fn store_batch(&self, entries: Vec<SanctumEntry>) -> Result<(), SanctumError>;
    async fn search(&self, query: SanctumQuery) -> Result<Vec<SanctumSearchResult>, SanctumError>;
    async fn delete(&self, id: &str) -> Result<bool, SanctumError>;
    async fn update(&self, entry: SanctumEntry) -> Result<(), SanctumError>;
    async fn count(&self) -> Result<u64, SanctumError>;
}

pub struct SanctumEntry {
    pub id: String,
    pub paladin_id: String,
    pub content: String,
    pub embedding: Vec<f32>,
    pub metadata: HashMap<String, Value>,
    pub timestamp: DateTime<Utc>,
}

pub struct SanctumQuery {
    pub embedding: Vec<f32>,
    pub top_k: usize,
    pub filter: Option<SanctumFilter>,
    pub min_score: Option<f32>,
}

pub struct SanctumSearchResult {
    pub entry: SanctumEntry,
    pub score: f32,
}
```

---

#### US-11.4: In-Memory Sanctum Adapter
**As a** developer  
**I want** an in-memory vector store for development and testing  
**So that** I can prototype without external dependencies

**Acceptance Criteria:**
- [ ] `InMemorySanctum` implements `SanctumPort`
- [ ] Uses brute-force cosine similarity for search
- [ ] Thread-safe with `RwLock`
- [ ] Supports all CRUD operations
- [ ] Configurable max capacity with LRU eviction
- [ ] Unit tests for all operations
- [ ] Performance acceptable for < 10,000 vectors

**Definition of Done:**
```rust
pub struct InMemorySanctum {
    entries: Arc<RwLock<HashMap<String, SanctumEntry>>>,
    config: InMemorySanctumConfig,
}

pub struct InMemorySanctumConfig {
    pub max_entries: usize,
    pub eviction_strategy: EvictionStrategy,
}
```

---

#### US-11.5: Sanctum Domain Model
**As a** framework developer  
**I want** domain models for long-term memory concepts  
**So that** the system has clear bounded contexts

**Acceptance Criteria:**
- [ ] `SanctumEntry` struct in `src/core/platform/container/sanctum.rs`
- [ ] `Memory` value object representing a stored insight
- [ ] `MemoryType` enum: Episodic, Semantic, Procedural
- [ ] Serialization/deserialization support
- [ ] Validation for embedding dimensions

**Definition of Done:**
```rust
pub struct Memory {
    pub id: Uuid,
    pub paladin_id: String,
    pub content: String,
    pub memory_type: MemoryType,
    pub importance: f32,  // 0.0 - 1.0
    pub access_count: u32,
    pub last_accessed: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub metadata: HashMap<String, Value>,
}

pub enum MemoryType {
    Episodic,    // Specific events/conversations
    Semantic,    // Facts and knowledge
    Procedural,  // How-to instructions
}
```

---

### Epic 11 Completion Criteria
- [ ] All 5 user stories completed and tested
- [ ] `EmbeddingPort` trait with OpenAI implementation
- [ ] `SanctumPort` trait with in-memory implementation
- [ ] Domain models for memory concepts
- [ ] Documentation in `docs/SANCTUM.md`
- [ ] Example: `examples/sanctum_basics.rs`

---

## Epic 12: Sanctum RAG Integration

**Theme:** Retrieval-Augmented Generation  
**Duration:** 2 weeks  
**Priority:** Critical  
**Dependencies:** Epic 11  

### Description
Integrate the Sanctum memory system with Paladin agents to enable RAG (Retrieval-Augmented Generation) capabilities, including automatic memory storage, retrieval during execution, and memory consolidation. This allows Paladins to draw upon accumulated wisdom from past encounters.

### User Stories

#### US-12.1: Qdrant Sanctum Adapter
**As a** developer  
**I want** to use Qdrant as a production vector database  
**So that** I can scale to millions of memories

**Acceptance Criteria:**
- [ ] `QdrantSanctum` implements `SanctumPort`
- [ ] Connects via `qdrant-client` crate
- [ ] Supports collection creation with configurable settings
- [ ] Supports payload filtering on metadata
- [ ] Configurable connection (local, cloud, API key)
- [ ] Health check implementation
- [ ] Integration test with Qdrant container

**Definition of Done:**
```rust
pub struct QdrantSanctum {
    client: QdrantClient,
    config: QdrantSanctumConfig,
}

pub struct QdrantSanctumConfig {
    pub url: String,
    pub api_key: Option<String>,
    pub collection_name: String,
    pub vector_size: u64,
    pub distance: Distance,  // Cosine, Euclidean, Dot
    pub on_disk: bool,
}
```

---

#### US-12.2: Paladin Sanctum Integration
**As a** developer  
**I want** to configure a Paladin with long-term memory  
**So that** it can remember across sessions

**Acceptance Criteria:**
- [ ] `PaladinBuilder::with_sanctum(sanctum)` method
- [ ] `PaladinBuilder::with_embedding_port(port)` method
- [ ] Paladin stores conversation insights automatically
- [ ] Memory extraction configurable (every turn, on completion, manual)
- [ ] Memory importance scoring based on content analysis

**Definition of Done:**
```rust
impl PaladinBuilder {
    pub fn with_sanctum(mut self, sanctum: Arc<dyn SanctumPort>) -> Self;
    pub fn with_embedding_port(mut self, embedding: Arc<dyn EmbeddingPort>) -> Self;
    pub fn memory_extraction_strategy(mut self, strategy: MemoryExtractionStrategy) -> Self;
}

pub enum MemoryExtractionStrategy {
    EveryTurn,
    OnCompletion,
    Manual,
    Threshold { importance: f32 },
}
```

---

#### US-12.3: Sanctum Retrieval Service
**As a** developer  
**I want** automatic context retrieval from long-term memory  
**So that** Paladins have relevant historical context

**Acceptance Criteria:**
- [ ] `SanctumRetrievalService` in `src/application/use_cases/sanctum/`
- [ ] Retrieves top-k relevant memories before LLM call
- [ ] Configurable retrieval trigger (always, keyword-based, semantic threshold)
- [ ] Formats retrieved memories for prompt injection
- [ ] Deduplication of similar memories
- [ ] Respects token budget for context

**Definition of Done:**
```rust
pub struct SanctumRetrievalService {
    sanctum: Arc<dyn SanctumPort>,
    embedding: Arc<dyn EmbeddingPort>,
    config: SanctumRetrievalConfig,
}

pub struct SanctumRetrievalConfig {
    pub top_k: usize,
    pub min_similarity: f32,
    pub max_tokens: usize,
    pub retrieval_trigger: RetrievalTrigger,
}

impl SanctumRetrievalService {
    pub async fn retrieve_context(
        &self,
        paladin_id: &str,
        query: &str,
    ) -> Result<Vec<Memory>, SanctumError>;

    pub fn format_for_prompt(&self, memories: &[Memory]) -> String;
}
```

---

#### US-12.4: Memory Extraction Service
**As a** developer  
**I want** automatic extraction of memorable information from conversations  
**So that** important insights are preserved in the Sanctum

**Acceptance Criteria:**
- [ ] `MemoryExtractionService` in `src/application/use_cases/sanctum/`
- [ ] Uses LLM to identify memorable content
- [ ] Extracts: facts, preferences, events, instructions
- [ ] Assigns importance scores
- [ ] Categorizes by memory type
- [ ] Deduplicates against existing memories

**Definition of Done:**
```rust
pub struct MemoryExtractionService {
    llm: Arc<dyn LlmPort>,
    embedding: Arc<dyn EmbeddingPort>,
    sanctum: Arc<dyn SanctumPort>,
}

impl MemoryExtractionService {
    pub async fn extract_memories(
        &self,
        paladin_id: &str,
        conversation: &[GarrisonEntry],
    ) -> Result<Vec<Memory>, SanctumError>;

    pub async fn store_memories(
        &self,
        memories: Vec<Memory>,
    ) -> Result<(), SanctumError>;
}
```

---

#### US-12.5: PaladinExecutionService Sanctum Integration
**As a** developer  
**I want** Sanctum integrated into the execution flow  
**So that** memory retrieval is automatic

**Acceptance Criteria:**
- [ ] `PaladinExecutionService` queries Sanctum before LLM call
- [ ] Retrieved context injected into system prompt or user message
- [ ] New memories extracted after successful execution
- [ ] Configurable via `PaladinConfig`
- [ ] Metrics for retrieval latency and hit rate

**Definition of Done:**
```rust
impl PaladinExecutionService {
    pub async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        // 1. Retrieve relevant memories from Sanctum (if configured)
        let memories = self.retrieve_memories(paladin, input).await?;

        // 2. Build prompt with memory context
        let enriched_prompt = self.build_prompt_with_context(paladin, input, &memories);

        // 3. Execute LLM call
        let result = self.execute_llm(paladin, &enriched_prompt).await?;

        // 4. Extract and store new memories (if configured)
        self.extract_and_store_memories(paladin, input, &result).await?;

        Ok(result)
    }
}
```

---

### Epic 12 Completion Criteria
- [ ] All 5 user stories completed and tested
- [ ] Qdrant adapter with integration tests
- [ ] Sanctum retrieval integrated into execution flow
- [ ] Memory extraction service functional
- [ ] Configuration via YAML supported
- [ ] Example: `examples/paladin_with_sanctum.rs`
- [ ] Example: `examples/cli_configs/paladin_sanctum.yaml`

---

## Epic 13: Sentinel Vision System

**Theme:** Multi-Modal Input Processing  
**Duration:** 2 weeks  
**Priority:** Critical  
**Dependencies:** Epic 6 (Provider Expansion)  

### Description
Add vision and document processing capabilities to Paladins, enabling image analysis, PDF processing, and multi-modal inputs through supported LLM providers. The Sentinel system grants Paladins the ability to perceive and interpret visual intelligence.

### User Stories

#### US-13.1: Vision Request Model
**As a** framework developer  
**I want** data structures for multi-modal requests  
**So that** the system can handle images and documents

**Acceptance Criteria:**
- [ ] `VisionContent` enum in `src/core/platform/container/vision.rs`
- [ ] Supports: ImageUrl, ImageBase64, ImageFile
- [ ] Supports multiple images in single request
- [ ] Image metadata: format, size, dimensions (if known)
- [ ] Validation for supported formats (PNG, JPEG, GIF, WebP)

**Definition of Done:**
```rust
pub enum VisionContent {
    ImageUrl {
        url: String,
        detail: ImageDetail,
    },
    ImageBase64 {
        data: String,
        media_type: String,
        detail: ImageDetail,
    },
    ImageFile {
        path: PathBuf,
        detail: ImageDetail,
    },
}

pub enum ImageDetail {
    Auto,
    Low,
    High,
}

pub struct VisionRequest {
    pub text: String,
    pub images: Vec<VisionContent>,
}
```

---

#### US-13.2: OpenAI Vision Support
**As a** developer  
**I want** to send images to GPT-4 Vision  
**So that** Paladins can analyze visual content

**Acceptance Criteria:**
- [ ] `OpenAILlmAdapter` supports vision requests
- [ ] Handles `gpt-4-vision-preview` and `gpt-4o` models
- [ ] Converts `VisionContent` to OpenAI message format
- [ ] Supports image URLs and base64 encoding
- [ ] Handles multiple images in single request
- [ ] Respects token limits for image tokens
- [ ] Integration test with mocked vision response

**Definition of Done:**
```rust
impl OpenAILlmAdapter {
    async fn generate_with_vision(
        &self,
        request: LlmRequest,
        vision: VisionRequest,
    ) -> Result<LlmResponse, LlmError>;
}

#[async_trait]
pub trait VisionCapableLlm: LlmPort {
    async fn generate_with_vision(
        &self,
        request: LlmRequest,
        vision: VisionRequest,
    ) -> Result<LlmResponse, LlmError>;

    fn supports_vision(&self) -> bool;
}
```

---

#### US-13.3: Anthropic Vision Support
**As a** developer  
**I want** to send images to Claude  
**So that** I can use Anthropic's vision capabilities

**Acceptance Criteria:**
- [ ] `AnthropicLlmAdapter` supports vision requests
- [ ] Handles Claude 3 models (Opus, Sonnet, Haiku)
- [ ] Converts `VisionContent` to Anthropic message format
- [ ] Supports base64 images (required by Anthropic)
- [ ] Automatic URL-to-base64 conversion
- [ ] Integration test with mocked vision response

**Definition of Done:**
```rust
impl VisionCapableLlm for AnthropicLlmAdapter {
    async fn generate_with_vision(
        &self,
        request: LlmRequest,
        vision: VisionRequest,
    ) -> Result<LlmResponse, LlmError> {
        // Convert to Anthropic format with image content blocks
    }

    fn supports_vision(&self) -> bool {
        self.model.starts_with("claude-3")
    }
}
```

---

#### US-13.4: Paladin Vision API
**As a** developer  
**I want** to run Paladins with image inputs  
**So that** I can build vision-based agents

**Acceptance Criteria:**
- [ ] `Paladin::run_with_vision(task, images)` method
- [ ] `PaladinBuilder::enable_vision(true)` configuration
- [ ] Validation that LLM adapter supports vision
- [ ] CLI support: `--image <path>` flag
- [ ] YAML config: `images: [path1, path2]`

**Definition of Done:**
```rust
impl Paladin {
    pub async fn run_with_vision(
        &self,
        task: &str,
        images: Vec<VisionContent>,
    ) -> Result<PaladinResult, PaladinError>;
}

// CLI usage
// paladin agent run --config agent.yaml --input "Describe this" --image photo.jpg

// YAML config
// task: "Analyze these charts"
// images:
//   - "./chart1.png"
//   - "./chart2.png"
```

---

#### US-13.5: Scroll Extraction (PDF Processing)
**As a** developer  
**I want** to extract text from PDF documents  
**So that** Paladins can process scrolls and manuscripts

**Acceptance Criteria:**
- [ ] `ScrollExtractor` utility in `src/infrastructure/adapters/document/`
- [ ] Uses `pdf-extract` or `lopdf` crate
- [ ] Extracts text content preserving structure
- [ ] Handles multi-page documents
- [ ] Returns `Scroll` struct with pages and metadata
- [ ] Error handling for encrypted/malformed PDFs

**Definition of Done:**
```rust
pub struct ScrollExtractor;

impl ScrollExtractor {
    pub fn extract(path: &Path) -> Result<Scroll, ScrollError>;
    pub fn extract_bytes(bytes: &[u8]) -> Result<Scroll, ScrollError>;
}

pub struct Scroll {
    pub pages: Vec<Page>,
    pub metadata: ScrollMetadata,
    pub total_chars: usize,
}

pub struct Page {
    pub number: usize,
    pub content: String,
}
```

---

#### US-13.6: Scroll Port Definition
**As a** developer  
**I want** a standardized port for document processing  
**So that** I can support multiple document types

**Acceptance Criteria:**
- [ ] `ScrollPort` trait in `src/application/ports/input/scroll_port.rs`
- [ ] Supports: PDF, TXT, MD, DOCX (future)
- [ ] Returns chunked content for large documents
- [ ] Configurable chunk size and overlap
- [ ] Metadata extraction (title, author, date)

**Definition of Done:**
```rust
#[async_trait]
pub trait ScrollPort: Send + Sync {
    async fn ingest(&self, source: ScrollSource) -> Result<Scroll, ScrollError>;
    async fn chunk(&self, scroll: &Scroll, config: ChunkConfig) -> Vec<ScrollChunk>;
}

pub enum ScrollSource {
    File(PathBuf),
    Bytes { data: Vec<u8>, format: ScrollFormat },
    Url(String),
}

pub struct ChunkConfig {
    pub chunk_size: usize,
    pub chunk_overlap: usize,
    pub separator: String,
}
```

---

### Epic 13 Completion Criteria
- [ ] All 6 user stories completed and tested
- [ ] OpenAI and Anthropic vision support
- [ ] Scroll extraction functional
- [ ] Document ingestion port defined
- [ ] CLI supports `--image` and `--scroll` flags
- [ ] Documentation in `docs/SENTINEL.md`
- [ ] Example: `examples/vision_analysis.rs`
- [ ] Example: `examples/scroll_processing.rs`

---

## Epic 14: Autonomous Agent Features

**Theme:** Agent Self-Direction and Planning  
**Duration:** 2 weeks  
**Priority:** Critical  
**Dependencies:** Epic 1 (Paladin Domain)  

### Description
Implement advanced agent autonomy features including automatic planning mode, dynamic temperature adjustment, auto-generated oaths (system prompts), and agent delegation for dynamic task routing. These features enable Paladins to operate with greater independence and strategic thinking.

### User Stories

#### US-14.1: Strategic Planning Mode
**As a** developer  
**I want** Paladins to automatically plan and execute subtasks  
**So that** complex missions are handled without manual decomposition

**Acceptance Criteria:**
- [ ] `MaxLoops::Auto` enum variant supported
- [ ] `StrategistService` decomposes tasks into objectives
- [ ] Objectives executed sequentially with dependency tracking
- [ ] Planning uses dedicated planning prompt
- [ ] Final summary synthesizes all objective results
- [ ] Configurable max objectives limit

**Definition of Done:**
```rust
pub enum MaxLoops {
    Fixed(u32),
    Auto { max_objectives: u32 },
}

pub struct StrategistService {
    llm: Arc<dyn LlmPort>,
}

impl StrategistService {
    pub async fn create_battle_plan(&self, mission: &str) -> Result<BattlePlan, StrategyError>;
    pub async fn execute_plan(&self, plan: BattlePlan, paladin: &Paladin) -> Result<PlanResult, StrategyError>;
}

pub struct BattlePlan {
    pub original_mission: String,
    pub objectives: Vec<Objective>,
    pub dependencies: HashMap<usize, Vec<usize>>,
}

pub struct Objective {
    pub id: usize,
    pub description: String,
    pub expected_outcome: String,
}
```

---

#### US-14.2: Oath Generation (Auto System Prompt)
**As a** developer  
**I want** automatic system prompt generation  
**So that** I can create Paladins without manual prompt engineering

**Acceptance Criteria:**
- [ ] `PaladinBuilder::auto_generate_oath(true)` flag
- [ ] Uses agent name and description to generate prompt
- [ ] `OathForgeService` creates contextual prompts
- [ ] Generated oaths include role, capabilities, constraints
- [ ] Caching of generated oaths for reuse

**Definition of Done:**
```rust
impl PaladinBuilder {
    pub fn auto_generate_oath(mut self, enabled: bool) -> Self;
    pub fn agent_description(mut self, desc: impl Into<String>) -> Self;
}

pub struct OathForgeService {
    llm: Arc<dyn LlmPort>,
}

impl OathForgeService {
    pub async fn forge_oath(
        &self,
        paladin_name: &str,
        paladin_description: &str,
        mission_context: Option<&str>,
    ) -> Result<String, OathError>;
}

// Example generated oath (system prompt):
// "You are {paladin_name}, a noble Paladin sworn to {description}.
//  Your sacred duty is to {capabilities}. You shall {constraints}."
```

---

#### US-14.3: Adaptive Temperament (Dynamic Temperature)
**As a** developer  
**I want** automatic temperature adjustment based on task  
**So that** responses are appropriately creative or precise

**Acceptance Criteria:**
- [ ] `PaladinBuilder::adaptive_temperament(true)` flag
- [ ] `TemperamentService` analyzes task complexity
- [ ] Lower temperature for: factual queries, code, calculations
- [ ] Higher temperature for: creative writing, brainstorming
- [ ] Temperature bounds: 0.1 - 1.0
- [ ] Logging of temperature decisions

**Definition of Done:**
```rust
impl PaladinBuilder {
    pub fn adaptive_temperament(mut self, enabled: bool) -> Self;
    pub fn temperament_bounds(mut self, min: f32, max: f32) -> Self;
}

pub struct TemperamentService;

impl TemperamentService {
    pub fn analyze_mission(&self, mission: &str) -> MissionType;
    pub fn recommend_temperature(&self, mission_type: MissionType) -> f32;
}

pub enum MissionType {
    Reconnaissance,   // Factual: 0.1 - 0.3
    Analysis,         // Analytical: 0.3 - 0.5
    Diplomacy,        // Conversational: 0.5 - 0.7
    Creative,         // Creative: 0.7 - 1.0
}
```

---

#### US-14.4: Squire Delegation Infrastructure
**As a** developer  
**I want** to configure Paladins that can delegate to specialists  
**So that** complex missions route to appropriate experts

**Acceptance Criteria:**
- [ ] `PaladinBuilder::with_squires(paladins)` method
- [ ] `DelegationService` routes tasks to squires
- [ ] Delegation decision based on mission analysis
- [ ] Delegation includes context transfer
- [ ] Maximum delegation depth to prevent loops
- [ ] Delegation history tracked in result

**Definition of Done:**
```rust
impl PaladinBuilder {
    pub fn with_squires(mut self, paladins: Vec<Arc<Paladin>>) -> Self;
    pub fn delegation_strategy(mut self, strategy: DelegationStrategy) -> Self;
}

pub enum DelegationStrategy {
    Automatic,  // LLM decides when to delegate
    Explicit,   // Only delegate when tool called
    Threshold { confidence: f32 }, // Delegate when confidence low
}

pub struct DelegationService {
    llm: Arc<dyn LlmPort>,
}

impl DelegationService {
    pub async fn should_delegate(
        &self,
        mission: &str,
        current_paladin: &Paladin,
        available_squires: &[Arc<Paladin>],
    ) -> Option<DelegationOrder>;

    pub async fn execute_delegation(
        &self,
        order: DelegationOrder,
        context: DelegationContext,
    ) -> Result<PaladinResult, DelegationError>;
}

pub struct DelegationOrder {
    pub target_squire: Arc<Paladin>,
    pub reason: String,
    pub context_to_transfer: String,
}
```

---

#### US-14.5: Delegation Armament
**As a** developer  
**I want** Paladins to use a delegation tool  
**So that** task transfers can happen during execution

**Acceptance Criteria:**
- [ ] `delegate_to_squire` tool registered automatically when squires configured
- [ ] Tool schema includes target squire and orders
- [ ] Tool execution triggers delegation service
- [ ] Result returned to original Paladin for synthesis
- [ ] Delegation chain visible in execution trace

**Definition of Done:**
```rust
pub struct DelegationArmament {
    available_squires: Vec<SquireInfo>,
    delegation_service: Arc<DelegationService>,
}

impl DelegationArmament {
    pub fn schema(&self) -> Armament {
        Armament {
            name: "delegate_to_squire".to_string(),
            description: "Delegate this mission to a specialized squire".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "squire_name": {
                        "type": "string",
                        "enum": self.available_squires.iter().map(|s| &s.name).collect::<Vec<_>>()
                    },
                    "orders": {
                        "type": "string",
                        "description": "Context and mission for the target squire"
                    }
                },
                "required": ["squire_name", "orders"]
            }),
            required_params: vec!["squire_name".to_string(), "orders".to_string()],
        }
    }
}
```

---

### Epic 14 Completion Criteria
- [ ] All 5 user stories completed and tested
- [ ] `max_loops="auto"` strategic planning mode functional
- [ ] Oath generation (auto system prompt) working
- [ ] Adaptive temperament working
- [ ] Squire delegation infrastructure complete
- [ ] Documentation in `docs/AUTONOMOUS.md`
- [ ] Example: `examples/strategic_planning.rs`
- [ ] Example: `examples/squire_delegation.rs`

---

## Epic 15: Conclave Expert Synthesis

**Theme:** Expert Panel Orchestration  
**Duration:** 2 weeks  
**Priority:** High  
**Dependencies:** Epic 4 (Battalion Orchestration)  

### Description
Implement the Conclave orchestration pattern where multiple expert Paladins process a mission in parallel, and a Loremaster synthesizes their counsel into a final, comprehensive response. The Conclave represents a gathering of specialized knights contributing their expertise to inform a unified decision.

### User Stories

#### US-15.1: Conclave Domain Model
**As a** framework developer  
**I want** domain models for expert synthesis orchestration  
**So that** the pattern has clear structure

**Acceptance Criteria:**
- [ ] `Conclave` struct in `src/core/platform/container/battalion/conclave.rs`
- [ ] Contains: expert Paladins, Loremaster Paladin, configuration
- [ ] `ConclaveConfig` with synthesis settings
- [ ] `ConclaveResult` with individual and synthesized outputs
- [ ] Validation: at least 2 experts, 1 Loremaster

**Definition of Done:**
```rust
pub struct Conclave {
    pub name: String,
    pub experts: Vec<Paladin>,
    pub loremaster: Paladin,
    pub config: ConclaveConfig,
}

pub struct ConclaveConfig {
    pub name: String,
    pub timeout_seconds: u64,
    pub synthesis_prompt: Option<String>,
    pub include_expert_names: bool,
    pub max_expert_output_tokens: Option<usize>,
}

pub struct ConclaveResult {
    pub expert_outputs: HashMap<String, PaladinResult>,
    pub synthesized_output: PaladinResult,
    pub execution_time_ms: u64,
    pub status: ConclaveStatus,
}
```

---

#### US-15.2: Conclave Execution Service
**As a** developer  
**I want** to execute expert synthesis workflows  
**So that** I get unified expert counsel

**Acceptance Criteria:**
- [ ] `ConclaveExecutionService` in `src/application/use_cases/battalion/`
- [ ] Executes all experts in parallel
- [ ] Collects outputs with error handling
- [ ] Formats outputs for Loremaster
- [ ] Executes Loremaster with expert context
- [ ] Returns combined result

**Definition of Done:**
```rust
pub struct ConclaveExecutionService {
    paladin_port: Arc<dyn PaladinPort>,
}

impl ConclaveExecutionService {
    pub async fn convene(
        &self,
        conclave: &Conclave,
        mission: &str,
    ) -> Result<ConclaveResult, BattalionError>;

    fn format_expert_counsel_for_loremaster(
        &self,
        outputs: &HashMap<String, PaladinResult>,
        config: &ConclaveConfig,
    ) -> String;
}

// Loremaster receives prompt like:
// "You are the Loremaster synthesizing counsel from expert Paladins.
//  Sir Strategist reports: {output1}
//  Sir Tactician reports: {output2}
//  Sir Scout reports: {output3}
//  
//  Synthesize their wisdom into a comprehensive response."
```

---

#### US-15.3: Commander Conclave Strategy
**As a** developer  
**I want** Commander to support Conclave strategy  
**So that** I can use unified orchestration API

**Acceptance Criteria:**
- [ ] `BattalionStrategy::Conclave` variant added
- [ ] Commander builds Conclave from configuration
- [ ] Designates last Paladin as Loremaster by default
- [ ] Configurable Loremaster selection
- [ ] Auto-strategy considers Conclave for expert scenarios

**Definition of Done:**
```rust
pub enum BattalionStrategy {
    Formation,
    Phalanx,
    Campaign,
    ChainOfCommand,
    Conclave,  // NEW
    Auto,
}

impl CommanderBuilder {
    pub fn loremaster(mut self, paladin: Paladin) -> Self;
}

// Auto-strategy heuristics for Conclave:
// - Multiple Paladins with distinct descriptions
// - Mission requires synthesis/comparison
// - Keywords: "compare", "synthesize", "combine perspectives"
```

---

#### US-15.4: Conclave CLI Support
**As a** developer  
**I want** to run Conclave from CLI  
**So that** I can use expert synthesis without code

**Acceptance Criteria:**
- [ ] `paladin battalion run --type conclave --config conclave.yaml`
- [ ] YAML schema for Conclave configuration
- [ ] Loremaster specified in YAML
- [ ] Output includes expert outputs and synthesis
- [ ] Template generation: `paladin battalion new --type conclave`

**Definition of Done:**
```yaml
# conclave.yaml
type: conclave
name: "WarCouncil"

loremaster:
  inline:
    name: "Loremaster"
    system_prompt: "Synthesize expert counsel into strategic wisdom"
    model: "gpt-4"

experts:
  - inline:
      name: "SirStrategist"
      system_prompt: "Provide strategic military analysis"
      model: "gpt-4"

  - inline:
      name: "SirQuartermaster"  
      system_prompt: "Analyze logistics and resource considerations"
      model: "gpt-4"

  - inline:
      name: "SirVigilant"
      system_prompt: "Identify risks and potential threats"
      model: "gpt-4"

config:
  timeout_seconds: 300
  include_expert_names: true
```

*Historical example name: "Quartermaster" as a budgeting term was retired and replaced by
Commissary — see `.planning/decisions/0049-commissary-design-and-rename.md`.*

---

### Epic 15 Completion Criteria
- [ ] All 4 user stories completed and tested
- [ ] Conclave domain model and execution service
- [ ] Commander integration with Conclave strategy
- [ ] CLI and YAML support
- [ ] Documentation in `docs/guides/conclave-pattern.md`
- [ ] Example: `examples/conclave_war_council.rs`
- [ ] Example: `examples/cli_configs/conclave.yaml`

---

## Epic 16: Advanced Battalion Patterns

**Theme:** Council and Grove Orchestration  
**Duration:** 2 weeks  
**Priority:** Medium  
**Dependencies:** Epic 4, Epic 15  

### Description
Implement additional orchestration patterns including Council for conversational collaboration (where Paladins deliberate like a round table) and Grove for tree-based agent routing (organizing Paladins into specialized guilds).

### User Stories

#### US-16.1: Council Domain Model (Round Table)
**As a** framework developer  
**I want** domain models for conversational multi-agent collaboration  
**So that** Paladins can deliberate and reach consensus

**Acceptance Criteria:**
- [ ] `Council` struct in `src/core/platform/container/battalion/council.rs`
- [ ] Contains: participant Paladins, Herald (moderator, optional), configuration
- [ ] `CouncilConfig` with turn settings, max rounds
- [ ] `CouncilMessage` for conversation tracking
- [ ] Turn-taking strategies: RoundRobin, Random, HeraldDirected

**Definition of Done:**
```rust
pub struct Council {
    pub name: String,
    pub knights: Vec<Paladin>,
    pub herald: Option<Paladin>,
    pub config: CouncilConfig,
}

pub struct CouncilConfig {
    pub max_rounds: u32,
    pub turn_strategy: TurnStrategy,
    pub termination_condition: TerminationCondition,
    pub include_history: bool,
}

pub enum TurnStrategy {
    RoundRobin,
    Random,
    HeraldDirected,
    VoluntaryWithTimeout { timeout_ms: u64 },
}

pub enum TerminationCondition {
    MaxRounds,
    Consensus,
    HeraldDecision,
    Keyword(String),
}

pub struct CouncilMessage {
    pub speaker: String,
    pub content: String,
    pub round: u32,
    pub timestamp: DateTime<Utc>,
}
```

---

#### US-16.2: Council Execution Service
**As a** developer  
**I want** to execute round table deliberations  
**So that** Paladins can collaboratively solve problems

**Acceptance Criteria:**
- [ ] `CouncilExecutionService` manages deliberation flow
- [ ] Tracks conversation history
- [ ] Implements turn-taking logic
- [ ] Detects termination conditions
- [ ] Returns full transcript and conclusion

**Definition of Done:**
```rust
pub struct CouncilExecutionService {
    paladin_port: Arc<dyn PaladinPort>,
}

impl CouncilExecutionService {
    pub async fn convene(
        &self,
        council: &Council,
        topic: &str,
    ) -> Result<CouncilResult, BattalionError>;
}

pub struct CouncilResult {
    pub transcript: Vec<CouncilMessage>,
    pub conclusion: Option<String>,
    pub rounds_completed: u32,
    pub termination_reason: TerminationCondition,
}
```

---

#### US-16.3: Grove Domain Model (Guild Forest)
**As a** framework developer  
**I want** domain models for tree-based agent routing  
**So that** missions route to best-fit experts

**Acceptance Criteria:**
- [ ] `Grove` struct in `src/core/platform/container/battalion/grove.rs`
- [ ] `Guild` struct containing related specialists
- [ ] Paladins have expertise keywords/embeddings
- [ ] `GroveConfig` with routing settings
- [ ] Routing based on semantic similarity to mission

**Definition of Done:**
```rust
pub struct Grove {
    pub name: String,
    pub guilds: Vec<Guild>,
    pub config: GroveConfig,
}

pub struct Guild {
    pub name: String,
    pub members: Vec<GuildMember>,
}

pub struct GuildMember {
    pub paladin: Paladin,
    pub expertise_keywords: Vec<String>,
    pub expertise_embedding: Option<Vec<f32>>,
}

pub struct GroveConfig {
    pub routing_strategy: RoutingStrategy,
    pub fallback_guild: Option<String>,
    pub similarity_threshold: f32,
}

pub enum RoutingStrategy {
    KeywordMatch,
    SemanticSimilarity,
    LlmRouting,
}
```

---

#### US-16.4: Grove Execution Service
**As a** developer  
**I want** automatic routing to best-fit guild members  
**So that** missions are handled by appropriate specialists

**Acceptance Criteria:**
- [ ] `GroveExecutionService` routes missions
- [ ] Calculates mission-to-member similarity
- [ ] Selects best guild, then best member
- [ ] Falls back if no good match
- [ ] Returns routing decision and result

**Definition of Done:**
```rust
pub struct GroveExecutionService {
    paladin_port: Arc<dyn PaladinPort>,
    embedding_port: Option<Arc<dyn EmbeddingPort>>,
}

impl GroveExecutionService {
    pub async fn dispatch(
        &self,
        grove: &Grove,
        mission: &str,
    ) -> Result<GroveResult, BattalionError>;

    async fn route_mission(
        &self,
        grove: &Grove,
        mission: &str,
    ) -> Result<RoutingDecision, BattalionError>;
}

pub struct RoutingDecision {
    pub selected_guild: String,
    pub selected_member: String,
    pub confidence: f32,
    pub reasoning: String,
}
```

---

#### US-16.5: Commander Integration
**As a** developer  
**I want** Commander to support Council and Grove  
**So that** I have unified orchestration

**Acceptance Criteria:**
- [ ] `BattalionStrategy::Council` variant
- [ ] `BattalionStrategy::Grove` variant
- [ ] Auto-strategy considers these patterns
- [ ] CLI support for both types

**Definition of Done:**
```rust
pub enum BattalionStrategy {
    Formation,
    Phalanx,
    Campaign,
    ChainOfCommand,
    Conclave,
    Council,  // NEW
    Grove,    // NEW
    Auto,
}
```

---

### Epic 16 Completion Criteria
- [ ] All 5 user stories completed and tested
- [ ] Council (Round Table) fully functional
- [ ] Grove (Guild Forest) fully functional
- [ ] Commander integration complete
- [ ] Documentation for both patterns
- [ ] Example: `examples/council_deliberation.rs`
- [ ] Example: `examples/grove_routing.rs`

---

## Epic 17: Tactical Flow DSL

**Theme:** Flexible Workflow Definition  
**Duration:** 2 weeks  
**Priority:** Medium  
**Dependencies:** Epic 4, Epic 15, Epic 16  

### Description
Implement the Maneuver pattern with a simple string-based DSL for defining complex Paladin relationships, enabling flexible workflow definition without verbose configuration. The tactical language allows commanders to quickly describe battle formations.

### User Stories

#### US-17.1: Tactical Flow Parser
**As a** developer  
**I want** to define workflows with simple string syntax  
**So that** I can quickly express Paladin relationships

**Acceptance Criteria:**
- [ ] Parser for flow expressions like `"scout -> archer, cavalry"` and `"scout -> infantry -> siege"`
- [ ] Supports: sequential (`->`), parallel (`,`), groups (`()`)
- [ ] `TacticalExpression` AST representation
- [ ] Validation of Paladin names against registered units
- [ ] Clear error messages for invalid syntax

**Definition of Done:**
```rust
// Supported syntax:
// "a -> b"           Sequential: a then b
// "a -> b, c"        Fan-out: a then b and c in parallel
// "a, b -> c"        Fan-in: a and b in parallel, then c
// "a -> (b -> c), d" Nested: a then (b->c) and d in parallel
// "a -> b -> c"      Chain: a then b then c

pub struct TacticalParser;

impl TacticalParser {
    pub fn parse(expression: &str) -> Result<TacticalExpression, TacticalParseError>;
}

pub enum TacticalExpression {
    Unit(String),
    Sequential(Vec<TacticalExpression>),
    Parallel(Vec<TacticalExpression>),
}
```

---

#### US-17.2: Maneuver Domain Model
**As a** framework developer  
**I want** domain models for flow-based orchestration  
**So that** the pattern has clear structure

**Acceptance Criteria:**
- [ ] `Maneuver` struct in `src/core/platform/container/battalion/maneuver.rs`
- [ ] Contains: units map, tactical expression, configuration
- [ ] Validates flow references valid units
- [ ] `ManeuverConfig` with execution settings

**Definition of Done:**
```rust
pub struct Maneuver {
    pub name: String,
    pub units: HashMap<String, Paladin>,
    pub tactics: TacticalExpression,
    pub config: ManeuverConfig,
}

pub struct ManeuverConfig {
    pub timeout_seconds: u64,
    pub error_strategy: ErrorStrategy,
    pub pass_output_as_input: bool,
}

impl Maneuver {
    pub fn new(
        name: impl Into<String>,
        units: Vec<Paladin>,
        tactics: &str,
    ) -> Result<Self, ManeuverError>;
}
```

---

#### US-17.3: Maneuver Execution Service
**As a** developer  
**I want** to execute tactical workflows  
**So that** complex patterns run correctly

**Acceptance Criteria:**
- [ ] `ManeuverExecutionService` interprets tactical expressions
- [ ] Executes sequential steps in order
- [ ] Executes parallel steps concurrently
- [ ] Handles nested expressions recursively
- [ ] Passes outputs between steps as configured

**Definition of Done:**
```rust
pub struct ManeuverExecutionService {
    paladin_port: Arc<dyn PaladinPort>,
}

impl ManeuverExecutionService {
    pub async fn execute(
        &self,
        maneuver: &Maneuver,
        orders: &str,
    ) -> Result<ManeuverResult, BattalionError>;

    async fn execute_expression(
        &self,
        expr: &TacticalExpression,
        units: &HashMap<String, Paladin>,
        input: &str,
    ) -> Result<StepResult, BattalionError>;
}

pub struct ManeuverResult {
    pub final_output: String,
    pub step_outputs: HashMap<String, PaladinResult>,
    pub execution_order: Vec<String>,
}
```

---

#### US-17.4: Commander Maneuver Strategy
**As a** developer  
**I want** Commander to support tactical workflows  
**So that** I can use the unified API

**Acceptance Criteria:**
- [ ] `BattalionStrategy::Maneuver` variant
- [ ] CommanderBuilder accepts tactical expression
- [ ] Auto-strategy does NOT select Maneuver (explicit only)
- [ ] CLI support with tactical syntax

**Definition of Done:**
```rust
impl CommanderBuilder {
    pub fn tactics(mut self, expression: &str) -> Self;
}

// CLI usage:
// paladin battalion run --type maneuver --tactics "scout -> archer, cavalry -> siege"

// YAML config:
// type: maneuver
// tactics: "scout -> archer, cavalry -> siege"
// units:
//   - name: scout
//     ...
```

---

#### US-17.5: Tactical Visualization
**As a** developer  
**I want** to visualize tactical expressions  
**So that** I can understand and debug battle plans

**Acceptance Criteria:**
- [ ] `TacticalVisualizer` generates ASCII/Mermaid diagrams
- [ ] CLI command: `paladin battalion visualize --tactics "..."`
- [ ] Shows execution order and parallelism
- [ ] Useful for documentation

**Definition of Done:**
```rust
pub struct TacticalVisualizer;

impl TacticalVisualizer {
    pub fn to_ascii(expr: &TacticalExpression) -> String;
    pub fn to_mermaid(expr: &TacticalExpression) -> String;
}

// Example ASCII output for "scout -> archer, cavalry -> siege":
// ┌───────┐
// │ scout │
// └───┬───┘
//     │
// ┌───┴───┬─────────┐
// │       │         │
// ▼       ▼         │
// ┌────────┐ ┌─────────┐
// │ archer │ │ cavalry │
// └───┬────┘ └────┬────┘
//     │           │
//     └─────┬─────┘
//           │
//           ▼
//       ┌───────┐
//       │ siege │
//       └───────┘
```

---

### Epic 17 Completion Criteria
- [ ] All 5 user stories completed and tested
- [ ] Tactical Flow DSL parser complete with tests
- [ ] Maneuver execution service functional
- [ ] Commander integration
- [ ] Visualization working
- [ ] Documentation in `docs/guides/tactical-dsl.md`
- [ ] Example: `examples/maneuver_workflow.rs`

---

## Epic 18: Armory CLI Enhancement

**Theme:** Developer Experience  
**Duration:** 2 weeks  
**Priority:** Medium  
**Dependencies:** Epics 11-17  

### Description
Enhance the Armory CLI with onboarding wizards, setup verification, feature discovery, and advanced commands for common multi-agent patterns. The Armory becomes the complete command center for Paladin operations.

### User Stories

#### US-18.1: Squire Onboarding Wizard
**As a** new developer  
**I want** an interactive setup experience  
**So that** I can get started quickly

**Acceptance Criteria:**
- [ ] `paladin onboarding` command
- [ ] Guides through API key configuration
- [ ] Creates initial `.env` file
- [ ] Offers to create sample config files
- [ ] Validates provider connectivity
- [ ] Colorful, friendly CLI output

**Definition of Done:**
```bash
$ paladin onboarding

🎖️  Welcome to Paladin!

Let's prepare your command post...

? Which LLM provider will you swear allegiance to?
  ❯ OpenAI
    Anthropic
    DeepSeek

? Enter your OpenAI API key: sk-...

✓ Credentials verified successfully!

? Create sample battle configurations?
  ❯ Yes, deploy examples
    No, I'll forge my own

✓ Deployed:
  - examples/basic_paladin.yaml
  - examples/formation.yaml
  - .env

🚀 Your forces are ready! Try: paladin agent run --config examples/basic_paladin.yaml
```

---

#### US-18.2: Garrison Check Command
**As a** developer  
**I want** to verify my environment  
**So that** I can troubleshoot issues

**Acceptance Criteria:**
- [ ] `paladin garrison-check` command
- [ ] Validates API keys for configured providers
- [ ] Checks optional dependencies (Redis, Qdrant, etc.)
- [ ] Reports versions of key components
- [ ] `--verbose` for detailed output
- [ ] Exit codes for CI/CD usage

**Definition of Done:**
```bash
$ paladin garrison-check

🔍 Inspecting Paladin garrison...

Core Components:
  ✓ Paladin CLI v0.1.0
  ✓ Rust 1.75.0

LLM Providers:
  ✓ OpenAI: Credentials valid (gpt-4 accessible)
  ✓ Anthropic: Credentials valid (claude-3 accessible)
  ✗ DeepSeek: Credentials not configured

Support Services:
  ⚠ Redis: Not running (Citadel caching unavailable)
  ✓ Qdrant: Running at localhost:6333 (Sanctum ready)

Summary: 4/5 checks passed
```

---

#### US-18.3: Arsenal Discovery Command
**As a** developer  
**I want** to see all available features  
**So that** I can discover capabilities

**Acceptance Criteria:**
- [ ] `paladin arsenal` command
- [ ] Lists all commands with descriptions
- [ ] Groups by category
- [ ] Shows feature flags and optional features
- [ ] Links to documentation

**Definition of Done:**
```bash
$ paladin arsenal

🎖️  Paladin Arsenal

Agent Commands:
  paladin agent new      Forge new Paladin configuration
  paladin agent run      Deploy a single Paladin
  paladin agent validate Inspect Paladin configuration

Battalion Commands:
  paladin battalion new       Create battalion template
  paladin battalion run       Execute multi-agent campaign
  paladin battalion visualize Display tactical diagram

Orchestration Patterns:
  ✓ Formation      Sequential deployment
  ✓ Phalanx        Parallel assault
  ✓ Campaign       Strategic DAG execution
  ✓ ChainOfCommand Hierarchical command
  ✓ Conclave       Expert synthesis council
  ✓ Council        Round table deliberation
  ✓ Grove          Guild-based routing
  ✓ Maneuver       Tactical flow DSL

Memory Systems:
  ✓ Garrison (Short-term) In-memory, SQLite
  ✓ Sanctum (Long-term)   Qdrant, In-memory

📚 Field Manual: https://docs.paladin.dev
```

---

#### US-18.4: AutoBattalion Command
**As a** developer  
**I want** automatic battalion generation  
**So that** I can quickly prototype workflows

**Acceptance Criteria:**
- [ ] `paladin auto-battalion --mission "..."` command
- [ ] Uses LLM to analyze mission and suggest battalion
- [ ] Generates appropriate configuration
- [ ] Optionally executes immediately
- [ ] Saves config for future use

**Definition of Done:**
```bash
$ paladin auto-battalion --mission "Research and write a report on market trends"

🤖 Analyzing mission parameters...

Recommended Pattern: Formation (Sequential)
Suggested Paladins:
  1. Scout - Gather intelligence on market trends
  2. Analyst - Evaluate and categorize findings
  3. Scribe - Compose final report

? Execute this battalion now? [Y/n]

Battle plan saved to: auto_battalion_20260129.yaml
```

---

#### US-18.5: Convene Council Command
**As a** developer  
**I want** quick access to council deliberations  
**So that** I can run collaborative sessions

**Acceptance Criteria:**
- [ ] `paladin convene --topic "..." --knights 3`
- [ ] Quick setup for round table discussions
- [ ] Configurable number of participants
- [ ] Default participant roles if not specified
- [ ] Interactive output of deliberation

**Definition of Done:**
```bash
$ paladin convene \
    --topic "Should we pursue microservices or monolith architecture?" \
    --knights 3 \
    --max-rounds 5

🏰  Council Convened: Architecture Decision

Knights Present:
  - Sir Advocate (Pro-Microservices)
  - Sir Skeptic (Pro-Monolith)  
  - Lord Herald (Moderator)

Round 1:
  Sir Advocate: "Microservices offer superior scalability..."
  Sir Skeptic: "However, the operational complexity..."
  Lord Herald: "Both present valid arguments. Let us explore..."

[... deliberation continues ...]

Council Verdict: The council recommends beginning with a modular monolith...
```

---

#### US-18.6: Rich CLI Output (Banner & Heraldry)
**As a** developer  
**I want** beautiful terminal output  
**So that** the CLI is pleasant to use

**Acceptance Criteria:**
- [ ] Progress indicators for long operations
- [ ] Colored output (respects NO_COLOR)
- [ ] Spinners during API calls
- [ ] Tables for structured data
- [ ] Box drawing for emphasis

**Definition of Done:**
```rust
// Using indicatif for progress
// Using console/colored for colors
// Using comfy-table for tables

// Example execution output:
// ⚔️  Deploying Paladin: Scout...
// ✓ Scout completed mission (1.2s, 450 tokens)
// ⚔️  Deploying Paladin: Scribe...
// ✓ Scribe completed mission (2.1s, 890 tokens)
//
// ╔══════════════════════════════════════╗
// ║ Battalion Mission Summary            ║
// ╠══════════════════════════════════════╣
// ║ Total Time: 3.3s                     ║
// ║ Total Tokens: 1,340                  ║
// ║ Status: Victory                      ║
// ╚══════════════════════════════════════╝
```

---

### Epic 18 Completion Criteria
- [ ] All 6 user stories completed and tested
- [ ] Onboarding wizard functional
- [ ] Garrison check comprehensive
- [ ] Arsenal discovery complete
- [ ] AutoBattalion generates valid configs
- [ ] Convene command working
- [ ] Rich CLI output throughout
- [ ] Updated CLI documentation

---

## Summary: Complete Epic List

| Epic | Name | Duration | Priority | Status |
|------|------|----------|----------|--------|
| 1-10 | (Existing Epics) | - | - | Complete |
| **11** | **Sanctum Memory Foundation** | 2 weeks | Critical | New |
| **12** | **Sanctum RAG Integration** | 2 weeks | Critical | New |
| **13** | **Sentinel Vision System** | 2 weeks | Critical | New |
| **14** | **Autonomous Agent Features** | 2 weeks | Critical | New |
| **15** | **Conclave Expert Synthesis** | 2 weeks | High | New |
| **16** | **Advanced Battalion Patterns** | 2 weeks | Medium | New |
| **17** | **Tactical Flow DSL** | 2 weeks | Medium | New |
| **18** | **Armory CLI Enhancement** | 2 weeks | Medium | New |

---

## Dependencies Graph

```
Epic 11 (Sanctum Foundation)
    │
    ▼
Epic 12 (Sanctum RAG) ───────────────────┐
    │                                     │
    ▼                                     ▼
Epic 13 (Sentinel Vision)    Epic 14 (Autonomous Features)
    │                                     │
    ▼                                     ▼
Epic 15 (Conclave) ◄─────────────────────┘
    │
    ├──────────────────┐
    ▼                  ▼
Epic 16 (Council/Grove)  Epic 17 (Tactical DSL)
    │                          │
    └──────────┬───────────────┘
               ▼
        Epic 18 (Armory CLI)
```

---

## Risk Assessment

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Qdrant integration complexity | Medium | Medium | Start with in-memory, add Qdrant incrementally |
| Vision API rate limits | Low | Medium | Implement proper backoff, mock for tests |
| Tactical DSL parsing edge cases | Medium | Medium | Extensive test suite, simple syntax first |
| Council termination detection | Low | High | Conservative defaults, explicit termination |
| Performance with large Sanctum | High | Medium | Pagination, caching, vector index optimization |

---

## Success Metrics

### Feature Completeness
- [ ] All orchestration patterns implemented
- [ ] Sanctum memory functional with at least one vector store
- [ ] Sentinel vision support for OpenAI and Anthropic
- [ ] Squire delegation working
- [ ] Armory CLI feature complete

### Quality Metrics
- [ ] Unit test coverage ≥ 80%
- [ ] Integration test coverage ≥ 70%
- [ ] All examples compile and run
- [ ] Documentation complete for all new features

### Performance Metrics
- [ ] Sanctum search < 100ms for 100k vectors
- [ ] Council deliberation < 5s per round
- [ ] Grove routing decision < 500ms

---

## Glossary: Paladin Terminology

| Term | Description |
|------|-------------|
| **Paladin** | AI agent entity - an autonomous knight of the realm |
| **Battalion** | Multi-agent orchestration - a coordinated group of Paladins |
| **Formation** | Sequential execution - Paladins advance one after another |
| **Phalanx** | Parallel execution - Paladins charge simultaneously |
| **Campaign** | DAG-based execution - strategic multi-path operations |
| **Chain of Command** | Hierarchical delegation - orders flow from commander to troops |
| **Conclave** | Expert synthesis - council of specialists with a Loremaster |
| **Council** | Conversational collaboration - round table deliberation |
| **Grove** | Tree-based routing - guilds of specialized Paladins |
| **Maneuver** | Tactical flow DSL - flexible battlefield choreography |
| **Commander** | Universal orchestrator - selects and executes strategies |
| **Sanctum** | Vector-based long-term memory - sacred repository of knowledge |
| **Garrison** | Conversation history - short-term memory barracks |
| **Arsenal** | External tool system - collection of weapons and equipment |
| **Armament** | Individual tool - a specific weapon in the Arsenal |
| **Sentinel** | Vision system - eyes and perception capabilities |
| **Scroll** | Document/PDF - written intelligence to be processed |
| **Squire** | Delegate agent - specialist that receives delegated tasks |
| **Loremaster** | Aggregator agent - synthesizes expert knowledge |
| **Herald** | Moderator/announcer - facilitates council or formats output |
| **Guild** | Agent group - collection of related specialists |
| **Citadel** | State persistence - fortress for checkpoint storage |
| **Oath** | System prompt - the sacred vow defining a Paladin's purpose |
| **Armory** | CLI interface - workshop for Paladin management |
| **Objective** | Subtask - individual goal within a battle plan |
| **Battle Plan** | Auto-generated task decomposition |
| **Mission** | Task assigned to a Paladin or Battalion |
