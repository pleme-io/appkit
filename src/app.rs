//! Event-loop bootstrap: wires [`AppState`](crate::AppState) to madori.

use madori::{App, AppConfig, AppEvent};

use crate::{AppState, EventResponse};

/// Outcome of [`run_gpu_app`].
///
/// `#[non_exhaustive]` so we can add variants (e.g. `Restart`, `Reload`)
/// without breaking consumers that already match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RunOutcome {
    /// The event loop returned normally (window closed, user quit).
    Exited,
}

/// Bootstrap and run a GPU app.
///
/// This replaces the 100-200 LOC main-function pattern that every GPU app
/// in the pleme-io tree was copying:
///
/// 1. Call `S::init(config)` to build the render+state object.
/// 2. Read window metadata (title, size, resizable, vsync, transparent)
///    from the state via the `AppState` accessors.
/// 3. Build a madori `App` and wire an event closure that dispatches to
///    [`AppState::on_key`], [`AppState::on_redraw`], [`AppState::on_close`],
///    and [`AppState::on_resize`].
/// 4. Run the event loop until the window closes.
///
/// # Errors
///
/// Any failure from `init` or madori's event loop is surfaced as
/// [`AppkitError`](crate::AppkitError), wrapped in `anyhow::Error` for the
/// convenient `main` signature.
pub fn run_gpu_app<S: AppState>(config: S::Config) -> anyhow::Result<RunOutcome> {
    // Fully qualified: RenderCallback::init(&mut self, &GpuContext) is also
    // in scope via the supertrait bound and would otherwise be ambiguous.
    let state = <S as AppState>::init(config).map_err(crate::AppkitError::AppInit)?;

    let title = state.window_title();
    let (width, height) = state.window_size();
    let resizable = state.resizable();
    let vsync = state.vsync();
    let transparent = state.transparent();

    let madori_config = AppConfig {
        title,
        width,
        height,
        resizable,
        vsync,
        transparent,
    };

    App::builder(state)
        .config(madori_config)
        .on_event(dispatch::<S>)
        .run()
        .map_err(crate::AppkitError::Madori)?;

    Ok(RunOutcome::Exited)
}

/// Translate a madori `AppEvent` into the corresponding `AppState` hook.
///
/// Pulled out of `run_gpu_app` so it's testable in isolation: call
/// `dispatch(&event, &mut state)` and assert the returned `EventResponse`.
/// (Making the closure monomorphic on `S` keeps the signature stable for
/// madori's builder, which requires `FnMut(&AppEvent, &mut S) -> …`.)
pub(crate) fn dispatch<S: AppState>(event: &AppEvent, state: &mut S) -> EventResponse {
    match event {
        AppEvent::Key(k) => state.on_key(k),
        AppEvent::RedrawRequested => state.on_redraw(),
        AppEvent::CloseRequested => state.on_close(),
        AppEvent::Resized { width, height } => {
            state.on_resize(*width, *height);
            EventResponse::default()
        }
        // Mouse, IME, scroll, focus — apps rarely handle these at the
        // appkit layer. Consumers that need them implement `RenderCallback`
        // directly and route via madori's native builder (escape hatch).
        _ => EventResponse::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::madori::{KeyEvent, RenderContext};
    use madori::event::{KeyCode, Modifiers};

    // A minimal AppState for dispatch testing — no GPU required because we
    // only exercise dispatch(), not run_gpu_app (which opens a window).

    #[derive(Default)]
    struct TestState {
        key_hits: u32,
        redraws: u32,
        closes: u32,
        last_resize: Option<(u32, u32)>,
    }

    #[derive(Default, serde::Deserialize, Clone)]
    struct TestConfig {}

    impl madori::RenderCallback for TestState {
        fn render(&mut self, _ctx: &mut RenderContext) {}
    }

    impl AppState for TestState {
        type Config = TestConfig;

        fn init(_: Self::Config) -> anyhow::Result<Self> {
            Ok(Self::default())
        }

        fn window_title(&self) -> String {
            "test".into()
        }

        fn on_key(&mut self, _: &KeyEvent) -> EventResponse {
            self.key_hits += 1;
            EventResponse::default()
        }

        fn on_redraw(&mut self) -> EventResponse {
            self.redraws += 1;
            EventResponse::default()
        }

        fn on_close(&mut self) -> EventResponse {
            self.closes += 1;
            EventResponse::default()
        }

        fn on_resize(&mut self, w: u32, h: u32) {
            self.last_resize = Some((w, h));
        }
    }

    fn key_event() -> AppEvent {
        AppEvent::Key(KeyEvent {
            key: KeyCode::Escape,
            pressed: true,
            modifiers: Modifiers::default(),
            text: None,
        })
    }

    #[test]
    fn dispatch_key_routes_to_on_key() {
        let mut s = TestState::default();
        dispatch(&key_event(), &mut s);
        assert_eq!(s.key_hits, 1);
        assert_eq!(s.redraws, 0);
    }

    #[test]
    fn dispatch_redraw_routes_to_on_redraw() {
        let mut s = TestState::default();
        dispatch(&AppEvent::RedrawRequested, &mut s);
        assert_eq!(s.redraws, 1);
    }

    #[test]
    fn dispatch_close_routes_to_on_close() {
        let mut s = TestState::default();
        dispatch(&AppEvent::CloseRequested, &mut s);
        assert_eq!(s.closes, 1);
    }

    #[test]
    fn dispatch_resize_routes_to_on_resize() {
        let mut s = TestState::default();
        dispatch(
            &AppEvent::Resized {
                width: 640,
                height: 480,
            },
            &mut s,
        );
        assert_eq!(s.last_resize, Some((640, 480)));
    }

    #[test]
    fn dispatch_unknown_event_returns_default() {
        let mut s = TestState::default();
        let resp = dispatch(&AppEvent::Focused(true), &mut s);
        assert!(!resp.consumed);
        assert!(!resp.exit);
        // None of the hooks should have fired
        assert_eq!(s.key_hits, 0);
        assert_eq!(s.redraws, 0);
    }

    #[test]
    fn window_metadata_defaults() {
        let s = TestState::default();
        assert_eq!(s.window_size(), (1280, 720));
        assert!(s.resizable());
        assert!(s.vsync());
        assert!(!s.transparent());
    }

    #[test]
    fn dispatch_many_events_accumulates() {
        let mut s = TestState::default();
        for _ in 0..3 {
            dispatch(&key_event(), &mut s);
        }
        for _ in 0..5 {
            dispatch(&AppEvent::RedrawRequested, &mut s);
        }
        assert_eq!(s.key_hits, 3);
        assert_eq!(s.redraws, 5);
    }

    #[test]
    fn run_outcome_is_copy_eq() {
        let a = RunOutcome::Exited;
        let b = a;
        assert_eq!(a, b);
    }
}
