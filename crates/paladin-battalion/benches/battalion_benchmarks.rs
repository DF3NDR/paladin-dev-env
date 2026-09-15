use async_trait::async_trait;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use paladin_battalion::campaign_service::CampaignExecutionService;
use paladin_battalion::chain_of_command_service::ChainOfCommandExecutionService;
use paladin_battalion::formation_service::FormationExecutionService;
use paladin_battalion::phalanx_service::PhalanxExecutionService;
use paladin_core::base::entity::node::Node;
use paladin_core::platform::container::battalion::BattalionConfig;
use paladin_core::platform::container::battalion::TokenUsage;
use paladin_core::platform::container::battalion::campaign::{
    Campaign, CampaignEdge, EdgeCondition,
};
use paladin_core::platform::container::battalion::chain_of_command::{
    ChainOfCommand, DelegationStrategy,
};
use paladin_core::platform::container::battalion::formation::Formation;
use paladin_core::platform::container::battalion::phalanx::{AggregationStrategy, Phalanx};
use paladin_core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus};
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_ports::output::paladin_port::{
    PaladinPort, PaladinResult, PaladinStream, PaladinStreamChunk, StopReason,
};
use std::sync::Arc;
use tokio::runtime::Runtime;

fn create_test_paladin(name: &str) -> Paladin {
    let data = PaladinData {
        system_prompt: format!("You are {}", name),
        name: name.to_string(),
        user_name: "BenchmarkUser".to_string(),
        model: "mock-model".to_string(),
        temperature: 0.1,
        max_loops: MaxLoops::Fixed(1),
        stop_words: Vec::new(),
        status: PaladinStatus::Idle,
        vision_enabled: false,
        ..Default::default()
    };

    Node::new(data, Some(name.to_string()))
}

struct MockPaladinPort;

#[async_trait]
impl PaladinPort for MockPaladinPort {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        Ok(PaladinResult {
            output: format!("{}::{}", paladin.node.name, input),
            usage: TokenUsage::new(12, 0),
            execution_time_ms: 1,
            loop_count: 1,
            stop_reason: StopReason::Completed,
            ..Default::default()
        })
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinStream, PaladinError> {
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        tokio::spawn(async move {
            let _ = tx
                .send(Ok(PaladinStreamChunk {
                    text: String::new(),
                    is_final: true,
                    metadata: None,
                }))
                .await;
        });
        Ok(rx)
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

fn benchmark_formation_three_agents(c: &mut Criterion) {
    let rt = Runtime::new().expect("runtime");
    let service = FormationExecutionService::new(Arc::new(MockPaladinPort));

    let paladins = vec![
        create_test_paladin("formation-1"),
        create_test_paladin("formation-2"),
        create_test_paladin("formation-3"),
    ];

    let formation =
        Formation::new(paladins, BattalionConfig::new("formation-bench")).expect("valid formation");

    c.bench_function("battalion/formation_3_agents", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = service
                .execute(&formation, black_box("formation-input"))
                .await
                .expect("formation execute");
        });
    });
}

fn benchmark_phalanx_five_agents(c: &mut Criterion) {
    let rt = Runtime::new().expect("runtime");
    let service = PhalanxExecutionService::new(Arc::new(MockPaladinPort));

    let paladins: Vec<Paladin> = (0..5)
        .map(|i| create_test_paladin(&format!("phalanx-{}", i)))
        .collect();

    let phalanx = Phalanx::new(paladins, BattalionConfig::new("phalanx-bench"))
        .expect("valid phalanx")
        .with_aggregation(AggregationStrategy::CollectAll)
        .with_max_concurrency(5);

    c.bench_function("battalion/phalanx_5_agents", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = service
                .execute(&phalanx, black_box("phalanx-input"))
                .await
                .expect("phalanx execute");
        });
    });
}

fn benchmark_campaign_branching_dag(c: &mut Criterion) {
    let rt = Runtime::new().expect("runtime");
    let service = CampaignExecutionService::new(Arc::new(MockPaladinPort));

    let mut campaign = Campaign::new(BattalionConfig::new("campaign-bench"));

    let entry = campaign.add_paladin(create_test_paladin("campaign-entry"));
    let branch_a = campaign.add_paladin(create_test_paladin("campaign-branch-a"));
    let branch_b = campaign.add_paladin(create_test_paladin("campaign-branch-b"));
    let join = campaign.add_paladin(create_test_paladin("campaign-join"));

    campaign
        .add_edge(CampaignEdge::new(entry, branch_a, EdgeCondition::Always))
        .expect("entry->a");
    campaign
        .add_edge(CampaignEdge::new(entry, branch_b, EdgeCondition::Always))
        .expect("entry->b");
    campaign
        .add_edge(CampaignEdge::new(branch_a, join, EdgeCondition::Always))
        .expect("a->join");
    campaign
        .add_edge(CampaignEdge::new(branch_b, join, EdgeCondition::Always))
        .expect("b->join");

    c.bench_function("battalion/campaign_branching_dag", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = service
                .execute(&campaign, black_box("campaign-input"))
                .await
                .expect("campaign execute");
        });
    });
}

fn benchmark_chain_of_command(c: &mut Criterion) {
    let rt = Runtime::new().expect("runtime");
    let service = ChainOfCommandExecutionService::new(Arc::new(MockPaladinPort));

    let specialists_3: Vec<Paladin> = (0..3)
        .map(|i| create_test_paladin(&format!("chain-specialist-3-{}", i)))
        .collect();
    let chain_3 = ChainOfCommand::new(
        create_test_paladin("chain-commander-3"),
        specialists_3,
        BattalionConfig::new("chain-of-command-bench-3"),
    )
    .expect("valid chain of command")
    .with_strategy(DelegationStrategy::Broadcast);

    c.bench_function("battalion/chain_of_command_2_levels_3_subordinates", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = service
                .execute(&chain_3, black_box("chain-of-command-input"))
                .await
                .expect("chain of command execute");
        });
    });

    let specialists_5: Vec<Paladin> = (0..5)
        .map(|i| create_test_paladin(&format!("chain-specialist-5-{}", i)))
        .collect();
    let chain_5 = ChainOfCommand::new(
        create_test_paladin("chain-commander-5"),
        specialists_5,
        BattalionConfig::new("chain-of-command-bench-5"),
    )
    .expect("valid chain of command")
    .with_strategy(DelegationStrategy::Broadcast);

    c.bench_function("battalion/chain_of_command_2_levels_5_subordinates", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = service
                .execute(&chain_5, black_box("chain-of-command-input"))
                .await
                .expect("chain of command execute");
        });
    });

    let specialists_10: Vec<Paladin> = (0..10)
        .map(|i| create_test_paladin(&format!("chain-specialist-10-{}", i)))
        .collect();
    let chain_10 = ChainOfCommand::new(
        create_test_paladin("chain-commander-10"),
        specialists_10,
        BattalionConfig::new("chain-of-command-bench-10"),
    )
    .expect("valid chain of command")
    .with_strategy(DelegationStrategy::Broadcast);

    c.bench_function("battalion/chain_of_command_wide_10_subordinates", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = service
                .execute(&chain_10, black_box("chain-of-command-input"))
                .await
                .expect("chain of command execute");
        });
    });
}

criterion_group!(
    battalion_benches,
    benchmark_formation_three_agents,
    benchmark_phalanx_five_agents,
    benchmark_campaign_branching_dag,
    benchmark_chain_of_command
);
criterion_main!(battalion_benches);
