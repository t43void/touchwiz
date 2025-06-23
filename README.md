# TypeMaster

A terminal-first, open-source typing trainer built to take you from 0 WPM to
elite speed — backed by typing science, not gimmicks. Single self-contained
binary, **zero runtime dependencies, zero telemetry, zero network calls.**

> Status: **Milestone 5** — polish & release: launch splash, custom-file import
> (`--file`), ghost racer, audio feedback, a generated man page, GitHub Actions
> CI, and full-screen render tests — on top of the content layer (M4), curriculum
> (M3), TUI (M2), and engine (M1). This rounds out the planned feature set.

## Install

```sh
# From source (requires Rust ≥ 1.88)
cargo build --release
./target/release/typemaster
```

## What works today

- **Complete 0→300 WPM curriculum** — 24 lessons across 4 phases, each with its
  own generated practice text:
  - *Phase 1 · Foundation* — home row, progressive key introduction, the full
    alphabet, bigram drills, the number row, shift/capitals, punctuation.
  - *Phase 2 · Building* — top-200 words, bigram/trigram fluency, sentence flow,
    code mode, rhythm, symbol sprints.
  - *Phase 3 · Speed* — flow state, burst training, word-frequency mastery,
    paragraphs, code at speed, numbers/URLs/emails/paths.
  - *Phase 4 · Elite* — rolling trigram sequences, high-density text, a sustained
    3-minute test, and the peak-performance protocol.
- **Progression & gating** — pass a lesson 3× in a row (meeting its WPM +
  accuracy bar) to unlock the next; progress persists across runs.
- **Full multi-screen TUI**: dashboard, curriculum browser, lesson, results
  (with pass/fail + unlock), keyboard heatmap, progress chart + leaderboard, and
  settings, plus a `?` help overlay.
- **Live typing test** over the 200 most common English words, with a
  three-state typing field (completed / current / upcoming), block cursor, and
  per-character correctness coloring.
- **Keyboard visualization** with per-finger color zones and current-key
  highlight; post-session it recolors as an error/latency heatmap.
- **Finger guide** naming the finger for the upcoming key.
- **Three themes** (`void`, `light`, `monochrome`), cycled with `Ctrl+T`.
- **Ghost racer** — race a cursor moving at your personal-best pace (`g`).
- **Custom file import** — `typemaster --file mycode.rs [--strip-comments]` turns
  any text or code file into a session.
- **Audio feedback** — optional error bell, off by default (`m`).
- **Launch splash** and a generated man page (`typemaster --generate-man <dir>`).
- **Real-time metrics**: net WPM (smoothed over a 10s window), raw WPM,
  accuracy, consistency, and a running clock.
- **Post-session results**: net/raw WPM, accuracy, consistency, errors, slowest
  keys, and most-errored bigrams.
- **Local persistence** (SQLite at `$XDG_DATA_HOME/typemaster/data.db`):
  sessions, per-key and per-bigram stats, spaced-repetition word mastery
  (modified SM-2), personal bests, and daily streaks.
- **Adaptive engine** (modified SM-2 + keybr-style word selection) and a metric
  engine, both covered by unit tests.

### Keys

| Key | Action |
|-----|--------|
| any character | type (in the lesson) |
| `Enter` | select / start / confirm |
| `Tab` `↑` `↓` | move the dashboard selection |
| `Esc` / `q` | back (quit from the dashboard) |
| `Ctrl+R` | restart with fresh text |
| `Ctrl+H` | heatmap · `Ctrl+S` settings |
| `Ctrl+T` | cycle theme |
| `k` / `Ctrl+K` | toggle keyboard view |
| `f` / `Ctrl+F` | toggle finger guide |
| `g` / `Ctrl+G` | toggle ghost racer |
| `m` | toggle audio feedback |
| `?` | keybindings help |
| `Ctrl+C` | quit from anywhere |

## Architecture

Two Rust crates plus a build-time content layer:

- **`engine/`** — pure logic, no I/O: metric formulas, session state machine,
  corpus loading, adaptive SM-2 scheduler, heatmap aggregation. Fully unit-tested.
- **`typemaster/`** — the binary: Ratatui/crossterm TUI, SQLite persistence
  (writes happen off the render thread), CLI.
- **`content/`** — TypeScript authoring layer. Corpora are **embedded into the
  binary at compile time** (`rust-embed`), so the runtime never needs Node or
  network access. This preserves the zero-runtime-dependency guarantee.

### Content authoring

Corpora live in `content/src/corpus/*.json` and are embedded into the binary at
compile time, so the runtime needs no Node and no network. The TypeScript CLI
(build/dev-time only) helps contributors author and validate them:

```sh
cd content
node src/cli.ts validate          # validate every corpus
node src/cli.ts list              # list corpora with stats
node src/cli.ts add swahili_200 --lang sw --kind words --from words.txt --dedupe
node src/cli.ts watch             # re-validate on every save
```

Live-edit corpora without recompiling by overriding the embedded copies:

```sh
TYPEMASTER_CONTENT_DIR=content/src/corpus cargo run
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full guide.

### Quality bar

`cargo test`, `cargo clippy -- -D warnings`, and `cargo fmt --check` all pass.
Use `just ci` (or the `cargo` aliases in `.cargo/config.toml`) to run the gate.

## Roadmap

- **M2** — ✅ Full TUI: dashboard, heatmap, progress chart, settings;
  keyboard-viz widget with finger zones; `light` + `monochrome` themes;
  finger guide.
- **M3** — ✅ The complete 0→300 WPM curriculum (4 phases, 24 lessons),
  progression gating, and persisted progress.
- **M4** — ✅ TypeScript content-authoring CLI (validate/list/add/watch) and
  dev-time hot-reload via `TYPEMASTER_CONTENT_DIR`.
- **M5** — ✅ Polish & release: splash, ghost racer, custom-file import
  (`--file`), audio feedback, man page, GitHub Actions CI, render tests.

## License

MIT — see [LICENSE](LICENSE).
