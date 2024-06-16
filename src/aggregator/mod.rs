// src/aggregator/mod.rs
use serde_json::Value;

pub fn aggregate_data(data: Vec<Value>) -> Value {
    // Simple aggregation logic
    Value::Array(data)
}
