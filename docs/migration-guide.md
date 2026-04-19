# Migrating a GPU app to appkit

This guide walks through porting an existing GPU app (e.g. hikki, fumi,
kagi, hibiki, nami, mado, tobirato, ayatsuri) onto appkit's `AppState`
trait + `run_gpu_app` bootstrap.

Before starting, check that the app today imports `madori` directly and
has a main.rs with a `madori::App::builder(renderer).on_event(…).run()`
shape. That's what appkit replaces.

## What appkit replaces vs. preserves

| Concern | Before | After |
|---|---|---|
| `fn main`: tracing init, clap parse | hand-rolled | preserved; call `appkit::load_config` for config |
| `madori::App::builder` wiring | hand-rolled per app | `appkit::run_gpu_app::<S>(config)` |
| `.on_event(closure)` with big match | per-app | appkit dispatches to typed hooks |
| Config loading via shikumi | per-app | `appkit::load_config` |
| MCP subcommand tokio runtime | per-app | `appkit::mcp::run_subcommand` (with `mcp` feature) |
| GPU state, renderer, widgets | per-app | untouched — `AppState: RenderCallback` |
| Subcommands other than `mcp` (search, list, …) | per-app | untouched — clap owns these |

## Step-by-step

### 1. Add appkit to Cargo.toml

```toml
[dependencies]
appkit = { git = "https://github.com/pleme-io/appkit", features = ["mcp"] }
# madori stays — appkit doesn't hide it; it re-exports common types under
# `appkit::madori::{AppEvent, KeyEvent, RenderCallback, RenderContext, …}`
# so you can also drop the direct madori dep if the only usage was events.
```

### 2. Implement `AppState` on your render struct

Find the struct that carries your render state (often called `XyzRenderer`
or `XyzApp`). It already implements `madori::RenderCallback`.

```rust
use appkit::{AppState, EventResponse};
use appkit::madori::KeyEvent;

impl AppState for MyRenderer {
    type Config = MyAppConfig;

    fn init(config: Self::Config) -> anyhow::Result<Self> {
        // Whatever your old `run_gui(config)` function did to build the
        // renderer, move here.
        Ok(MyRenderer::new(&config.appearance.background, ...))
    }

    fn window_title(&self) -> String {
        "myapp".into()
    }

    fn window_size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    fn on_key(&mut self, event: &KeyEvent) -> EventResponse {
        // Everything that was in the `AppEvent::Key(k) => { ... }` arm
        // of your old .on_event closure goes here.
        if event.pressed && event.key == appkit::madori::KeyCode::Escape {
            return EventResponse { exit: true, ..Default::default() };
        }
        EventResponse::default()
    }

    fn on_redraw(&mut self) -> EventResponse {
        // Content from `AppEvent::RedrawRequested => { ... }` goes here.
        EventResponse::default()
    }
}
```

### 3. Replace the GUI entry point

Old:

```rust
fn run_gui(config: MyAppConfig) -> anyhow::Result<()> {
    let renderer = MyRenderer::new(...);
    let mut state = MyState::new();
    madori::App::builder(renderer)
        .title("myapp")
        .size(1280, 720)
        .on_event(move |event, renderer| match event {
            madori::AppEvent::Key(k) => state.handle_key(k, renderer),
            madori::AppEvent::RedrawRequested => state.tick(renderer),
            // …
        })
        .run()?;
    Ok(())
}
```

New:

```rust
fn run_gui(config: MyAppConfig) -> anyhow::Result<()> {
    let _ = appkit::run_gpu_app::<MyRenderer>(config)?;
    Ok(())
}
```

If your "state" was a separate object from the renderer, merge them into
one struct that implements both `RenderCallback` and `AppState`.

### 4. Swap the MCP subcommand

Old (repeated across six apps):

```rust
Some(Commands::Mcp) => {
    let rt = shidou::create_runtime()?;
    rt.block_on(mcp::run())
        .map_err(|e| anyhow::anyhow!("MCP server error: {e}"))?;
}
```

New:

```rust
Some(Commands::Mcp) => {
    appkit::mcp::run_subcommand(async {
        mcp::run().await.map_err(|e| anyhow::anyhow!("MCP server error: {e}"))
    })?;
}
```

(Requires the `mcp` feature on appkit.)

### 5. Swap config loading

If your app used shikumi directly:

Old:
```rust
let config: MyConfig = shidou::load_config("myapp");
// or
let path = ConfigDiscovery::new("myapp").discover()?;
let store = ConfigStore::<MyConfig>::load(&path, "MYAPP_")?;
let config = (**store.get()).clone();
```

New:
```rust
let config: MyConfig = appkit::load_config("myapp", cli.config.as_deref())?;
```

`appkit::load_config` wraps shikumi's `ConfigDiscovery` + `ConfigStore`
and honors a CLI `--config <path>` override. If you need live hot-reload
(the `ArcSwap` path) keep using shikumi directly — appkit intentionally
owns only the "load once at startup" flow.

### 6. Verify

Run the test suite (any lib tests that didn't depend on main should be
untouched) and do a smoke launch:

```
cargo build
cargo run                      # GUI mode
cargo run -- mcp               # MCP subcommand
cargo run -- <your-cli-subcommand>
```

Typical wins per app: -200 to -400 LOC from main.rs, one less direct
madori dep (optional — you may still want it for advanced event types),
one less hand-rolled shikumi bootstrap.

## When NOT to migrate

- **Apps that don't use madori at all** (e.g. shirase, which uses tsuuchi
  for notifications and has no GPU surface). Skip.
- **Apps mid-refactor** on their event handler (e.g. ayatsuri's
  Bevy-based wm doesn't fit this pattern). Wait.
- **Apps whose event loop touches internals madori doesn't expose**
  (rare; file a madori issue instead).

## Rollback

The migration is per-file. If a step breaks tests, revert that file with
`git checkout -- path/to/file.rs` and the app returns to its pre-appkit
shape. appkit only mediates the event loop; the rest of the crate is
untouched.
