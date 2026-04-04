# breathe

Terminal breathing pacer with flower TUI visualization.

A blooming flower guides your breath. Inhale — it expands in cool blue. Exhale — it contracts in warm amber. Petals shift organically; colors deepen as you settle in.

## Install

```
cargo install --path .
```

Or run directly:

```
cargo run --release
```

## Usage

```
breathe              # interactive menu
breathe calm         # extended exhale (4:8), parasympathetic activation
breathe coherent     # HRV resonance (5.5:5.5)
breathe sigh         # physiological sigh (2:1:6)
breathe box          # balanced (4:4:4:4)
breathe energize     # rapid power breathing (1.5:1)
breathe custom 4:7:8 # custom ratio (inhale:hold:exhale:hold)
breathe log          # recent sessions
```

### Options

```
-r, --rounds <N>     # number of rounds (default varies by preset)
--bell               # terminal bell on phase transitions
```

### Controls

| Key | Action |
|-----|--------|
| `space` | pause/resume |
| `b` | toggle bell |
| `q` | quit |

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Session completed |
| 1 | Session interrupted |
| 2 | Error |

## Session log

Sessions are logged to `%LOCALAPPDATA%/breathe/sessions.jsonl`. View with `breathe log`.

Pipe to get JSON: `breathe calm -r 3 2>/dev/null | jq`

## License

MIT
