# Battalion Vision Support

## Overview

All Battalion patterns (Formation, Phalanx, Campaign, Chain of Command) support vision-enabled Paladins **without requiring any modifications**. This document explains how vision capabilities integrate seamlessly with Battalion orchestration.

## Key Principle

**Vision support is implemented at the Paladin execution layer, not the Battalion orchestration layer.**

Battalions orchestrate Paladins regardless of their capabilities:
- They don't need to know if a Paladin has vision enabled
- They don't need special handling for vision content
- They pass inputs and collect outputs the same way for all Paladins

## How It Works

### 1. Paladin Level
- `Paladin.vision_enabled` flag enables vision capabilities
- `PaladinExecutionService.execute_with_vision()` handles vision requests
- Vision content (images, documents) is processed by the LLM provider

### 2. Battalion Level
- Battalions call `PaladinPort.execute(paladin, input)`
- The same interface works for both vision and text-only Paladins
- Input can reference images ("analyze this image") or be purely textual
- Output is always text, which Battalions can route/aggregate

## Pattern-Specific Behaviors

###  Formation: Sequential Vision Processing

**Use Case**: Multi-stage image analysis pipeline

```rust
// Stage 1: Image detection
let detector = PaladinBuilder::new(llm_port)
    .enable_vision(true)
    .system_prompt("Detect objects in the image")
    .build()?;

// Stage 2: Classification
let classifier = PaladinBuilder::new(llm_port)
    .enable_vision(true)
    .system_prompt("Classify the detected objects")
    .build()?;

// Stage 3: Summarization
let summarizer = PaladinBuilder::new(llm_port)
    .system_prompt("Summarize the analysis")
    .build()?;

let formation = Formation::new(
    vec![detector, classifier, summarizer],
    BattalionConfig::new("image_pipeline")
)?;

// Input references the image
let result = formation_service.execute(&formation, "Analyze image.jpg").await?;
```

**Behavior**:
- Detector processes image → outputs text description
- Classifier receives text → may still access image context via shared Garrison
- Summarizer receives text → produces final summary
- Output flows sequentially: detector → classifier → summarizer

### Phalanx: Parallel Vision Processing

**Use Case**: Multi-aspect image analysis (objects, faces, text, colors)

```rust
let object_detector = create_vision_paladin("object_detector");
let face_detector = create_vision_paladin("face_detector");
let text_detector = create_vision_paladin("text_detector");
let color_analyzer = create_vision_paladin("color_analyzer");

let phalanx = Phalanx::new(
    vec![object_detector, face_detector, text_detector, color_analyzer],
    BattalionConfig::new("parallel_analysis")
)?
.with_aggregation(AggregationStrategy::Concatenate);

let result = phalanx_service.execute(&phalanx, "Analyze photo.jpg").await?;
```

**Behavior**:
- All 4 Paladins process the same input simultaneously
- Each analyzes different aspects of the image
- Results are aggregated according to strategy
- Significantly faster than sequential processing

**Batch Processing**:
For processing multiple images, distribute across Paladins:
- Input: "Process images 1-10"
- Phalanx distributes: Paladin 1 → images 1-3, Paladin 2 → images 4-7, etc.
- Parallelism scales with number of Paladins

### Campaign: Vision-Based Conditional Routing

**Use Case**: Conditional workflows based on image content

```rust
let mut campaign = Campaign::new(BattalionConfig::new("smart_routing"));

let analyzer_id = campaign.add_paladin(vision_analyzer);
let cat_specialist_id = campaign.add_paladin(cat_specialist);
let dog_specialist_id = campaign.add_paladin(dog_specialist);
let generic_handler_id = campaign.add_paladin(generic_handler);

// Route based on detection output
campaign.add_edge(CampaignEdge::new(
    analyzer_id,
    cat_specialist_id,
    EdgeCondition::Contains("cat".to_string())
))?;

campaign.add_edge(CampaignEdge::new(
    analyzer_id,
    dog_specialist_id,
    EdgeCondition::Contains("dog".to_string())
))?;

campaign.add_edge(CampaignEdge::new(
    analyzer_id,
    generic_handler_id,
    EdgeCondition::Always
))?;

campaign.set_entry_point(analyzer_id)?;
```

**Behavior**:
- Analyzer processes image → outputs "Detected: cat"
- Campaign evaluates edge conditions on the text output
- Routes to cat_specialist (condition matches)
- Specialist performs deep analysis
- Enables intelligent branching based on image content

**Advanced**: Can combine vision and text conditions:
```rust
EdgeCondition::Custom("has_medical_imagery_and_urgent")
```

### Chain of Command: Vision Task Delegation

**Use Case**: Hierarchical image analysis with specialist delegation

```rust
let commander = create_vision_paladin("chief_analyst");
commander.system_prompt = "Analyze images and delegate to specialists as needed";

let specialists = vec![
    create_vision_paladin("medical_image_specialist"),
    create_vision_paladin("satellite_image_specialist"),
    create_vision_paladin("industrial_qc_specialist"),
];

let chain = ChainOfCommand::new(commander, specialists, config)?
    .with_strategy(DelegationStrategy::Automatic);

let result = chain_service.execute(&chain, "Analyze xray.jpg").await?;
```

**Behavior**:
- Commander analyzes image → determines it's medical
- Automatic delegation selects medical_image_specialist
- Specialist performs detailed analysis
- Commander aggregates results
- Hierarchical decision-making based on image content

**Broadcast Mode**: All specialists analyze simultaneously
```rust
.with_strategy(DelegationStrategy::Broadcast)
```
- Useful for quality assurance (multiple independent analyses)
- Defect detection from multiple perspectives
- Consensus-based classification

## Implementation Status

✅ **Complete**: All Battalion patterns work with vision-enabled Paladins

- ✅ Formation sequential execution
- ✅ Phalanx parallel execution
- ✅ Campaign conditional routing
- ✅ Chain of Command delegation

**No code changes required** - Battalions are capability-agnostic by design.

## Testing Strategy

Battalions test vision support by:

1. **Creating vision-enabled Paladins** using `PaladinBuilder::enable_vision(true)`
2. **Passing vision-referencing inputs** like "Analyze image.jpg"
3. **Verifying correct orchestration** (sequential, parallel, conditional, delegated)
4. **Checking output flows** between Paladins

The actual vision execution (LLM + images) is tested at the Paladin layer with mocked LLM providers.

## Best Practices

### When to Use Each Pattern

| Pattern | Best For | Vision Use Cases |
|---------|----------|------------------|
| **Formation** | Sequential refinement | Multi-stage analysis, quality improvement |
| **Phalanx** | Parallel diversity | Multi-aspect analysis, batch processing |
| **Campaign** | Conditional logic | Content-based routing, adaptive workflows |
| **Chain of Command** | Hierarchical delegation | Specialist selection, quality escalation |

### Performance Considerations

**Formation**:
- Slowest for vision (serial processing)
- Best when each stage needs previous output
- Use when order matters (detect → classify → report)

**Phalanx**:
- Fastest for parallel tasks
- Scales linearly with Paladin count
- Best for independent analyses
- Limit concurrency to avoid API rate limits

**Campaign**:
- Performance depends on graph structure
- Conditional branches save resources
- Fan-out increases parallelism
- Use DAG optimization for complex workflows

**Chain of Command**:
- Automatic delegation adds overhead (commander analysis)
- Broadcast is slower but more thorough
- RoundRobin is fastest for load distribution

### Memory and Context

**Shared Garrison**:
```rust
let garrison = Arc::new(SqliteGarrison::new("shared_memory.db")?);

let paladin = PaladinBuilder::new(llm_port)
    .enable_vision(true)
    .with_garrison(garrison.clone())
    .build()?;
```

- Vision Paladins can store image analysis in Garrison
- Subsequent Paladins (even non-vision) can reference this context
- Enables "vision once, reference many times" pattern

**RAG Integration**:
```rust
let sanctum = Arc::new(QdrantSanctum::new(config)?);
let rag_service = Arc::new(RagRetrievalService::new(sanctum));

let paladin = PaladinBuilder::new(llm_port)
    .enable_vision(true)
    .with_rag_retrieval(rag_service)
    .build()?;
```

- Store image embeddings in Sanctum
- Retrieve relevant images for context
- Combine vision + retrieved knowledge

## Example: Complete Vision Pipeline

```rust
use paladin::application::services::battalion::formation_service::FormationExecutionService;
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::core::platform::container::battalion::formation::Formation;
use paladin::core::platform::container::battalion::BattalionConfig;

async fn vision_pipeline_example() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create vision-enabled Paladins
    let llm_port = Arc::new(OpenAIAdapter::new(openai_config)?);

    let detector = PaladinBuilder::new(llm_port.clone())
        .name("detector")
        .system_prompt("Detect all objects in the image")
        .enable_vision(true)
        .model("gpt-4o")
        .build()?;

    let classifier = PaladinBuilder::new(llm_port.clone())
        .name("classifier")
        .system_prompt("Classify the detected objects")
        .enable_vision(true)
        .model("gpt-4o")
        .build()?;

    let reporter = PaladinBuilder::new(llm_port.clone())
        .name("reporter")
        .system_prompt("Generate a detailed report")
        .build()?; // Text-only

    // 2. Create Formation
    let config = BattalionConfig::new("vision_pipeline")
        .with_timeout(600)
        .with_description("Three-stage image analysis");

    let formation = Formation::new(
        vec![detector, classifier, reporter],
        config
    )?;

    // 3. Execute with image reference
    let service = FormationExecutionService::new(Arc::new(paladin_port));
    let result = service.execute(
        &formation,
        "Analyze the image at ./photos/sample.jpg"
    ).await?;

    println!("Analysis complete: {}", result.final_output);
    Ok(())
}
```

## Conclusion

Battalion vision support is **architectural, not implementational**. The hexagonal design allows Battalions to orchestrate any Paladin capability through a unified interface. Vision, RAG, tool usage, and future capabilities all work seamlessly within existing Battalion patterns.

**Key Takeaway**: If you can build it with a Paladin, you can orchestrate it with a Battalion.
