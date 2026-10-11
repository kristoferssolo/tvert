export RUST_LOG := env("RUST_LOG", "tvert=debug,tvert_wayland=debug")

set shell := ["bash", "-euo", "pipefail", "-c"]

alias a := audit
alias b := build
alias c := check
alias f := fmt
alias i := install
alias r := run
alias rr := run-release
alias t := test

# List available recipes
default:
    @just --list

# Run local pre-commit checks, formatting the workspace first
[group("checks")]
ci: fmt clippy test doc

# Run the same checks as CI without modifying files
[group("checks")]
check: fmt-check clippy test doc

# Format the workspace
[group("checks")]
fmt:
    cargo fmt --all

# Fail if anything is unformatted
[group("checks")]
fmt-check:
    cargo fmt --all -- --check

# Lint all workspace targets
[group("checks")]
clippy:
    cargo clippy --workspace --all-features --all-targets -- --deny warnings

# Run the test suite
[group("checks")]
test *ARGS:
    cargo nextest run --workspace --all-features {{ ARGS }}

# Build documentation, including private items
[group("checks")]
doc *ARGS:
    RUSTDOCFLAGS="-D warnings" cargo doc \
        --workspace \
        --all-features \
        --document-private-items \
        --no-deps \
        {{ ARGS }}

# Run Tvert
[group("run")]
run *ARGS:
    cargo run --package tvert-cli -- {{ ARGS }}

# Run an optimized build of Tvert
[group("run")]
run-release *ARGS:
    cargo run --release --package tvert-cli -- {{ ARGS }}

# Build the entire workspace
[group("build")]
build:
    cargo build --workspace --all-features

# Build the entire workspace in release mode
[group("build")]
build-release:
    cargo build --release --workspace --all-features

# Install Tvert from this checkout
[group("build")]
install:
    cargo install --path crates/tvert-cli --locked

# Watch the workspace during development
[group("development")]
watch:
    bacon

# Run benchmarks
[group("development")]
bench *ARGS:
    cargo bench --workspace -- {{ ARGS }}

# Show dependency updates without modifying Cargo.lock
[group("dependencies")]
outdated:
    cargo update --dry-run --verbose

# Update dependencies within Cargo.toml constraints
[group("dependencies")]
update:
    cargo update

# Audit dependencies for known advisories
[group("dependencies")]
audit:
    cargo audit

# Remove build artifacts
[group("maintenance")]
clean:
    cargo clean

# Install development tools used by this Justfile
[group("maintenance")]
setup:
    cargo install cargo-nextest bacon cargo-audit
