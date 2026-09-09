/// Paladin CLI - Command-line interface for Paladin multi-agent orchestration
use clap::{Parser, Subcommand};
use paladin::application::cli::commands::{
    agent::{AgentCommands, handle_agent_new, handle_agent_run},
    arsenal::{ArsenalCommands, handle_arsenal_command},
    battalion::{BattalionCommands, handle_battalion_new, handle_battalion_run},
    council,
    eval::{EvalCommands, run_eval},
    features,
    graph::{GraphCommands, run_graph_export},
    maneuver::{ManeuverCommands, handle_maneuver_command},
    muster, onboarding,
    run::{RunCommands, run_run_export},
    setup_check,
};
use paladin::application::cli::error::CliError;
use std::process;
use tokio::signal;

#[derive(Parser)]
#[command(name = "paladin")]
#[command(version, about = "Paladin Multi-Agent Orchestration CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable quiet mode (minimal output)
    #[arg(long, global = true)]
    quiet: bool,

    /// Enable verbose mode (detailed output)
    #[arg(long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Paladin agent operations (create, run)
    Agent {
        #[command(subcommand)]
        action: AgentCommands,
    },
    /// Battalion multi-agent operations (create, run)
    Battalion {
        #[command(subcommand)]
        action: BattalionCommands,
    },
    /// Arsenal tool management (list, test)
    Arsenal {
        #[command(subcommand)]
        action: ArsenalCommands,
    },
    /// Maneuver flow DSL operations (visualize, validate, execute)
    Maneuver {
        #[command(subcommand)]
        action: ManeuverCommands,
    },
    /// Interactive onboarding wizard for initial setup
    Onboarding,
    /// Check environment setup and configuration
    SetupCheck {
        /// Show detailed diagnostic information
        #[arg(long)]
        verbose: bool,
    },
    /// Discover available features and commands
    Features {
        /// Filter by category
        #[arg(long)]
        category: Option<String>,
        /// Output format (table, json)
        #[arg(long)]
        format: Option<String>,
    },
    /// Generate battalion configuration from task description
    Muster {
        /// Task description
        #[arg(long)]
        task: Option<String>,
        /// Output file path
        #[arg(long, short)]
        output: Option<String>,
        /// Execute immediately after generation
        #[arg(long)]
        execute: bool,
        /// LLM provider to use
        #[arg(long)]
        provider: Option<String>,
        /// Model to use
        #[arg(long)]
        model: Option<String>,
        /// Skip review step
        #[arg(long)]
        no_review: bool,
    },
    /// Evaluation harness operations (run scripted scenarios)
    Eval {
        #[command(subcommand)]
        action: EvalCommands,
    },
    /// Graph document operations (export to Mermaid/DOT)
    Graph {
        #[command(subcommand)]
        action: GraphCommands,
    },
    /// Run/thread execution overlay operations (export to Mermaid)
    Run {
        #[command(subcommand)]
        action: RunCommands,
    },
    /// Run a council discussion
    Council {
        /// Discussion topic
        #[arg(long)]
        topic: Option<String>,
        /// Number of participants (2-10)
        #[arg(long, default_value = "3")]
        participants: usize,
        /// Custom roles (comma-separated)
        #[arg(long, value_delimiter = ',')]
        roles: Option<Vec<String>>,
        /// Maximum discussion rounds
        #[arg(long, default_value = "5")]
        max_rounds: usize,
        /// Save transcript to file
        #[arg(long)]
        save: Option<String>,
        /// Model to use
        #[arg(long)]
        model: Option<String>,
        /// Temperature setting
        #[arg(long)]
        temperature: Option<f32>,
    },
}

#[tokio::main]
async fn main() {
    // Setup SIGINT handler for graceful shutdown (Ctrl+C)
    let _sigint_handler = tokio::spawn(async {
        if signal::ctrl_c().await.is_ok() {
            eprintln!("\n\nReceived interrupt signal (Ctrl+C). Exiting...");
            process::exit(130); // Standard SIGINT exit code
        }
    });

    let cli = Cli::parse();

    // Store global flags for later use
    let _quiet = cli.quiet;
    let _verbose = cli.verbose;

    let result = match cli.command {
        Commands::Agent { action } => match action {
            AgentCommands::New(args) => handle_agent_new(args),
            AgentCommands::Run(args) => handle_agent_run(args).await,
        },
        Commands::Battalion { action } => match action {
            BattalionCommands::New(args) => handle_battalion_new(args),
            BattalionCommands::Run(args) => handle_battalion_run(args).await,
        },
        Commands::Arsenal { action } => handle_arsenal_command(action).await,
        Commands::Maneuver { action } => handle_maneuver_command(action).await,
        Commands::Onboarding => onboarding::run_onboarding().await,
        Commands::SetupCheck { verbose } => {
            setup_check::run_setup_check(verbose)
                .await
                .map(|exit_code| {
                    process::exit(exit_code);
                })
        }
        Commands::Features { category, format } => features::run_features(category, format).await,
        Commands::Eval { action } => match action {
            EvalCommands::Run(args) => {
                run_eval(
                    args.glob,
                    args.repeat,
                    args.bless,
                    args.live,
                    args.registries,
                )
                .await
            }
        },
        Commands::Graph { action } => match action {
            GraphCommands::Export(args) => {
                run_graph_export(args.format, args.file, args.assistant, args.out).await
            }
        },
        Commands::Run { action } => match action {
            RunCommands::Export(args) => {
                run_run_export(args.thread, args.waypoint, args.run, args.graph, args.out).await
            }
        },
        Commands::Muster {
            task,
            output,
            execute,
            provider,
            model,
            no_review,
        } => muster::run_muster(task, output, execute, provider, model, no_review).await,
        Commands::Council {
            topic,
            participants,
            roles,
            max_rounds,
            save,
            model,
            temperature,
        } => {
            council::run_council(
                topic,
                participants,
                roles,
                max_rounds,
                save,
                model,
                temperature,
            )
            .await
        }
    };

    // Handle errors and exit with appropriate code per FR-21
    // - 0: Success
    // - 1: User errors (config, validation, missing args)
    // - 2: Runtime errors (LLM, execution, tools)
    // - 130: SIGINT (handled by signal handler above)
    if let Err(e) = result {
        let error: CliError = e;
        eprintln!("{}", error.format_detailed());
        process::exit(error.exit_code());
    }
}
