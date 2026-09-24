# manifest + verify wiring

Deterministic helpers live in the `zer` CLI (installed via `cargo install --path zer --locked`). The coordinator uses them; ensemble passes do not call them directly.

## Coordinator flow

1. After the PR Identity Gate, build the manifest from the review worktree:
   `zer manifest --repo <worktree> --base refs/remotes/origin/<target> --head HEAD --out /tmp/review-manifest.json`
2. Copy `scopes` into `검증 결과` instead of hand-grepping (motion / test / navigation / react_rn / sonar / rn_security).
3. Before final synthesis, convert draft findings to `schema/finding.schema.json` records and run:
   `zer verify --manifest /tmp/review-manifest.json --findings /tmp/findings.json`
4. Drop `unknown-file` findings. Demote `out-of-hunk` to yellow or below unless the line is re-anchored inside a hunk. Fix `invalid-record` fields.
5. Record `manifest: ok (<n> files, <m> bundles)` and `verify: <ok>/<total>` in `검증 결과`.

## Rules

- The manifest never promotes a finding; it only scopes passes and kills hallucinated line refs.
- Scope over-trigger is expected. A spawned-but-empty pass is cheaper than a missed axis.
- Keep `schema/finding.schema.json` and the `zer verify` severity list in sync by hand.
