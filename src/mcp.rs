//! MCP subcommand boilerplate. Feature-gated on `mcp`.
//!
//! Six GPU apps (mado, hibiki, kagi, hikyaku, nami, fumi) repeat the same
//! four-line pattern when the CLI receives the `mcp` subcommand: build a
//! tokio runtime, `block_on` an async entry point, convert errors to
//! `anyhow`. Collapse into one helper.
//!
//! Apps don't have to depend on this module — they can still use kaname
//! directly. But sharing the runtime-creation shape means a single point
//! of change when we want (e.g.) `#[tokio::main(flavor = "current_thread")]`
//! vs multi-threaded.
//!
//! # Usage
//!
//! Enable the `mcp` feature in Cargo.toml:
//!
//! ```toml
//! appkit = { version = "0.1", features = ["mcp"] }
//! ```
//!
//! Then:
//!
//! ```no_run
//! # #[cfg(feature = "mcp")]
//! fn run_mcp() -> anyhow::Result<()> {
//!     appkit::mcp::run_subcommand(async {
//!         // your kaname::run(...) call goes here
//!         Ok(())
//!     })
//! }
//! ```

/// Run an async MCP entry point on a single-threaded tokio runtime.
///
/// Every consumer that was open-coding this pattern built a
/// `tokio::runtime::Builder::new_current_thread().enable_all().build()` —
/// we do the same here. The single-threaded runtime is sufficient for MCP
/// stdio servers, which are inherently serial I/O.
///
/// # Errors
///
/// Propagates the future's error unchanged. Tokio runtime construction
/// errors (extremely rare — only on a misconfigured OS) map to `anyhow`.
pub fn run_subcommand<F>(future: F) -> anyhow::Result<()>
where
    F: std::future::Future<Output = anyhow::Result<()>>,
{
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| anyhow::anyhow!("tokio runtime init failed: {e}"))?;
    runtime.block_on(future)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_subcommand_returns_future_value() {
        let result = run_subcommand(async { Ok(()) });
        assert!(result.is_ok());
    }

    #[test]
    fn run_subcommand_propagates_error() {
        let result = run_subcommand(async { Err::<(), _>(anyhow::anyhow!("boom")) });
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("boom"));
    }

    #[test]
    fn run_subcommand_drives_async_work() {
        // Verify we actually block_on and poll the future to completion
        // rather than just dropping it.
        let result = run_subcommand(async {
            let mut counter = 0;
            for _ in 0..3 {
                tokio::task::yield_now().await;
                counter += 1;
            }
            assert_eq!(counter, 3);
            Ok(())
        });
        assert!(result.is_ok());
    }
}
