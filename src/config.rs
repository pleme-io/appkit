//! Config loading — thin facade over `shidou::config`.
//!
//! shidou already owns the shikumi-backed discovery + env + fallback
//! pattern. appkit re-exposes those entry points under the appkit name so
//! GPU apps only need a single `use appkit::load_config;` instead of
//! learning shidou's API on day one.
//!
//! If you need hot-reload (`ConfigStore::load_and_watch`), depend on
//! shikumi directly — appkit is for the "load once at startup" path.

use std::path::Path;

use serde::de::DeserializeOwned;

/// Load config for an app, honoring an optional explicit path override.
///
/// Delegates to shidou:
/// - `Some(path)` → `shidou::config::load_config_from_path(app_name, path)`
/// - `None` → `shidou::config::load_config(app_name)` (XDG discovery)
///
/// shidou's contract: the return is `T`, not `Result<T>`. Missing files
/// and parse errors log a warning and fall back to `T::default()`, so
/// `load_config` never fails — apps proceed with defaults on error.
/// Design matches shidou because swallowing a bad config at launch is
/// friendlier for desktop apps than aborting.
#[must_use]
pub fn load_config<C: LoadConfig>(app_name: &str, override_path: Option<&Path>) -> C {
    match override_path {
        Some(path) => shidou::config::load_config_from_path(app_name, path),
        None => shidou::config::load_config(app_name),
    }
}

/// Trait alias for configs that appkit can load.
///
/// Auto-satisfied by any `#[derive(Deserialize, Default, Clone)]` struct
/// that is `Send + Sync + 'static`. Bounds match shidou's `AppRunner::Config`
/// so consumers can reuse the same type for `appkit::run_gpu_app` and
/// `shidou::AppRunner` implementations.
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

        let cfg: TestConfig = load_config("testapp", Some(&path));
        assert_eq!(cfg.name.as_deref(), Some("overridden"));
        assert_eq!(cfg.count, Some(42));
    }

    #[test]
    fn missing_config_returns_default() {
        let cfg: TestConfig = load_config("appkit-nonexistent-xyzzy-test-app", None);
        assert_eq!(cfg, TestConfig::default());
    }

    #[test]
    fn parse_error_falls_back_to_default() {
        // shidou's contract: invalid config logs a warning and returns
        // default — never an error. This is a behaviour change vs. the
        // pre-shidou appkit::load_config which returned Result.
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("bad.yaml");
        fs::write(&path, "name: [unclosed\n").unwrap();

        let cfg: TestConfig = load_config("testapp", Some(&path));
        assert_eq!(cfg, TestConfig::default());
    }

    #[test]
    fn trait_alias_accepts_standard_deserializable() {
        fn takes_loadable<C: LoadConfig>() {}
        takes_loadable::<TestConfig>();
    }
}
