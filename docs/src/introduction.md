# Paladin Documentation

Welcome to the Paladin documentation! Paladin is a Rust-based enterprise multi-agent orchestration framework built with Hexagonal Architecture and Domain-Driven Design principles.

## 🚀 Getting Started

New to Paladin? Start here:

1. **[Quickstart Guide](getting-started/quickstart.md)** - Get your first Paladin agent running in 15 minutes
2. **[Installation](getting-started/installation.md)** - Detailed setup instructions for all platforms
3. **[Examples Gallery](https://github.com/DF3NDR/paladin-dev-env/tree/main/examples)** - Working code examples for common use cases

## 📚 User Guides

Learn how to build with Paladin:

- **[Autonomous Agent Features](user-guides/paladin-agents.md)** - Auto-planning, prompt generation, dynamic temperature, and agent handoffs
- **[Battalion Orchestration](user-guides/battalion-patterns.md)** - Multi-agent coordination with orchestration patterns
- **[Maneuver Flow DSL](user-guides/maneuver-flow-dsl.md)** - Declarative workflows with Flow DSL syntax
- **[Tool Integration (Arsenal)](user-guides/arsenal-tools.md)** - Integrate external tools via MCP protocol
- **[Memory Management (Garrison)](user-guides/garrison-memory.md)** - Conversation context and persistence
- **[Output Formatting (Herald)](user-guides/herald-output.md)** - Format and stream agent responses
- **[WarEngine: Superstep Execution](user-guides/superstep-engine.md)** - Battlefield state, Waypoint checkpointing, and cyclic graph execution
- **[Control Flow](user-guides/control-flow.md)** - Dynamic routing and subgraphs
- **[Parley & Chronicle](user-guides/parley-and-chronicle.md)** - Pause, resume, history and graceful shutdown
- **[Aegis: Fault Tolerance](user-guides/fault-tolerance.md)** - Retry, timeout, error handlers, model fallback and node caching
- **[Agent Runtime](user-guides/agent-runtime.md)** - Middleware, context management, vault memory, structured output and the reasoning agent
- **[Eval Harness](user-guides/eval-harness.md)** - Evaluate Paladin agent quality and catch regressions
- **[CLI Usage Guide](appendix/cli-usage.md)** - Complete command-line interface reference

## 🏗️ Architecture

Understand Paladin's design:

- **[Architecture Overview](architecture/overview.md)** - Three-layer hexagonal architecture
- **[Hexagonal Design](architecture/hexagonal-design.md)** - Port/adapter pattern implementation
- **[Domain Model](architecture/domain-model.md)** - DDD entities and relationships
- **[Commissary](architecture/commissary.md)** - Input-side, per-call window-rationing officer
- **[Design Patterns](architecture/design-patterns.md)** - Patterns used throughout Paladin

## 🧭 Deployment Topologies

Choose how to run Paladin:

- **[Choosing a Topology](deployment-topologies/overview.md)** - Compare embedded library, orchestrated Battalion, HTTP service host, queue/worker and sidecar deployments

## 🚢 Deployment

Deploy Paladin to production:

- **[Docker](deployment/docker.md)** - Containerized deployment
- **[Kubernetes](deployment/kubernetes.md)** - Cloud-native orchestration
- **[CI/CD](deployment/cicd.md)** - Automated pipelines with GitHub Actions
- **[Production Best Practices](deployment/production.md)** - Security, scaling, and reliability
- **[Platform API](api-reference/platform-api.md)** - Runs, threads, assistants, schedules and webhooks REST surface
- **[Versioning Policy](api-reference/stable-api.md)** - Lockstep versioning rules and transition criteria
- **[Release Checklist](appendix/release-checklist.md)** - Dependency-aware release and publish workflow

## 🔧 Operations

Monitor and maintain Paladin:

- **[Observability](operations/observability.md)** - Traces, sinks and persistence
- **[Logging](operations/logging.md)** - Structured logging configuration
- **[Monitoring](operations/monitoring.md)** - Metrics and dashboards
- **[Troubleshooting](operations/troubleshooting.md)** - Common issues and solutions
- **[Performance Tuning](operations/performance-tuning.md)** - Optimize for throughput and latency

## 🤝 Contributing

Extend and improve Paladin:

- **[Contribution Guide](contributing/development-setup.md)** - How to contribute
- **[Adapter Development](contributing/contributing-providers.md)** - Create custom adapters
- **[Testing Guide](contributing/testing-guide.md)** - Testing requirements and patterns

## 📖 API Reference

Comprehensive API documentation is available via rustdoc:

```bash
cargo doc --open
```

Or browse online at: [https://docs.rs/paladin](https://docs.rs/paladin) (when published)

## 🎯 Key Concepts

### Medieval Military Theme

Paladin uses a consistent Medieval Military naming convention. This is a short excerpt — see
[Domain Model](architecture/domain-model.md#medieval-military-naming-convention) for the
complete ubiquitous-language table of every Medieval-Military term Paladin uses:

| Term | Definition |
|------|------------|
| **Paladin** | An autonomous AI agent |
| **Battalion** | A coordinated group of Paladins |
| **Formation** | Sequential Paladin execution |
| **Phalanx** | Concurrent Paladin execution |
| **Campaign** | Graph-based orchestration |
| **Chain of Command** | Hierarchical delegation |
| **Garrison** | Agent memory storage |
| **Arsenal** | Tool and capability registry |

### Architecture Layers

Paladin follows hexagonal (ports and adapters) architecture:

- **Core Layer** - Pure domain logic, no external dependencies
- **Application Layer** - Use cases and port definitions (interfaces)
- **Infrastructure Layer** - Adapter implementations for external systems

Dependencies flow inward only: Infrastructure → Application → Core

## 💡 Support

- **Issues**: [GitHub Issues](https://github.com/DF3NDR/paladin-dev-env/issues)
- **Discussions**: [GitHub Discussions](https://github.com/DF3NDR/paladin-dev-env/discussions)
- **Documentation**: You're reading it!

## 📄 License

See [LICENSE](https://github.com/DF3NDR/paladin-dev-env/blob/main/LICENSE) for details.
