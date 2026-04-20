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
//! // Clone is required because shidou's config loader returns T by value,
//! // and Send + Sync because it may be held across async boundaries.
//! #[derive(Default, Clone, Deserialize)]
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
//!     let cfg = appkit::load_config::<MyConfig>("myapp", None);
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

#[cfg(test)]
mod tests {
    //! Coverage for `AppState`'s default-method contract.
    //!
    //! Nine consumer apps (mado, hibiki, kagi, fumi, nami, hikyaku,
    //! tobirato, ayatsuri, hikki) rely on the trait's defaults to
    //! avoid boilerplate. If a default ever changes silently (`vsync`
    //! flipping to false, `window_size` shrinking, `on_close` starting
    //! to veto by default) every app regresses. These tests pin the
    //! contract.
    use super::*;
    // `madori` names collide between our re-export module and the
    // underlying crate — fully-qualify with `::madori` for the types
    // we need here.
    use ::madori::event::{KeyCode, Modifiers};
    use ::madori::RenderCallback;

    /// Bare-bones AppState that overrides only the two required
    /// accessors (`init`, `window_title`). Every other method falls
    /// through to its default so we can observe it.
    struct BareState;

    #[derive(Default, serde::Deserialize, Clone)]
    struct BareConfig {}

    impl RenderCallback for BareState {
        fn render(&mut self, _ctx: &mut RenderContext) {}
    }

    impl AppState for BareState {
        type Config = BareConfig;

        fn init(_: Self::Config) -> anyhow::Result<Self> {
            Ok(BareState)
        }

        fn window_title(&self) -> String {
            "bare".into()
        }
    }

    #[test]
    fn default_window_size_is_720p_landscape() {
        // 1280×720 is the baseline — apps that need something else
        // override (hibiki uses 900×600, for instance). If this
        // default drifts, every app-with-no-override window opens
        // differently.
        assert_eq!(BareState.window_size(), (1280, 720));
    }

    #[test]
    fn default_resizable_is_true() {
        // Flipping this to false would make launcher-style apps
        // (tobirato) suddenly refuse user resizing.
        assert!(BareState.resizable());
    }

    #[test]
    fn default_vsync_is_on() {
        // vsync-on is the battery-friendly default; games / real-time
        // visualisers opt out. A regression to false would spike power
        // draw on every laptop running a pleme-io app.
        assert!(BareState.vsync());
    }

    #[test]
    fn default_transparent_is_false() {
        // Opaque by default. Transparent windows need extra compositor
        // setup and aren't what most apps want at launch.
        assert!(!BareState.transparent());
    }

    #[test]
    fn default_on_key_returns_default_event_response() {
        // Default `on_key` must return a non-consumed, non-exit
        // response so apps that don't care about keys let madori's
        // own handling proceed.
        let ev = KeyEvent {
            key: KeyCode::Escape,
            pressed: true,
            modifiers: Modifiers::default(),
            text: None,
        };
        let mut s = BareState;
        let resp = s.on_key(&ev);
        assert!(!resp.consumed);
        assert!(!resp.exit);
    }

    #[test]
    fn default_on_redraw_does_not_exit() {
        // on_redraw fires every frame; if the default ever started
        // returning `exit: true`, every app would close on first paint.
        let mut s = BareState;
        let resp = s.on_redraw();
        assert!(!resp.exit);
    }

    #[test]
    fn default_on_close_does_not_veto() {
        // `on_close` defaults to non-consumed so madori exits normally
        // when the user clicks the window close button. A flip to
        // consumed=true would trap users inside the app.
        let mut s = BareState;
        let resp = s.on_close();
        assert!(!resp.consumed, "default on_close must not veto close");
    }

    #[test]
    fn default_on_resize_is_noop() {
        // The contract is: `on_resize` is called but the actual GPU
        // viewport adjustment happens via `RenderCallback::resize`.
        // If the default ever started touching state, apps that rely
        // on the separation would break. Nothing to assert beyond
        // "it didn't panic" — BareState has no mutable state.
        let mut s = BareState;
        s.on_resize(640, 480);
    }

    #[test]
    fn required_window_title_is_exposed_by_override() {
        // Sanity check that the `window_title` override is actually
        // called by the trait contract — a typo in the trait would
        // silently fall back to a default (if we had one) instead of
        // using BareState's value.
        assert_eq!(BareState.window_title(), "bare");
    }

    #[test]
    fn init_consumes_default_config() {
        // `Config: Default` means consumers can call `init(Default::default())`
        // in tests without constructing a full config. Fully qualify
        // the trait so the call doesn't collide with
        // `RenderCallback::init`.
        let cfg = BareConfig::default();
        assert!(<BareState as AppState>::init(cfg).is_ok());
    }

    #[test]
    fn config_bound_allows_clone_and_send_sync() {
        // The `AppState::Config` bound is Deserialize + Default +
        // Clone + Send + Sync. We exercise each at compile time:
        // if the bound ever loses one, this test fails to build.
        fn needs_full_bound<T: serde::de::DeserializeOwned + Default + Clone + Send + Sync>() {}
        needs_full_bound::<BareConfig>();
    }
}
