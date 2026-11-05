// examples/muster_baseline.rs
//
// Muster Baseline — Performance Measurement Harness
//
// This example is a purpose-built measurement harness for the performance baseline
// document at `docs/src/appendix/performance-baseline.md` (Phase 3 / QUAL-05, D-13).
// It records the two metric families `criterion` does not produce: memory-per-Paladin
// (process resident-set-size delta across a controlled muster of constructed Paladins)
// and startup time (in-process wall clock to the first fully-constructed Paladin).
//
// Every figure this harness prints is this host's baseline, measured under its stated
// environment — never a portable performance claim and never a cross-machine
// regression signal (see the baseline document's provenance block for the environment
// this run's figures belong to).
//
// `muster` is the medieval-military term for assembling troops, which is what this
// harness does to Paladins.
//
// To run this example:
// ```bash
// APP_ENV=test cargo run --offline --release --example muster_baseline
// ```

use paladin::MockLlmAdapter;
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin_ports::output::llm_port::LlmPort;
use std::io::Read as _;
use std::sync::Arc;
use std::time::Instant;

/// Number of Paladins mustered for the memory measurement. Named so the
/// `bytes_per_paladin` divisor printed by this harness is traceable to a fixed,
/// visible constant rather than a magic number.
const MUSTER_SIZE: usize = 1000;

/// Reads the current process's resident set size (RSS) from `/proc/self/status`, in
/// kilobytes. Returns a `Result` rather than unwrapping/panicking — an RSS read
/// failure must surface to the caller, not abort the harness (threat T-03-11).
fn read_vm_rss_kb() -> std::io::Result<u64> {
    let mut status = String::new();
    std::fs::File::open("/proc/self/status")?.read_to_string(&mut status)?;

    let line = status
        .lines()
        .find(|l| l.starts_with("VmRSS:"))
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "VmRSS line not found in /proc/self/status",
            )
        })?;

    // Pull the digits out of a line shaped like "VmRSS:\t   12345 kB" rather than
    // matching the "kB" suffix exactly, so the parse is resilient to whitespace width.
    let digits: String = line.chars().filter(char::is_ascii_digit).collect();
    digits
        .parse::<u64>()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// Builds one measurement-fixture Paladin against a shared mock LLM port. A real
/// `Paladin` aggregate is constructed via the same `PaladinBuilder` path the rest of
/// the workspace's examples use — not an empty struct — so the RSS delta reflects an
/// actual Paladin's memory footprint.
async fn build_fixture_paladin(
    llm_port: Arc<dyn LlmPort>,
    index: usize,
) -> Result<paladin::core::platform::container::paladin::Paladin, Box<dyn std::error::Error>> {
    let paladin = PaladinBuilder::new(llm_port)
        .system_prompt("You are a measurement fixture Paladin for the muster baseline harness")
        .name(format!("MusterFixture-{index}"))
        .model("gpt-4o-mini")
        .temperature(0.7)
        .max_loops(1)
        .build()
        .await?;
    Ok(paladin)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();

    let llm_port: Arc<dyn LlmPort> =
        Arc::new(MockLlmAdapter::new().with_response("muster baseline response".to_string()));

    // Construct the first Paladin — this is the event `startup_to_first_paladin_ms`
    // measures elapsed time to. This figure excludes pre-`main` dynamic-link and Rust
    // runtime initialization time; the baseline document also records a whole-process
    // wall-clock figure captured from the shell, which does include that time.
    let first_paladin = build_fixture_paladin(llm_port.clone(), 0).await?;
    let startup_to_first_paladin_ms = start.elapsed().as_millis();
    println!("startup_to_first_paladin_ms={startup_to_first_paladin_ms}");

    // RSS immediately before mustering the remaining Paladins.
    let rss_before_kb = read_vm_rss_kb()?;
    println!("rss_before_kb={rss_before_kb}");

    // Hold every constructed Paladin alive in one Vec so nothing is dropped
    // mid-measurement, which would understate the memory footprint.
    let mut mustered = Vec::with_capacity(MUSTER_SIZE);
    mustered.push(first_paladin);
    for index in 1..MUSTER_SIZE {
        let paladin = build_fixture_paladin(llm_port.clone(), index).await?;
        mustered.push(paladin);
    }

    let rss_after_kb = read_vm_rss_kb()?;
    println!("rss_after_kb={rss_after_kb}");
    println!("paladins_mustered={}", mustered.len());

    let rss_delta_kb = rss_after_kb.saturating_sub(rss_before_kb);
    println!("rss_delta_kb={rss_delta_kb}");

    // bytes_per_paladin = (rss_delta_kb * 1024) / paladins_mustered — printed so a
    // reader can re-derive the figure without re-running the harness.
    let bytes_per_paladin = (rss_delta_kb * 1024) / mustered.len() as u64;
    println!("bytes_per_paladin={bytes_per_paladin}");

    // Keep `mustered` alive through the final measurement above; drop explicitly here
    // rather than letting scope-end drop happen implicitly, so the measurement window
    // is unambiguous to a reader.
    drop(mustered);

    Ok(())
}
