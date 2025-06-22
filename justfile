# TypeMaster build automation.
# Requires `just` (cargo install just). A no-install fallback exists via
# `.cargo/config.toml` aliases, e.g. `cargo t`, `cargo lint`, `cargo fmtcheck`.

# List available recipes.
default:
    @just --list

# Build the whole workspace (debug).
build:
    cargo build

# Run the binary.
run *ARGS:
    cargo run -p typemaster -- {{ARGS}}

# Run all tests.
test:
    cargo test

# Clippy with warnings denied (Quality rule 14).
lint:
    cargo clippy --all-targets -- -D warnings

# Check formatting (Quality rule 15).
fmt-check:
    cargo fmt --all --check

# Apply formatting.
fmt:
    cargo fmt --all

# Full CI gate: format, lint, test.
ci: fmt-check lint test

# Optimized static release build.
release:
    cargo build --release

# Remove build artifacts.
clean:
    cargo clean
