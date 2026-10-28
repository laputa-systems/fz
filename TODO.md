Prefix: FZY
Next: 4

### FZY-2 · P3 · A failed termios restore is not retried

- **Area:** Operability
- **Where:** `src/terminal.rs:631`
- **Problem:** `Terminal::restore` sets `restore_needed` to false before `tcsetattr` returns. On failure, `Drop` calls `restore` again and skips the termios write, so the tty stays raw while the process exits. The Linux signal-mask path does retry: `ResizeEvents::restore` puts the saved mask back when `pthread_sigmask` fails (`src/terminal.rs:167`). This is the code path only. A live `tcsetattr` failure was not reproduced.
- **Recommendation:** Clear `restore_needed` only after `tcsetattr` succeeds, and retry `EINTR`. Leave the mask retry as it is.
- **Fix with:** none
- **Effort:** S

### FZY-3 · P4 · `make lint` rewrites the tree

- **Area:** Build
- **Where:** `Makefile:23`, `AGENTS.md:7`
- **Problem:** `make lint` runs `cargo clippy --fix --allow-dirty`. The check named in AGENTS.md is `cargo clippy --all-targets --all-features -- --deny warnings`, which does not edit files. `make lint` can change source as a side effect of a check.
- **Recommendation:** Make the `lint` target the AGENTS.md command, with no `--fix`.
- **Fix with:** none
- **Effort:** S
