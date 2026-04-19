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
