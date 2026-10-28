Prefix: FZY
Next: 4

### FZY-3 · P4 · `make lint` rewrites the tree

- **Area:** Build
- **Where:** `Makefile:23`, `AGENTS.md:7`
- **Problem:** `make lint` runs `cargo clippy --fix --allow-dirty`. The check named in AGENTS.md is `cargo clippy --all-targets --all-features -- --deny warnings`, which does not edit files. `make lint` can change source as a side effect of a check.
- **Recommendation:** Make the `lint` target the AGENTS.md command, with no `--fix`.
- **Fix with:** none
- **Effort:** S
