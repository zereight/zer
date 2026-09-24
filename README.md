# zer

`zer` reads a git diff and checks that finding line numbers sit inside that diff. It does not call a model.

## Layout

- `zer/` — Rust CLI source (`zer manifest`, `zer verify`)
- `schema/` — finding record shape enforced by `zer verify`
- `install.sh` — install a prebuilt binary
- `release.sh` — tag and push a release

## zer

`zer` builds review manifests from diffs and validates finding line refs. Install once per machine.

### Prerequisites

- git
- Rust 1.74+ with cargo. Check with `cargo --version`. If missing, install via rustup:

    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

  (accept the defaults when prompted), then restart your shell (or run `source "$HOME/.cargo/env"`) and re-run `cargo --version`.

### Install (prebuilt binary)

macOS arm64/x86_64 and Linux x86_64/arm64. The script downloads a release asset and checks `SHA256SUMS` before copy. No Rust toolchain required.

    curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/zereight/zer/main/install.sh | sh

Pin a version:

    curl --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/zereight/zer/main/install.sh | ZER_VERSION=v0.1.0 sh

Binary lands in `~/.local/bin`. If that directory is not on `PATH`, the script prints the `export` line.

### Install (from source)

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

### Release

From a clean `main` that matches `origin/main`:

    ./release.sh 0.2.0

That writes the version into `zer/Cargo.toml` and `zer/Cargo.lock`, commits, tags `v0.2.0`, and pushes the tag. The `release` GitHub Action builds the four binaries and publishes the GitHub Release. `main` commits alone do not publish.

### Developing

    cargo test --manifest-path zer/Cargo.toml   # run the suite

### Output shapes

Manifest: `{ version, base, head, files: [{ path, status, hunks: [{ start, count }] }], scopes: { react_rn, tests, motion, navigation, sonar, rn_security }, bundles: [{ key, files }] }`. Each scope is `{ in_scope, files }`. Verdicts: `ok`, `out-of-hunk`, `unknown-file`, `invalid-record`.

Known limits: findings must cite new-file lines inside changed hunks (pure-deletion hunks accept none); binary files carry zero hunks; scope matchers over-trigger by design (extra pass beats missed coverage); file paths containing spaces or quotes are matched verbatim and may miss git's quoted output.
