// examples/eval_scenarios_demo.rs
//
// Eval Scenarios: Declaration, the Live-Mode Toggle, and the CLI Form
// (EX-105, EX-106, EX-107) -- PRD 07 OBS-FR-13, 28-12/28-16/28-17
//
// `paladin-eval` is an UNCONDITIONAL dev-dependency of the root package
// (`tests/evals.rs`'s own `paladin_eval::eval_scenarios!` needs
// `ScenarioRunner` with no `--features` at all), so this program needs no
// required-features and gets no manifest declaration -- it is picked up by
// the bulk `cargo build --examples` selector.
//
// `paladin_eval::eval_scenarios!` itself expands to a `fn main()` that
// globs a `.eval.yaml` pattern at runtime and hands each `(file, case)`
// pair to `libtest_mimic` -- see `tests/evals.rs`. This program cannot
// invoke that macro literally (it already has its own `main()`), so it
// drives the SAME `Scenario`/`ScenarioRunner`/`Case` types the macro's
// expansion drives, directly and in-process:
//   1. declares two scenarios in Rust against a Paladin built with the
//      mock adapter, and runs them through `ScenarioRunner::run_case`
//      (EX-105),
//   2. names the live-mode environment variable's effect without ever
//      enabling it (EX-106), and
//   3. writes a real `.eval.yaml` scenario file to a temp directory,
//      prints the exact `paladin eval run` command line a user would run
//      against it, and drives the SAME glob pattern through
//      `ScenarioRunner::trials`/`run_case` in-process, so the printed
//      command and the demonstrated behaviour are the same thing (EX-107).
//
// To run this example:
// ```bash
// cargo run --example eval_scenarios_demo
// ```
//
// This program deliberately runs in the default NON-LIVE mode throughout
// (see Part 2) -- it needs no provider key and no external service. The
// workspace's own CLI binary is never spawned as a subprocess: it is
// gated behind the `cli` feature, which would force this program to
// declare a required-features list it does not otherwise need.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use paladin::MockLlmAdapter;
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin_battalion::engine::InputMapping;
use paladin_battalion::engine::graph::{EngineLimits, NodeSpec, WarGraph};
use paladin_core::platform::container::battlefield::{
    BattlefieldSchema, DispatchRule, FieldName, FieldSpec,
};
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::waypoint::NodeId;
use paladin_eval::{
    Assertion, Case, EVAL_SCHEMA_VERSION, LiveOptions, LlmScript, PALADIN_EVAL_LIVE_ENV,
    RunOptions, RunStatusValue, Scenario, ScenarioRunner, ScenarioTarget, ScriptEntry,
    ScriptedPorts, StoreKind, Times, check_live_mode,
};
use paladin_ports::output::llm_port::LlmPort;

/// The name every scenario below targets via `ScenarioTarget::Registered`.
const GRAPH_NAME: &str = "eval-scenarios-demo";

/// The Paladin node id every scenario's `llm.per_node` script is keyed to.
const NODE_NAME: &str = "assistant";

/// Build the one-Paladin-node demo graph `assistant` registers under
/// [`GRAPH_NAME`]. `ScenarioRunner`'s `GraphConstructor` is `Fn`, called
/// fresh per case, so `paladin` and `schema` are cloned on every call
/// rather than consumed.
fn build_demo_graph(
    paladin: Paladin,
    schema: BattlefieldSchema,
    answer: FieldName,
) -> impl Fn(&ScriptedPorts) -> WarGraph + Send + Sync {
    move |_ports: &ScriptedPorts| {
        // Not using `ScriptedPorts`: a `NodeSpec::Paladin` node's LLM
        // routing is substituted automatically by the runner's own
        // `ScenarioPaladinPort`, keyed by the Paladin's own `name` field --
        // `ScriptedPorts` exists for a `Function` node that needs a raw
        // `LlmPort` call, which this graph has none of.
        let mut graph = WarGraph::new(schema.clone(), EngineLimits::default());
        let node_id = NodeId::new(NODE_NAME);
        graph.add_node(
            node_id.clone(),
            NodeSpec::paladin(
                paladin.clone(),
                InputMapping::new("{question}"),
                answer.clone(),
            ),
        );
        graph.add_entry(node_id);
        graph
    }
}

/// Build a two-case scenario declared entirely in Rust (EX-105): the same
/// `Scenario`/`Case`/`Assertion` types a `.eval.yaml` file deserializes
/// into, constructed directly rather than parsed.
fn declared_scenario() -> Scenario {
    let cases = vec![
        Case {
            name: "answers_the_question".to_string(),
            input: BTreeMap::from([("question".to_string(), serde_json::json!("What is Rust?"))]),
            interrupt_after_superstep: None,
            parley_responses: Vec::new(),
            llm: None,
            assertions: vec![
                Assertion::RunStatus(RunStatusValue::Completed),
                Assertion::NodeExecuted {
                    node: NODE_NAME.to_string(),
                    times: Times::Exact(1),
                },
            ],
        },
        Case {
            name: "answers_a_second_question".to_string(),
            input: BTreeMap::from([(
                "question".to_string(),
                serde_json::json!("What is ownership?"),
            )]),
            interrupt_after_superstep: None,
            parley_responses: Vec::new(),
            llm: None,
            assertions: vec![Assertion::RunStatus(RunStatusValue::Completed)],
        },
    ];

    Scenario {
        schema_version: EVAL_SCHEMA_VERSION.to_string(),
        target: ScenarioTarget::Registered {
            registered: GRAPH_NAME.to_string(),
        },
        store: StoreKind::InMemory,
        llm: LlmScript {
            per_node: BTreeMap::from([(
                NODE_NAME.to_string(),
                vec![ScriptEntry::Text(
                    "Rust is a systems programming language focused on safety, speed, and \
                     concurrency."
                        .to_string(),
                )],
            )]),
            ..LlmScript::default()
        },
        cases,
        live: LiveOptions::default(),
        registries: None,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== eval_scenarios_demo: EX-105, EX-106, EX-107 ===\n");

    let question = FieldName::new("question")?;
    let answer = FieldName::new("answer")?;
    let schema = BattlefieldSchema::new(vec![
        FieldSpec::new(question.clone(), DispatchRule::LastWrite, None, false),
        FieldSpec::new(answer.clone(), DispatchRule::LastWrite, None, false),
    ]);

    // A mock LLM adapter is required to build a `Paladin` at all -- it is
    // never actually called: `ScenarioRunner` substitutes its own
    // scripted `PaladinPort` for every node at run time, keyed by node
    // name, per the `llm.per_node` script below.
    let paladin = PaladinBuilder::new(Arc::new(MockLlmAdapter::new()) as Arc<dyn LlmPort>)
        .system_prompt("You answer questions about Rust concisely.")
        .name(NODE_NAME)
        .model("demo-model")
        .temperature(0.0)
        .max_loops(1)
        .build()
        .await?;

    let mut runner = ScenarioRunner::new();
    runner.register_graph(
        GRAPH_NAME,
        Arc::new(build_demo_graph(paladin, schema, answer)),
    );

    // Part 1 (EX-105): declare scenarios through the scenario format and
    // run them through the scenario runner.
    println!("--- Part 1: declaring scenarios and running them (EX-105) ---");
    let scenario = declared_scenario();
    let declared_path = Path::new("eval_scenarios_demo");
    for case in &scenario.cases {
        let report = runner
            .run_case(declared_path, &scenario, case, RunOptions::default())
            .await;
        println!(
            "  case '{}': {}",
            report.case_name,
            if report.passed() { "PASSED" } else { "FAILED" }
        );
        if !report.passed() {
            println!("    {}", report.render_failures());
        }
    }
    println!();

    // Part 2 (EX-106): name the live-mode toggle without enabling it.
    println!("--- Part 2: the live-mode toggle (EX-106) ---");
    let live_value = std::env::var(PALADIN_EVAL_LIVE_ENV).ok();
    println!(
        "{PALADIN_EVAL_LIVE_ENV}={live_value:?} -- enabling it (together with the CLI's \
         --live flag and a configured provider credential) would make every scenario call \
         a REAL provider instead of this program's scripted mock text above. This program \
         deliberately stays in the default non-live mode throughout, so it needs no \
         provider key."
    );
    // `check_live_mode` is the three-way gate `ScenarioRunner::run_case`
    // itself consults whenever a caller asks for live mode -- called here
    // with `live_flag: false` (this program's own intent), it refuses
    // immediately with the typed `FlagNotSet` variant rather than
    // `Ok(())`: there is no such thing as "live mode, but only a little".
    match check_live_mode(false) {
        Ok(()) => println!("check_live_mode(false) -> Ok (unexpected)"),
        Err(error) => println!(
            "check_live_mode(false) -> Err({error}) -- refused immediately, exactly as this \
             program's own non-live intent expects.\n"
        ),
    }

    // Part 3 (EX-107): the command-line form.
    println!("--- Part 3: the command-line form (EX-107) ---");
    let temp_dir = tempfile::tempdir()?;
    let scenario_file = temp_dir.path().join("demo.eval.yaml");
    std::fs::write(
        &scenario_file,
        r#"schema_version: "1"
target:
  registered: "eval-scenarios-demo"
llm:
  per_node:
    assistant:
      - text: "Rust is a systems programming language focused on safety, speed, and concurrency."
cases:
  - name: cli_form_case
    input:
      question: "What is a borrow checker?"
    assertions:
      - run_status: completed
"#,
    )?;
    let glob_pattern = format!("{}/*.eval.yaml", temp_dir.path().display());
    println!("Wrote a scenario file to {}.", scenario_file.display());
    println!(
        "The equivalent CLI form:\n  cargo run --bin paladin --features cli -- eval run \
         \"{glob_pattern}\"\n  (the temp file above is removed when this program exits; \
         point the glob at a scenario file of your own to actually run this command)\n"
    );

    let trial_count = runner.trials(&glob_pattern).len();
    println!(
        "Driving the same glob through ScenarioRunner::trials in-process resolves \
         {trial_count} trial(s) -- the same discovery `paladin eval run` and the `evals` \
         test harness both use."
    );

    let written_scenario = Scenario::from_path(&scenario_file)?;
    for case in &written_scenario.cases {
        let report = runner
            .run_case(
                &scenario_file,
                &written_scenario,
                case,
                RunOptions::default(),
            )
            .await;
        println!(
            "  case '{}' (from the written file): {}",
            report.case_name,
            if report.passed() { "PASSED" } else { "FAILED" }
        );
    }

    println!("\n=== eval_scenarios_demo complete ===");
    Ok(())
}
