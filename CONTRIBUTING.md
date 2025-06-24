# Contributing to TypeMaster

Thanks for helping build the best terminal typing trainer! Contributions of new
**corpora** (especially non-English) are particularly welcome.

## Quality gates

All of these must pass before a PR is merged (they run in CI):

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all --check
cd content && node --test          # TypeScript content tests
```

`just ci` runs the Rust gate in one shot.

## Adding a corpus (no Rust required)

Corpora live in `content/src/corpus/<name>.json` and are embedded into the binary
at compile time. The shape is:

```json
{
  "name": "swahili_200",
  "language": "sw",
  "kind": "words",
  "entries": ["na", "ya", "kwa", "..."]
}
```

- `name` **must** match the file name (without `.json`).
- `kind` is one of `words`, `sentences`, or `code`.
- `entries` is a non-empty list of strings. For `words`, one word per entry; for
  `sentences`/`code`, one full line per entry.

You can generate a corpus from a plain-text file with the content CLI:

```sh
cd content
node src/cli.ts add swahili_200 --lang sw --kind words --from words.txt --dedupe --lowercase
node src/cli.ts validate          # validate every corpus
node src/cli.ts list              # show all corpora with stats
```

### Live-editing without recompiling

Point the binary at your corpus directory to override the embedded copies while
you iterate — no rebuild needed between sessions:

```sh
TYPEMASTER_CONTENT_DIR=content/src/corpus cargo run
```

Run `node src/cli.ts watch` in another terminal to re-validate on every save.
When you're happy, rebuild (`cargo build`) to embed the final files.

To wire a new corpus into the curriculum, reference its file name from a lesson's
`Drill::Corpus { asset, .. }` in `engine/src/curriculum/`.

## Adding a theme

Add a `Theme` constant in `typemaster/src/themes.rs` following the existing
pattern, then add it to the `THEMES` array so `Ctrl+T` cycles to it.

## Adding a lesson

Add a `Lesson` to the appropriate phase module in `engine/src/curriculum/`
(`beginner.rs`, `intermediate.rs`, `advanced.rs`, `elite.rs`) using the `lesson(..)`
helper and a `Drill`, then keep the curriculum test (`every_lesson_generates_*`)
green.

## Code style

- Rust: `cargo fmt` + `clippy` clean; no `unwrap()` in non-test code; document
  public items with `///`.
- TypeScript: no runtime dependencies; the content layer is build/dev-time only.
