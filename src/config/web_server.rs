//! Configuration for web server, sources, and message service.

use serde::{Deserialize, Serialize};

use crate::config::env_utils::{EnvOverridable, read_env};

/// Configuration for a content source
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SourceConfig {
    pub name: String,
    pub source_type: String,
    pub url: String,
    pub prompt: String,
    pub tags: Vec<String>,
}

/// Configuration for the HTTP server
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

/// Configuration for the message service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageServiceSettings {
    pub max_queue_size: Option<usize>,
    pub default_ttl_seconds: Option<i64>,
    pub enable_persistence: Option<bool>,
    pub worker_threads: Option<usize>,
    pub retry_attempts: Option<u32>,
    pub retry_delay_ms: Option<u64>,
}

/// Configuration for the developer-facing trace inspector page (D-26, D-36,
/// X-09).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DevUiConfig {
    /// The Mermaid ESM bundle URL the inspector page's module script
    /// imports (D-26). Defaults to the jsDelivr `mermaid@11` CDN bundle;
    /// settable so an air-gapped operator can point it at a local mirror.
    ///
    /// RED-phase stub (Task 3): deliberately wrong (empty) so the new
    /// tests fail. The GREEN commit sets the real jsDelivr URL.
    pub mermaid_url: String,
}

impl Default for DevUiConfig {
    fn default() -> Self {
        Self {
            mermaid_url: String::new(),
        }
    }
}

/// Top-level web-server-scoped configuration for concerns that are not core
/// HTTP bind/port settings (see [`ServerConfig`] for those) -- currently
/// just the developer inspector UI (D-26, D-36, X-09). Every field defaults
/// to today's behavior, so an absent `web_server:` key resolves to
/// [`WebServerConfig::default`] and boots identically to a v0.9
/// configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WebServerConfig {
    /// Developer inspector UI settings.
    pub dev_ui: DevUiConfig,
}

impl Default for WebServerConfig {
    fn default() -> Self {
        Self {
            dev_ui: DevUiConfig::default(),
        }
    }
}

impl WebServerConfig {
    /// Always valid today -- `mermaid_url` has no invalid state to reject
    /// (any string, including a local path or an empty override, is
    /// acceptable).
    pub fn validate(&self) -> Result<(), String> {
        Ok(())
    }
}

impl EnvOverridable for WebServerConfig {
    fn apply_env_overrides(&mut self) {
        if let Some(v) = read_env::<String>("APP_WEB_SERVER_DEV_UI_MERMAID_URL") {
            self.dev_ui.mermaid_url = v;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Behavior 1: `WebServerConfig::default().dev_ui.mermaid_url` is the
    /// jsDelivr `mermaid@11` ESM bundle URL (D-26, D-36).
    #[test]
    fn default_mermaid_url_is_the_jsdelivr_bundle() {
        let config = WebServerConfig::default();
        assert_eq!(
            config.dev_ui.mermaid_url,
            "https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs"
        );
    }

    /// Behavior 2: a `web_server: { dev_ui: { mermaid_url: ... } }` section
    /// loads that value; an absent `dev_ui:` key yields the default.
    #[test]
    fn partial_dev_ui_section_overrides_and_absent_defaults() {
        #[derive(Debug, Deserialize)]
        struct Wrapper {
            #[serde(default)]
            web_server: WebServerConfig,
        }

        fn deserialize_wrapper(yaml: &str) -> Wrapper {
            config::Config::builder()
                .add_source(config::File::from_str(yaml, config::FileFormat::Yaml))
                .build()
                .expect("config should build from the inline yaml fixture")
                .try_deserialize()
                .expect("wrapper should deserialize")
        }

        let overridden = deserialize_wrapper(
            "web_server:\n  dev_ui:\n    mermaid_url: \"/static/mermaid.esm.min.mjs\"\n",
        );
        assert_eq!(
            overridden.web_server.dev_ui.mermaid_url,
            "/static/mermaid.esm.min.mjs"
        );

        let absent = deserialize_wrapper("other_key: 1\n");
        assert_eq!(absent.web_server, WebServerConfig::default());
    }

    /// Behavior 3: the repository's own `config.example.yml`'s new
    /// `trace:` and `web_server:` sections deserialize and validate.
    ///
    /// This deliberately does NOT go through
    /// `Settings::load_from_file("config.example.yml")` end-to-end: that
    /// path already fails on a pre-existing, documented, out-of-scope gap
    /// unrelated to this plan -- `llm.ollama` intentionally carries no
    /// `api_key` field (see that block's own comment in the file) while
    /// `LlmProviderConfig::api_key` is a required `String`, so the whole
    /// file fails `try_deserialize` regardless of anything this plan
    /// touches. `agent_runtime.rs`'s own test module hits the identical
    /// wall and documents the same workaround
    /// (`deserialize_wrapper_from_file`, "isolates the pinning assertion
    /// from unrelated config schema drift"); this test follows that
    /// established precedent rather than re-fixing an unrelated crate's
    /// schema as a side effect of this plan (scope boundary).
    #[test]
    fn example_config_still_loads_and_validates() {
        #[derive(Debug, Deserialize)]
        struct Wrapper {
            #[serde(default)]
            trace: crate::config::trace::TraceConfig,
            #[serde(default)]
            web_server: WebServerConfig,
        }

        let wrapper: Wrapper = config::Config::builder()
            .add_source(config::File::new(
                "config.example.yml",
                config::FileFormat::Yaml,
            ))
            .build()
            .unwrap_or_else(|e| panic!("config.example.yml should build: {e}"))
            .try_deserialize()
            .unwrap_or_else(|e| {
                panic!("config.example.yml's trace/web_server sections should deserialize: {e}")
            });

        assert!(wrapper.trace.validate().is_ok());
        assert!(wrapper.web_server.validate().is_ok());
    }
}
