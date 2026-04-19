//! Error types for appkit.

use thiserror::Error;

/// Errors surfaced by the appkit bootstrap layer.
///
/// User-level errors from `AppState::init` return `anyhow::Error` and are
/// propagated as-is. `AppkitError` covers only the framework seam.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AppkitError {
    /// shikumi config loading failed.
    #[error("config load failed: {0}")]
    Config(#[from] shikumi::ShikumiError),

    /// madori event loop returned an error (window creation, GPU init, etc.).
    #[error("madori: {0}")]
    Madori(#[from] madori::MadoriError),

    /// `AppState::init` returned an error.
    #[error("app init: {0}")]
    AppInit(#[source] anyhow::Error),
}
