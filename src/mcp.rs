//! MCP subcommand boilerplate — thin facade over `shidou::runtime`.
//!
//! Six GPU apps (mado, hibiki, kagi, hikyaku, nami, fumi) repeat the same
//! four-line pattern when the CLI receives the `mcp` subcommand: build a
//! current-thread tokio runtime, `block_on` an async entry point, convert
//! errors to `anyhow`. Collapse into one helper delegating to shidou so
//! we don't carry a second runtime-construction implementation.
//!
//! Apps don't have to depend on this module — they can still use
//! `shidou::block_on_current_thread` directly. But routing through appkit
//! means the whole GPU-app layer has one import path (`appkit::mcp`).
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
/// Delegates to `shidou::runtime::block_on_current_thread`, then flattens
/// the nested `Result<Result<()>, _>` that shidou returns: the outer
/// `anyhow::Result` covers runtime-construction failure (extremely rare),
/// the inner is the future's own result. Consumers see a single
/// `anyhow::Result<()>`.
///
/// # Errors
///
/// Propagates the future's error unchanged. Tokio runtime construction
/// errors (extremely rare) map to `anyhow` via shidou.
pub fn run_subcommand<F>(future: F) -> anyhow::Result<()>
where
    F: std::future::Future<Output = anyhow::Result<()>>,
{
    shidou::runtime::block_on_current_thread(future)?
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
        // Verify shidou's runtime actually polls the future to completion
        // rather than just dropping it. Avoid direct tokio imports here —
        // appkit doesn't depend on tokio; shidou brings it transitively
        // for the runtime but we don't touch tokio types at the appkit seam.
        use std::future::Future;
        use std::pin::Pin;
        use std::task::{Context, Poll};

        /// A future that yields once to force the runtime to re-poll.
        struct YieldOnce {
            yielded: bool,
        }
        impl Future for YieldOnce {
            type Output = ();
            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
                if self.yielded {
                    Poll::Ready(())
                } else {
                    self.yielded = true;
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            }
        }

        let result = run_subcommand(async {
            let mut counter = 0;
            for _ in 0..3 {
                YieldOnce { yielded: false }.await;
                counter += 1;
            }
            assert_eq!(counter, 3);
            Ok(())
        });
        assert!(result.is_ok());
    }
}
