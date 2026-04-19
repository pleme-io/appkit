//! Appkit — shared GPU app bootstrap for pleme-io.
//!
//! Extracts the ~800 LOC of boilerplate that every GPU app (mado, hibiki,
//! kagi, fumi, nami, hikyaku, tobirato, ayatsuri, hikki) was copying:
//!
//! - config loading through shikumi
//! - madori `App::builder` wiring
//! - event-handler closure dispatch to typed hook methods
//! - window title / size / vsync wiring
//!
//! # Usage
//!
//! Implement [`AppState`] on the application's render+state type, then call
//! [`run_gpu_app`] with a config value:
//!
//! ```no_run
//! use appkit::{AppState, EventResponse, RunOutcome};
//! use appkit::madori::{AppEvent, KeyEvent, RenderContext};
//! use serde::Deserialize;
//!
//! #[derive(Default, Deserialize, Clone)]
//! struct MyConfig { title: Option<String> }
//!
//! struct MyApp { title: String }
//!
//! impl appkit::madori::RenderCallback for MyApp {
//!     fn render(&mut self, _ctx: &mut RenderContext) {}
//! }
//!
//! impl AppState for MyApp {
//!     type Config = MyConfig;
//!
//!     fn init(cfg: Self::Config) -> anyhow::Result<Self> {
//!         Ok(Self { title: cfg.title.unwrap_or_else(|| "myapp".into()) })
//!     }
//!
//!     fn window_title(&self) -> String { self.title.clone() }
//! }
//!
//! fn main() -> anyhow::Result<()> {
//!     let cfg = appkit::load_config::<MyConfig>("myapp", None)?;
//!     let _ = appkit::run_gpu_app::<MyApp>(cfg)?;
//!     Ok(())
//! }
//! ```
//!
//! # What appkit deliberately does NOT own
//!
//! - GPU primitives (use `garasu` directly)
//! - Widget state (use `egaku`)
//! - Theme generation (use `irodori` + `irodzuki`)
//! - HTTP clients (use `todoku`)
//! - MCP server scaffold (use `kaname`; appkit exposes
//!   [`mcp_subcommand_async`] only for the common "if CLI asked for MCP,
//!   start a tokio runtime and run a `kaname` handler" shim)

pub mod app;
pub mod config;
pub mod error;

#[cfg(feature = "mcp")]
pub mod mcp;

/// Re-exports so consumers can depend on a single `appkit` version and get
/// matching `madori` types without a direct dep.
pub mod madori {
    pub use madori::{
        App, AppBuilder, AppConfig, AppEvent, EventResponse, ImeEvent, InputEvent, KeyEvent,
        MouseEvent, RenderCallback, RenderContext,
    };
}

pub use app::{RunOutcome, run_gpu_app};
pub use config::{LoadConfig, load_config};
pub use error::AppkitError;
pub use madori::{AppEvent, EventResponse, KeyEvent, RenderCallback, RenderContext};

/// Shared application surface: init from config, expose renderer, handle events.
///
/// A type implementing `AppState` is *both* the render callback (via the
/// [`RenderCallback`] supertrait) *and* the event handler — a single object
/// owns GPU state and input dispatch. This matches how every consuming app
/// was already organised; appkit just formalises the shape.
///
/// Default implementations of the event hooks return [`EventResponse::default`]
/// (non-consumed). Apps override only what they care about.
pub trait AppState: RenderCallback + 'static {
    /// App-specific config, loaded before `init` via shikumi.
    type Config: serde::de::DeserializeOwned + Default + Clone + Send + Sync;

    /// Construct the app from its loaded config. Runs before the event loop.
    ///
    /// # Errors
    ///
    /// Any error surfaced here is propagated to [`run_gpu_app`] and then to
    /// the caller — typically `main` returns it for the process to exit
    /// non-zero with the diagnostic.
    fn init(config: Self::Config) -> anyhow::Result<Self>
    where
        Self: Sized;

    /// Window title. Called at startup; not live-updated.
    fn window_title(&self) -> String;

    /// Initial window size. Defaults to 1280×720 — override for apps that
    /// need a different starting geometry (e.g. hibiki uses 900×600).
    fn window_size(&self) -> (u32, u32) {
        (1280, 720)
    }

    /// Whether the window is resizable. Defaults to true.
    fn resizable(&self) -> bool {
        true
    }

    /// Whether to enable vsync. Defaults to true (battery-friendly); turn
    /// off for low-latency apps (games, real-time visualizers).
    fn vsync(&self) -> bool {
        true
    }

    /// Whether the window background is transparent. Defaults to false.
    fn transparent(&self) -> bool {
        false
    }

    /// Handle a key event. Default: not consumed.
    #[must_use]
    fn on_key(&mut self, _event: &KeyEvent) -> EventResponse {
        EventResponse::default()
    }

    /// Handle per-frame redraw. Default: not consumed. Apps use this hook
    /// to tick animation state or update the title with FPS info.
    #[must_use]
    fn on_redraw(&mut self) -> EventResponse {
        EventResponse::default()
    }

    /// Handle the window-close request. Default: not consumed, which lets
    /// madori exit. Return a consumed response to veto the close
    /// (e.g. prompt "unsaved changes").
    #[must_use]
    fn on_close(&mut self) -> EventResponse {
        EventResponse::default()
    }

    /// Handle window resize. Default: no-op. The `RenderCallback::resize`
    /// contract is called separately by madori.
    fn on_resize(&mut self, _width: u32, _height: u32) {}
}
