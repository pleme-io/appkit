# Appkit

Shared GPU application bootstrap for pleme-io. Extracts the ~800 LOC of
boilerplate that every GPU app (mado, hibiki, kagi, fumi, nami, hikyaku,
tobirato, ayatsuri, hikki) was copying into its own `main.rs`:

- config loading through shikumi
- `madori::App::builder` wiring
- event-handler closure dispatch to typed hook methods
- window title / size / vsync plumbing
- MCP subcommand runtime setup (feature-gated)

## Usage

```rust
use appkit::{AppState, EventResponse, RunOutcome};
use appkit::madori::{KeyEvent, RenderCallback, RenderContext};
use serde::Deserialize;

#[derive(Default, Deserialize, Clone)]
struct Config { title: Option<String> }

struct App { title: String }

impl RenderCallback for App {
    fn render(&mut self, _ctx: &mut RenderContext) { /* draw */ }
}

impl AppState for App {
    type Config = Config;

    fn init(cfg: Config) -> anyhow::Result<Self> {
        Ok(Self { title: cfg.title.unwrap_or_else(|| "myapp".into()) })
    }

    fn window_title(&self) -> String { self.title.clone() }

    fn on_key(&mut self, event: &KeyEvent) -> EventResponse {
        // handle input
        EventResponse::default()
    }
}

fn main() -> anyhow::Result<()> {
    let cfg = appkit::load_config::<Config>("myapp", None)?;
    appkit::run_gpu_app::<App>(cfg)?;
    Ok(())
}
```

## What appkit owns

| Concern | Location |
|---------|----------|
| Config discovery + loading | `appkit::load_config` (delegates to shikumi) |
| Event loop + window creation | `appkit::run_gpu_app` (delegates to madori) |
| `AppEvent` → typed hook dispatch | `appkit::AppState::on_key` / `on_redraw` / `on_close` / `on_resize` |
| Window metadata (title, size, vsync, transparent, resizable) | `AppState` accessor defaults |
| MCP subcommand async runtime | `appkit::mcp::run_subcommand` (behind the `mcp` feature) |

## What appkit deliberately does NOT own

- GPU primitives → use `garasu` directly
- Widget state → use `egaku`
- Theme generation → use `irodori` + `irodzuki`
- HTTP clients → use `todoku`
- MCP server scaffold → use `kaname` directly; `appkit::mcp` is only the runtime shim

## Features

- `mcp` — enable `appkit::mcp::run_subcommand` for apps that dispatch an
  MCP stdio server subcommand. Pulls in tokio with `rt` + `macros`.

## License

MIT
