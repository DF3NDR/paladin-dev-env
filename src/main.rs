use clap::Parser;
use env_logger::Env;
use log::info;
use paladin::config::Settings;
use paladin::config::setup::setup_and_run;

#[derive(Parser, Debug)]
#[command(name = "smartcontent-aggregator")]
struct Opt {
    #[arg(short, long, default_value = "config.yml")]
    config: String,
}

/// Render the startup line for `config`.
///
/// Names only the shape of the configuration: `llm_type`, the bind address, the agent count and
/// the `http.auth` facts (enabled flag and the *number* of API keys). It never renders a
/// credential or a URL, so it is safe at the binary's default `debug` log filter (DF-07).
fn settings_summary(config: &Settings) -> String {
    let http = match config.http.as_ref() {
        Some(http) => format!(
            "http.auth.enabled={}, http.auth.api_keys={}",
            http.auth.enabled,
            http.auth.api_keys.len()
        ),
        None => "http=none".to_string(),
    };
    format!(
        "Loaded configuration: llm_type={}, server={}:{}, agents={}, {}",
        config.llm_type,
        config.server.host,
        config.server.port,
        config.agents.len(),
        http
    )
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env file in development (debug builds only)
    // Production deployments should use proper secrets management
    #[cfg(debug_assertions)]
    {
        // Silently ignore if .env doesn't exist - not all dev setups need it
        let _ = dotenv::dotenv();
    }

    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    let opt = Opt::parse();
    let config = match Settings::load_from_file(&opt.config) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to load configuration: {:?}", e);
            std::process::exit(1);
        }
    };

    info!("{}", settings_summary(&config));

    setup_and_run(config).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin::config::ApiKeyConfig;
    use paladin_core::platform::container::user::UserRole;

    const LLM_KEY_SENTINEL: &str = "test-key-not-a-secret-df07-llm-api-key";
    const API_KEY_SENTINEL: &str = "test-key-not-a-secret-df07-api-key";

    fn example_settings_with_fakes() -> Settings {
        let mut config =
            Settings::load_from_file(concat!(env!("CARGO_MANIFEST_DIR"), "/config.example.yml"))
                .expect("config.example.yml loads");
        config.llm_api_key = LLM_KEY_SENTINEL.to_string();
        let http = config.http.as_mut().expect("example config has http");
        http.auth.api_keys.push(ApiKeyConfig {
            key: API_KEY_SENTINEL.to_string(),
            name: "df07-probe".to_string(),
            role: UserRole::User,
            tenant: "Acme-Prod".to_string(),
        });
        config
    }

    #[test]
    fn settings_summary_names_no_credential() {
        let config = example_settings_with_fakes();
        // Non-vacuous: both sentinels really sit in the config we summarise.
        assert_eq!(config.llm_api_key, LLM_KEY_SENTINEL);
        assert_eq!(config.http.as_ref().map(|h| h.auth.api_keys.len()), Some(1));
        let line = settings_summary(&config);
        assert!(
            !line.contains(LLM_KEY_SENTINEL),
            "startup line leaked llm_api_key"
        );
        assert!(
            !line.contains(API_KEY_SENTINEL),
            "startup line leaked an http.auth.api_keys key"
        );
        assert!(!line.contains("Settings {"), "startup line dumps Settings");
    }

    #[test]
    fn settings_summary_names_the_shape_without_values() {
        let config = example_settings_with_fakes();
        let line = settings_summary(&config);
        assert!(
            line.starts_with("Loaded configuration: "),
            "unexpected prefix: {}",
            line.chars().take(40).collect::<String>()
        );
        assert!(line.contains("llm_type=openai"), "no llm_type in summary");
        assert!(
            line.contains("http.auth.api_keys=1"),
            "no api_keys count in summary"
        );
    }

    #[test]
    fn test_opt_default_config() {
        let opt = Opt::parse_from(["test"]);
        assert_eq!(opt.config, "config.yml");
    }

    #[test]
    fn test_opt_custom_config() {
        let opt = Opt::parse_from(["test", "--config", "custom.yml"]);
        assert_eq!(opt.config, "custom.yml");
    }

    #[test]
    fn test_opt_short_config() {
        let opt = Opt::parse_from(["test", "-c", "short.yml"]);
        assert_eq!(opt.config, "short.yml");
    }
}
