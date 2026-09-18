// examples/structured_output_schema.rs
//
// Structured Output: Schema Derivation and Validation
//
// This example demonstrates the Phase 26 structured-output surface: deriving a
// JSON Schema straight from a Rust type via `schemars`, and executing a Paladin
// through the typed structured-execution path (`StructuredExecutorExt::execute_structured`)
// so the caller gets a typed value back rather than a raw string. It shows:
//
// 1. Deriving a JSON Schema from a local Rust type through `schemars::schema_for!`
//    -- the same deriver the shipped structured-output machinery uses, never a
//    hand-built schema.
// 2. Executing against that schema with a response that conforms, returning a
//    typed value with its own fields printed (not a raw string).
// 3. A response that violates the schema being rejected rather than silently
//    accepted -- the repair loop exhausts and returns a typed
//    `PaladinError::StructuredOutputInvalid`, printed in full.
//
// This example is fully offline: it uses `MockLlmAdapter` and reads no LLM provider
// API key from the environment -- no external service is needed.
//
// To run this example:
// ```bash
// cargo run --example structured_output_schema
// ```

use std::sync::Arc;
use std::time::Duration;

use schemars::JsonSchema;
use serde::Deserialize;

use paladin::MockLlmAdapter;
use paladin::StructuredExecutorExt;
use paladin::application::services::paladin::error::PaladinError;
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin_ports::output::llm_port::LlmPort;

/// A small local result type the structured-execution path derives a schema from
/// (EX-90). `Deserialize` + `JsonSchema` are the two derives the shipped machinery
/// requires (`StructuredExecutorExt::execute_structured`'s own bound).
#[derive(Debug, Deserialize, JsonSchema, PartialEq)]
struct WeatherReport {
    /// The city the report covers.
    city: String,
    /// The temperature, in Celsius.
    temp_c: f32,
    /// A short one-line summary of the conditions.
    conditions: String,
}

/// A deliberately narrow type: `value` must fit in an `i8` (-128..=127).
/// `shape_check`'s partial JSON Schema subset accepts any JSON integer, so an
/// out-of-range value passes the shape check but fails serde deserialization --
/// exactly the "schema-violating response" this example's second part triggers.
#[derive(Debug, Deserialize, JsonSchema, PartialEq)]
struct NarrowByte {
    value: i8,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Structured Output: Schema Derivation and Validation\n");

    // ------------------------------------------------------------------------------
    // Part 1 -- derive a JSON Schema from a Rust type (EX-90).
    // ------------------------------------------------------------------------------
    println!("1. Deriving a JSON Schema from WeatherReport via schemars::schema_for!\n");

    let schema_value: serde_json::Value = schemars::schema_for!(WeatherReport).to_value();
    println!(
        "{}\n",
        serde_json::to_string_pretty(&schema_value).unwrap_or_else(|_| schema_value.to_string())
    );

    // ------------------------------------------------------------------------------
    // Part 2 -- schema-validated structured output: the accepted path (EX-88).
    // ------------------------------------------------------------------------------
    println!("2. Schema-validated structured output -- a conforming response\n");

    let ok_llm = Arc::new(
        MockLlmAdapter::new()
            .with_response(r#"{"city": "Oslo", "temp_c": 4.5, "conditions": "light snow"}"#),
    );
    let ok_service = PaladinExecutionService::new(
        ok_llm.clone(),
        Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(30))),
        None,
        None,
    );
    let ok_paladin = PaladinBuilder::new(ok_llm.clone() as Arc<dyn LlmPort>)
        .system_prompt("Report the weather as structured data")
        .max_loops(3)
        .build()
        .await?;

    let weather = ok_service
        .execute_structured::<WeatherReport>(&ok_paladin, "What is the weather in Oslo?")
        .await?
        .value;
    println!("   Typed value returned: {weather:?}");
    println!(
        "   city={}, temp_c={}, conditions={}\n",
        weather.city, weather.temp_c, weather.conditions
    );

    // ------------------------------------------------------------------------------
    // Part 3 -- schema-validated structured output: the rejected path (EX-88).
    // ------------------------------------------------------------------------------
    println!("3. Schema-validated structured output -- a schema-violating response\n");

    // `200` and `201` both parse as JSON integers (shape_check's "integer" type
    // check passes) but overflow i8 (max 127) -- serde rejects them, shape_check
    // does not catch it. One initial call plus one repair attempt (the default
    // budget), then the repair loop exhausts.
    let bad_llm = Arc::new(MockLlmAdapter::new().with_responses(vec![
        r#"{"value": 200}"#.to_string(),
        r#"{"value": 201}"#.to_string(),
    ]));
    let bad_service = PaladinExecutionService::new(
        bad_llm.clone(),
        Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(30))),
        None,
        None,
    );
    let bad_paladin = PaladinBuilder::new(bad_llm.clone() as Arc<dyn LlmPort>)
        .system_prompt("Report a single signed byte")
        .max_loops(3)
        .build()
        .await?;

    match bad_service
        .execute_structured::<NarrowByte>(&bad_paladin, "give me a byte")
        .await
    {
        Ok(value) => println!("   UNEXPECTED: the out-of-range value was accepted: {value:?}"),
        Err(PaladinError::StructuredOutputInvalid {
            attempts,
            last_error,
            raw_output,
        }) => {
            println!(
                "   Rejected, not silently accepted -- typed PaladinError::StructuredOutputInvalid:"
            );
            println!("     attempts   = {attempts}");
            println!("     last_error = {last_error}");
            println!("     raw_output = {raw_output}");
        }
        Err(other) => println!("   UNEXPECTED error variant: {other:?}"),
    }

    println!("\nDone -- fully offline, no provider API key was read.");
    Ok(())
}
