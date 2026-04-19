//! Config loading for GPU apps, backed by shikumi.
//!
//! Every GPU app in pleme-io ran the same five-line config bootstrap:
//! build a `ConfigDiscovery`, call `discover()` (with a CLI override),
//! load into a typed struct, return `anyhow::Result`. Collapsing that
//! into [`load_config`] saves ~50 LOC per app and ensures one canonical
//! XDG search path.
//!
//! Delegates to shikumi's `ConfigDiscovery` + `ConfigStore<T>` rather than
//! re-implementing figment layering. Hot-reload (`ConfigStore::load_and_watch`)
//! is not exposed here — GPU apps that need it can depend on shikumi
//! directly. This module is for the "load once at startup" path that every
//! app was duplicating.

use std::path::Path;

use anyhow::Context;
use serde::de::DeserializeOwned;
use shikumi::{ConfigDiscovery, ConfigStore, Format};

/// Load config for an app, honoring an optional explicit path override.
///
/// Behaviour:
/// 1. If `override_path` is set, load only that file through shikumi.
/// 2. Otherwise run shikumi's XDG discovery
///    (`$XDG_CONFIG_HOME/<app>/<app>.yaml|yml|toml`, …, legacy `$HOME/.<app>`).
/// 3. When nothing is found, return `Config::default()` — apps should
///    make fields `Option<T>` or rely on serde defaults so the no-config
///    path is meaningful.
///
/// The env-var prefix is derived from `app_name` by upper-casing and
/// appending `_` (`myapp` → `MYAPP_`). Use [`load_config_with_prefix`]
/// for apps with a non-standard convention.
///
/// # Errors
///
/// Propagates shikumi discovery and parse errors, annotated with the
/// resolved path for easy diagnosis.
pub fn load_config<C: DeserializeOwned + Default + Clone + Send + Sync + 'static>(
    app_name: &str,
    override_path: Option<&Path>,
) -> anyhow::Result<C> {
    let prefix = format!("{}_", app_name.to_ascii_uppercase());
    load_config_with_prefix(app_name, &prefix, override_path)
}

/// Like [`load_config`] but with an explicit env-var prefix.
///
/// Apps in a family that share a common env namespace (e.g. all
/// `BLACKMATTER_*`) use this to route through one prefix.
///
/// # Errors
///
/// Propagates shikumi discovery and parse errors.
pub fn load_config_with_prefix<C: DeserializeOwned + Default + Clone + Send + Sync + 'static>(
    app_name: &str,
    env_prefix: &str,
    override_path: Option<&Path>,
) -> anyhow::Result<C> {
    if let Some(path) = override_path {
        let store = ConfigStore::<C>::load(path, env_prefix)
            .with_context(|| format!("loading {app_name} config from {}", path.display()))?;
        return Ok((**store.get()).clone());
    }

    let discovery = ConfigDiscovery::new(app_name).formats(&[Format::Yaml, Format::Toml]);

    match discovery.discover() {
        Ok(path) => {
            let store = ConfigStore::<C>::load(&path, env_prefix)
                .with_context(|| format!("loading {app_name} config from {}", path.display()))?;
            Ok((**store.get()).clone())
        }
        Err(err) if err.is_not_found() => {
            tracing::debug!(app = app_name, "no config file found; using defaults");
            Ok(C::default())
        }
        Err(err) => {
            Err(anyhow::Error::new(err).context(format!("discovering {app_name} config")))
        }
    }
}

/// Trait alias for configs that appkit can load.
///
/// Auto-satisfied by any `#[derive(Deserialize, Default, Clone)]` struct
/// that is `Send + Sync + 'static`.
pub trait LoadConfig: DeserializeOwned + Default + Clone + Send + Sync + 'static {}

impl<T> LoadConfig for T where T: DeserializeOwned + Default + Clone + Send + Sync + 'static {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use std::fs;
    use tempfile::TempDir;

    #[derive(Deserialize, Default, Clone, Debug, PartialEq)]
    #[serde(default)]
    struct TestConfig {
        name: Option<String>,
        count: Option<u32>,
    }

    #[test]
    fn override_path_reads_yaml() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("app.yaml");
        fs::write(&path, "name: overridden\ncount: 42\n").unwrap();

        let cfg: TestConfig = load_config("testapp", Some(&path)).unwrap();
        assert_eq!(cfg.name.as_deref(), Some("overridden"));
        assert_eq!(cfg.count, Some(42));
    }

    #[test]
    fn override_path_reads_toml() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("app.toml");
        fs::write(&path, "name = \"from_toml\"\ncount = 7\n").unwrap();

        let cfg: TestConfig = load_config("testapp", Some(&path)).unwrap();
        assert_eq!(cfg.name.as_deref(), Some("from_toml"));
        assert_eq!(cfg.count, Some(7));
    }

    #[test]
    fn missing_config_returns_default() {
        let cfg: TestConfig =
            load_config("appkit-nonexistent-xyzzy-test-app", None).unwrap();
        assert_eq!(cfg, TestConfig::default());
    }

    #[test]
    fn parse_error_propagates() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("bad.yaml");
        fs::write(&path, "name: [unclosed\n").unwrap();

        let result: Result<TestConfig, _> = load_config("testapp", Some(&path));
        assert!(result.is_err(), "bad YAML should error");
        let msg = format!("{:#}", result.unwrap_err());
        assert!(msg.contains("bad.yaml"), "error should name the file: {msg}");
    }

    #[test]
    fn explicit_prefix_is_used() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("app.yaml");
        fs::write(&path, "name: base\n").unwrap();

        // shikumi's load() chain is: env -> file (file wins). The purpose
        // of this test is to verify the custom prefix is accepted and the
        // happy path still returns the file value.
        let cfg: TestConfig =
            load_config_with_prefix("testapp", "CUSTOM_APPKIT_", Some(&path)).unwrap();
        assert_eq!(cfg.name.as_deref(), Some("base"));
    }

    #[test]
    fn load_config_with_lowercase_app_name() {
        // Smoke test: load_config derives APP_ prefix from lowercase name.
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("mado.yaml");
        fs::write(&path, "name: from_file\n").unwrap();

        let cfg: TestConfig = load_config("mado", Some(&path)).unwrap();
        assert_eq!(cfg.name.as_deref(), Some("from_file"));
    }

    #[test]
    fn trait_alias_accepts_standard_deserializable() {
        fn takes_loadable<C: LoadConfig>() {}
        takes_loadable::<TestConfig>();
    }
}
