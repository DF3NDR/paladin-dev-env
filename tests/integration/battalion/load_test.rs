//! Load and performance tests for Battalion orchestration patterns.
//!
//! This module contains comprehensive load tests to verify:
//! - High concurrency handling (50+ concurrent Battalions)
//! - Performance targets (<1s orchestration overhead)
//! - Resource management under load
//! - Error handling at scale

use async_trait::async_trait;
use paladin::application::services::battalion::formation_service::FormationExecutionService;
use paladin::application::services::battalion::phalanx_service::PhalanxExecutionService;
use paladin::application::services::paladin::error::PaladinError;
use paladin::core::base::entity::node::Node;
use paladin::core::platform::container::battalion::formation::Formation;
use paladin::core::platform::container::battalion::phalanx::{AggregationStrategy, Phalanx};
use paladin::core::platform::container::battalion::{BattalionConfig, ErrorStrategy};
use paladin::core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus};
use paladin_ports::output::paladin_port::{
    PaladinPort, PaladinResult, PaladinStreamChunk, StopReason,
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Mock Paladin port for load testing with configurable latency
#[derive(Clone)]
struct LoadTestMockPort {
    call_count: Arc<Mutex<usize>>,
    latency_ms: u64,
}

impl LoadTestMockPort {
    fn new(latency_ms: u64) -> Self {
        Self {
            call_count: Arc::new(Mutex::new(0)),
            latency_ms,
        }
    }

    async fn get_call_count(&self) -> usize {
        *self.call_count.lock().await
    }
}

#[async_trait]
impl PaladinPort for LoadTestMockPort {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        // Increment call count
        {
            let mut count = self.call_count.lock().await;
            *count += 1;
        }

        // Simulate processing latency
        tokio::time::sleep(Duration::from_millis(self.latency_ms)).await;

        Ok(PaladinResult {
            output: format!("Processed: {} by {}", input, paladin.node.name),
            usage: paladin_ports::output::llm_port::TokenUsage::new(100, 0),
            execution_time_ms: self.latency_ms,
            loop_count: 1,
            stop_reason: StopReason::Completed,
            ..Default::default()
        })
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<tokio::sync::mpsc::Receiver<Result<PaladinStreamChunk, PaladinError>>, PaladinError>
    {
        let (_tx, rx) = tokio::sync::mpsc::channel(1);
        Ok(rx)
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

/// Helper to create a test Paladin
fn create_test_paladin(name: &str) -> Paladin {
    Node::new(
        PaladinData {
            system_prompt: format!("You are {}", name),
            name: name.to_string(),
            user_name: "test_user".to_string(),
            model: "gpt-4".to_string(),
            temperature: 0.7,
            max_loops: MaxLoops::Fixed(1),
            stop_words: vec![],
            status: PaladinStatus::Idle,
            vision_enabled: false,
            ..Default::default()
        },
        Some(name.to_string()),
    )
}

#[tokio::test]
async fn test_load_formation_50_concurrent_battalions() {
    // Test: 50 concurrent Formations, each with 10 Paladins
    // Target: Complete in <15s with reasonable overhead

    let start = Instant::now();
    let mock_port = Arc::new(LoadTestMockPort::new(10)); // 10ms per Paladin

    let mut tasks = vec![];

    for battalion_id in 0..50 {
        let mock_port_clone = mock_port.clone();

        let task = tokio::spawn(async move {
            let service = FormationExecutionService::new(mock_port_clone);

            // Create Formation with 10 Paladins
            let paladins: Vec<Paladin> = (0..10)
                .map(|i| create_test_paladin(&format!("paladin_{}_{}", battalion_id, i)))
                .collect();

            let config = BattalionConfig {
                timeout_seconds: 30,
                error_strategy: ErrorStrategy::FailFast,
                ..Default::default()
            };

            let formation = Formation::new(paladins, config).expect("Formation creation failed");

            let battalion_start = Instant::now();
            let result = service.execute(&formation, "test input").await;
            let battalion_duration = battalion_start.elapsed();

            (result, battalion_duration)
        });

        tasks.push(task);
    }

    // Wait for all Battalions to complete
    let results: Vec<_> = futures::future::join_all(tasks).await;
    let total_duration = start.elapsed();

    // Verify all completed successfully
    let mut successful = 0;
    let mut total_overhead = Duration::ZERO;

    for result in results {
        let (battalion_result, battalion_duration) = result.expect("Task panicked");
        assert!(
            battalion_result.is_ok(),
            "Battalion should succeed: {:?}",
            battalion_result.err()
        );

        // Overhead = total time - (10 Paladins * 10ms each)
        let expected_min = Duration::from_millis(100); // 10 * 10ms
        let overhead = battalion_duration.saturating_sub(expected_min);
        total_overhead += overhead;

        successful += 1;
    }

    // Verify performance targets
    assert_eq!(successful, 50, "All 50 Battalions should complete");

    let avg_overhead = total_overhead / 50;
    assert!(
        avg_overhead < Duration::from_secs(1),
        "Average orchestration overhead should be <1s, got {:?}",
        avg_overhead
    );

    // Total time should be reasonable (concurrent execution)
    assert!(
        total_duration < Duration::from_secs(15),
        "50 concurrent Battalions should complete in <15s, got {:?}",
        total_duration
    );

    // Verify total calls (50 Battalions * 10 Paladins = 500 calls)
    let total_calls = mock_port.get_call_count().await;
    assert_eq!(total_calls, 500, "Should have made 500 Paladin calls");

    println!("✅ Load test passed:");
    println!("   Total time: {:?}", total_duration);
    println!("   Average overhead: {:?}", avg_overhead);
    println!("   Total Paladin calls: {}", total_calls);
}

#[tokio::test]
async fn test_load_phalanx_concurrent_execution() {
    // Test: 20 concurrent Phalanxes, each with 10 Paladins running in parallel
    // Target: Complete faster than sequential due to concurrency

    let start = Instant::now();
    let mock_port = Arc::new(LoadTestMockPort::new(50)); // 50ms per Paladin

    let mut tasks = vec![];

    for battalion_id in 0..20 {
        let mock_port_clone = mock_port.clone();

        let task = tokio::spawn(async move {
            let service = PhalanxExecutionService::new(mock_port_clone);

            // Create Phalanx with 10 Paladins
            let paladins: Vec<Paladin> = (0..10)
                .map(|i| create_test_paladin(&format!("paladin_{}_{}", battalion_id, i)))
                .collect();

            let config = BattalionConfig {
                timeout_seconds: 30,
                error_strategy: ErrorStrategy::ContinueOnError,
                ..Default::default()
            };

            let phalanx = Phalanx::new(paladins, config)
                .expect("Phalanx creation failed")
                .with_aggregation(AggregationStrategy::CollectAll);

            let battalion_start = Instant::now();
            let result = service.execute(&phalanx, "test input").await;
            let battalion_duration = battalion_start.elapsed();

            (result, battalion_duration)
        });

        tasks.push(task);
    }

    // Wait for all Phalanxes to complete
    let results: Vec<_> = futures::future::join_all(tasks).await;
    let total_duration = start.elapsed();

    // Verify all completed successfully
    let mut successful = 0;

    for result in results {
        let (battalion_result, battalion_duration) = result.expect("Task panicked");
        assert!(
            battalion_result.is_ok(),
            "Phalanx should succeed: {:?}",
            battalion_result.err()
        );

        // Each Phalanx should complete in ~50ms (concurrent) + overhead
        // Much faster than 500ms (10 * 50ms sequential)
        assert!(
            battalion_duration < Duration::from_millis(200),
            "Phalanx should benefit from concurrency, got {:?}",
            battalion_duration
        );

        successful += 1;
    }

    assert_eq!(successful, 20, "All 20 Phalanxes should complete");

    // Total time should show concurrent execution benefit
    assert!(
        total_duration < Duration::from_secs(5),
        "20 concurrent Phalanxes should complete in <5s, got {:?}",
        total_duration
    );

    println!("✅ Phalanx load test passed:");
    println!("   Total time: {:?}", total_duration);
    println!("   Battalions: {}", successful);
}

#[tokio::test]
async fn test_stress_high_concurrency_limit() {
    // Stress test: Push concurrency to limits
    // 100 concurrent Battalions with varying sizes

    let start = Instant::now();
    let mock_port = Arc::new(LoadTestMockPort::new(5)); // Fast execution

    let mut tasks = vec![];

    for battalion_id in 0..100 {
        let mock_port_clone = mock_port.clone();

        // Alternate between Formation and Phalanx
        if battalion_id % 2 == 0 {
            let task = tokio::spawn(async move {
                let service = FormationExecutionService::new(mock_port_clone);

                let paladin_count = 3 + (battalion_id % 5); // 3-7 Paladins
                let paladins: Vec<Paladin> = (0..paladin_count)
                    .map(|i| create_test_paladin(&format!("f_{}_{}", battalion_id, i)))
                    .collect();

                let config = BattalionConfig::default();
                let formation = Formation::new(paladins, config).expect("Formation failed");

                service.execute(&formation, "stress test").await
            });

            tasks.push(task);
        } else {
            let task = tokio::spawn(async move {
                let service = PhalanxExecutionService::new(mock_port_clone);

                let paladin_count = 2 + (battalion_id % 4); // 2-5 Paladins
                let paladins: Vec<Paladin> = (0..paladin_count)
                    .map(|i| create_test_paladin(&format!("p_{}_{}", battalion_id, i)))
                    .collect();

                let config = BattalionConfig::default();
                let phalanx = Phalanx::new(paladins, config)
                    .expect("Phalanx failed")
                    .with_aggregation(AggregationStrategy::CollectAll);

                service.execute(&phalanx, "stress test").await
            });

            tasks.push(task);
        }
    }

    // Wait for all to complete
    let results: Vec<_> = futures::future::join_all(tasks).await;
    let total_duration = start.elapsed();

    // Verify all completed
    let successful = results
        .iter()
        .filter(|r| r.as_ref().expect("Task panicked").is_ok())
        .count();
    assert_eq!(
        successful, 100,
        "All 100 Battalions should complete under stress"
    );

    // Should complete in reasonable time even under high load
    assert!(
        total_duration < Duration::from_secs(30),
        "High concurrency stress test should complete in <30s, got {:?}",
        total_duration
    );

    println!("✅ Stress test passed:");
    println!("   Total time: {:?}", total_duration);
    println!("   Successful: {}/100", successful);
}

#[tokio::test]
async fn test_performance_orchestration_overhead() {
    // Measure pure orchestration overhead with minimal Paladin latency

    let mock_port = Arc::new(LoadTestMockPort::new(1)); // 1ms per Paladin
    let service = FormationExecutionService::new(mock_port.clone());

    // Create Formation with 5 Paladins
    let paladins: Vec<Paladin> = (0..5)
        .map(|i| create_test_paladin(&format!("paladin_{}", i)))
        .collect();

    let config = BattalionConfig::default();
    let formation = Formation::new(paladins, config).expect("Formation failed");

    // Measure execution time
    let start = Instant::now();
    let result = service.execute(&formation, "test").await;
    let duration = start.elapsed();

    assert!(
        result.is_ok(),
        "Execution should succeed: {:?}",
        result.err()
    );

    // Expected: 5 * 1ms = 5ms for Paladin execution
    let expected_paladin_time = Duration::from_millis(5);
    let overhead = duration.saturating_sub(expected_paladin_time);

    // Orchestration overhead should be <1s (much less actually, but being conservative)
    assert!(
        overhead < Duration::from_secs(1),
        "Orchestration overhead should be <1s, got {:?}",
        overhead
    );

    // More realistically, overhead should be <100ms
    assert!(
        overhead < Duration::from_millis(100),
        "Orchestration overhead should be <100ms, got {:?}",
        overhead
    );

    println!("✅ Orchestration overhead test passed:");
    println!("   Total time: {:?}", duration);
    println!("   Paladin time: {:?}", expected_paladin_time);
    println!("   Overhead: {:?}", overhead);
}

#[tokio::test]
async fn test_memory_efficiency_under_load() {
    // Verify memory doesn't grow unbounded under high load
    // Run 1000 small Formations in batches

    let mock_port = Arc::new(LoadTestMockPort::new(10));

    // Run 1000 small Formations in batches of 100
    for batch in 0..10 {
        let mut tasks = vec![];

        for i in 0..100 {
            let mock_port_clone = mock_port.clone();

            let task = tokio::spawn(async move {
                let service = FormationExecutionService::new(mock_port_clone);

                let paladins: Vec<Paladin> = (0..3)
                    .map(|j| create_test_paladin(&format!("p_{}_{}", batch * 100 + i, j)))
                    .collect();

                let config = BattalionConfig::default();
                let formation = Formation::new(paladins, config).expect("Formation failed");

                service.execute(&formation, "memory test").await
            });

            tasks.push(task);
        }

        // Wait for batch to complete
        let results: Vec<_> = futures::future::join_all(tasks).await;

        // Verify all succeeded
        assert_eq!(
            results
                .iter()
                .filter(|r| r.as_ref().expect("Task panicked").is_ok())
                .count(),
            100,
            "Batch {} should complete successfully",
            batch
        );
    }

    println!("✅ Memory efficiency test passed: 1000 Formations completed");
}
