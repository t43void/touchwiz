# touchwiz

Terminal typing trainer. One binary. Zero telemetry, zero network, zero
nonsense.

Get from hunt-and-peck to uncomfortably fast without leaving the shell.
Science-backed drills, not a flashy website pretending to care about your WPM.

## Install

```sh
curl -sSL https://raw.githubusercontent.com/t43void/touchwiz/main/install.sh | bash
```

Drops `touchwiz` into `~/.local/bin`. Override with `PREFIX=/somewhere ./install.sh`.

From source (Rust ≥ 1.88):

```sh
cargo build --release
./target/release/touchwiz
```

## What you get

- Full 0→300 WPM curriculum (24 lessons, gated progression)
- Live metrics: net/raw WPM, accuracy, consistency
- Keyboard heatmap, finger guide, ghost racer
- Themes, optional audio, custom file import (`--file`)
- Local SQLite only — your data stays on your machine

## Keys

| Key | Action |
|-----|--------|
| `Enter` | select / start |
| `Tab` `↑` `↓` | move selection |
| `Esc` / `q` | back (quit from dashboard) |
| `Ctrl+R` | restart |
| `Ctrl+T` | cycle theme |
| `g` | ghost racer |
| `m` / `Ctrl+M` | audio toggle |
| `Ctrl+X` | reset progress (settings, twice) |
| `?` | help |
| `Ctrl+C` | quit |

More contributor notes live in [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT — see [LICENSE](LICENSE).
