# zer

Public repo for the `zer` CLI. The review procedure stays the `zereight-review` skill in `skills/zereight-review/`. `zer` does not call a model.

## Layout

- `skills/zereight-review/` — skill source of truth (SKILL.md + references)
- `zer/` — Rust CLI source (`zer manifest`, `zer verify`)
- `schema/` — finding record shape enforced by `zer verify`

## zer

`zer` is the deterministic helper CLI for zereight-review: it builds review manifests from diffs and validates finding line refs. Install once per machine.

### Prerequisites

- git
- Rust 1.74+ with cargo. Check with `cargo --version`. If missing, install via rustup:

    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

  (accept the defaults when prompted), then restart your shell (or run `source "$HOME/.cargo/env"`) and re-run `cargo --version`.

### Install

    git clone https://github.com/zereight/zer.git
    cd zer
    cargo install --path zer --locked

`--locked` pins the exact dependency versions from `zer/Cargo.lock` for reproducible builds.

### Verify

    zer --help

Expected: lists the `manifest` and `verify` subcommands. If you see `zer: command not found`, add cargo binaries to PATH:

    export PATH="$HOME/.cargo/bin:$PATH"

and persist it in `~/.zshrc` (macOS default) or `~/.bashrc`, then open a new terminal (or `source` the file).

### Usage

    # 1. Build the manifest from a three-dot diff (range: base...HEAD)
    zer manifest --repo /path/to/repo --base origin/main --out /tmp/manifest.json

    # 2. Verify finding line refs (findings follow schema/finding.schema.json)
    zer verify --manifest /tmp/manifest.json --findings /tmp/findings.json --out /tmp/verdict.json

### Update / uninstall

    git pull && cargo install --path zer --locked   # update (from the repo root)
    cargo uninstall zer                             # uninstall

### Developing

    cargo test --manifest-path zer/Cargo.toml   # run the suite

### Output shapes

Manifest: `{ version, base, head, files: [{ path, status, hunks: [{ start, count }] }], scopes: { react_rn, tests, motion, navigation, sonar, rn_security }, bundles: [{ key, files }] }`. Each scope is `{ in_scope, files }`. Verdicts: `ok`, `out-of-hunk`, `unknown-file`, `invalid-record`.

Known limits: findings must cite new-file lines inside changed hunks (pure-deletion hunks accept none); binary files carry zero hunks; scope matchers over-trigger by design (extra pass beats missed coverage); file paths containing spaces or quotes are matched verbatim and may miss git's quoted output.
