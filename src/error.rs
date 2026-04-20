//! Error types for appkit.

use thiserror::Error;

/// Errors surfaced by the appkit bootstrap layer.
///
/// User-level errors from `AppState::init` return `anyhow::Error` and are
/// propagated as-is. `AppkitError` covers only the framework seam.
///
/// Config errors have no variant here because `load_config` is infallible:
/// shidou absorbs parse/discovery failures as tracing warnings and returns
/// `T::default()`. If a consumer wants strict config validation they pull
/// `shikumi::ConfigStore` directly.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AppkitError {
    /// madori event loop returned an error (window creation, GPU init, etc.).
    #[error("madori: {0}")]
    Madori(#[from] madori::MadoriError),

    /// `AppState::init` returned an error.
    #[error("app init: {0}")]
    AppInit(#[source] anyhow::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn app_init_display_format() {
        let inner = anyhow::anyhow!("config missing");
        let err = AppkitError::AppInit(inner);
        assert_eq!(err.to_string(), "app init: config missing");
    }

    #[test]
    fn app_init_source_preserved() {
        // `#[source]` on AppInit means the inner anyhow::Error must be
        // reachable via Error::source(). Consumers walk the chain when
        // rendering diagnostics; losing the source would erase the root
        // cause.
        let inner = anyhow::anyhow!("root cause");
        let err = AppkitError::AppInit(inner);
        let source = err.source();
        assert!(source.is_some());
        assert!(source.unwrap().to_string().contains("root cause"));
    }

    #[test]
    fn app_init_message_does_not_duplicate_source() {
        // Display format is "app init: <inner>" — the inner appears
        // once. A thiserror typo like `{0:?}{0}` would double it and
        // we'd ship noisy diagnostics.
        let err = AppkitError::AppInit(anyhow::anyhow!("unique-token-9a9a"));
        let rendered = err.to_string();
        let count = rendered.matches("unique-token-9a9a").count();
        assert_eq!(count, 1, "source appears {count}× in `{rendered}`");
    }

    #[test]
    fn variants_are_distinguishable() {
        // Each variant matches a distinct arm — guards against a
        // future lazy `Box<dyn Error>` collapse in this enum.
        let init = AppkitError::AppInit(anyhow::anyhow!("x"));
        match init {
            AppkitError::AppInit(_) => {}
            AppkitError::Madori(_) => panic!("AppInit matched Madori"),
        }
    }

    #[test]
    fn debug_format_includes_variant_name() {
        // `{:?}` on AppkitError is the last-resort diagnostic when a
        // developer hits `unwrap()`. The variant name must be in the
        // output or they're stuck guessing which branch broke.
        let err = AppkitError::AppInit(anyhow::anyhow!("x"));
        let debug = format!("{err:?}");
        assert!(debug.contains("AppInit"), "debug was `{debug}`");
    }
}
