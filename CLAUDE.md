# CLAUDE.md

Terminal breathing pacer with flower TUI visualization.

## Build & Run

```bash
cargo build --release
cp target/release/breathe.exe ~/bin/breathe.exe  # Windows Application Control blocks .cargo/bin
cargo run                                         # dev mode, opens interactive menu
cargo run -- calm -r 3                            # direct mode, 3 rounds of calm
cargo clippy --all-targets                        # lint (must stay clean)
```

## Architecture

Five source files, no modules beyond the crate root:

- `main.rs` — CLI (clap), terminal setup/teardown, panic hook, session/menu loop. Two modes: direct (subcommand) runs once and exits with meaningful exit codes (0=completed, 1=interrupted, 2=error); interactive (no args) loops between menu and sessions.
- `pattern.rs` — `Phase`, `Pattern`, `Preset` enum. Presets: calm (4:8), coherent (5.5:5.5), sigh (2:1:6), box (4:4:4:4), energize (1.5:1). Custom ratios via `parse_custom("4:7:8")`.
- `menu.rs` — Interactive preset picker. j/k/arrows navigate, 1-5 jump, left/right adjust rounds, enter starts. Shows estimated session duration.
- `ui.rs` — Session TUI. Flower rendered as polar rose (6 petals, organic variation, distance-based shading). State machine: lead-in (pulsing seed) → breathing (expand/contract with eased fill_level) → closing (shrink to resting) → done. Color blends between phases (blue inhale, amber exhale, sage hold). Rotation speed varies by phase.
- `session.rs` — JSONL session log at `%LOCALAPPDATA%/breathe/sessions.jsonl`. `breathe log` shows recent sessions with local timestamps.

## Key Design Decisions

- **Flower, not rectangle** — organic polar rose with `cos(angle * 6)` petal pattern. Petals have slow angle-dependent variation (`sin(angle * 1.3 + time * 0.07)`) so they're never perfectly symmetric.
- **Two-level shading** — only `█` and `░`, color gradient does the work. Avoids visible banding from too many character types.
- **Warm amber exhale** — blue/amber pair chosen for colorblind accessibility (deuteranopia-safe) and parasympathetic association (warmth = release).
- **Closing animation** — flower shrinks to 8% resting size over 1.5s instead of vanishing. Done screen keeps the resting flower drifting.
- **Lead-in pulse** — seed throbs at ~1Hz for 3 seconds before first inhale. Somatic priming, not cognitive countdown.
- **Exit codes** — 0=completed, 1=interrupted, 2=error. SCT principle.
- **Install to ~/bin/** — Windows Application Control blocks executables in .cargo/bin. Copy release binary to C:\Users\Trist\bin\ instead.

## Conventions

- Clippy must pass with zero warnings.
- All input handling uses let-chains (`if event::poll()? && let Event::Key(key) = ...`).
- Colors defined as `(u8, u8, u8)` tuples, converted to `Color::Rgb` at render time for lerp support.
- `fill_level` (0.0-1.0) drives all visual scaling. Eased with cubic ease-in-out.
- Rotation accumulated incrementally in `tick()`, never computed from total_elapsed.
