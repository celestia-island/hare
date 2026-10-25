set shell := ["bash", "-c"]
# Windows: PowerShell (the 5.1 floor ships with every Windows; pwsh 7 is
# NOT assumed). Linewise recipes must stay PS-5.1-safe: no `&&` chains,
# `cd X; cmd` instead of `cd X && cmd`. Bash-only recipes use
# [script('bash')] and need Git Bash (or WSL) when actually run.
set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command", "[Console]::OutputEncoding=[System.Text.Encoding]::UTF8; $PSDefaultParameterValues['*:Encoding']='utf8';"]
set unstable
set lists

# Repo definitions override the shared template's (imported below).
set allow-duplicate-recipes
set allow-duplicate-variables

# Shared celestia-devtools recipes — NOT in git. Stage with: just fetch.
# `import?` silently skips when absent, so this justfile parses pre-fetch.
import? "./.just/celestia-devtools.just"

default:
    @just --list

# ── hare (no_std bridge library) ─────────────────────────────────────────
# The workspace root is the library and its only member is `qemu-mps2/`, a
# cortex-m demo. Cargo's default members are the root package, so these
# recipes mirror `ci.yml` and never drag the demo into a host build.

check:
    cargo check --all-features --all-targets

test:
    cargo test --all-features --tests

fmt:
    just fmt-toml
    just fmt-markdown .
    cargo fmt --all

fmt-check:
    just fmt-markdown . --check
    cargo fmt --all -- --check

clippy:
    cargo clippy --all-features --all-targets -- -D warnings

bench-check:
    cargo bench --no-run

ci: fmt-check clippy test
