# Architecture

`src/main.rs` parses options, reads candidate bytes, and chooses interactive or filter mode. `src/lib.rs` stores candidates and ranks matches. `src/score.rs` implements fzy matching, scoring, and highlight positions. `src/terminal.rs` owns the raw terminal session and rendering.

## Matching and ranking

Eligibility is a byte-wise subsequence search. Lowercase ASCII query bytes accept either case; uppercase query bytes accept only uppercase. Other bytes match exactly. Scoring folds ASCII case after eligibility, as fzy does. Matching does not normalize or case-fold Unicode.

Exact matches score positive infinity. Empty queries, nonmatches, and candidates longer than 1024 bytes score negative infinity. Candidates over 1024 bytes remain eligible, but they have no highlighted positions. Equal scores retain input order.

The upstream C build can round the initial gap score differently by platform: the macOS Clang build contracts the expression when FMA is available, while upstream's Linux GCC build rounds it twice. Preserve those target-specific defaults because a one-ULP difference can change ranking.

`Choices` ranks only the result prefix needed by the screen and expands that prefix as rows are read. The library type defaults to one worker; the CLI requests automatic sizing, capped at four, and small searches remain serial. The workers share candidate storage and combine disjoint result ranges before ranking.

## Terminal and process behavior

Interactive mode reads candidates from stdin and opens the configured terminal device, `/dev/tty` by default. The terminal saves its original termios state, enters raw mode, and restores the saved state on completion, cancellation, or drop. Linux resize delivery uses a blocked `SIGWINCH` mask and `signalfd`; it does not install a signal handler. macOS waits with `kevent` on a real terminal device. `poll` reports `POLLNVAL` for `/dev/tty` and `kevent` rejects that device with `EINVAL`, so that path stays on `select`. A positive `select` timeout does not return on the Darwin 25 GitHub runner, so that deadline writes the wake pipe instead of passing a timeval. `SIGWINCH` stays blocked, and a thread blocked in `sigwait` writes one byte to a pipe; a handler does not run, because handling `SIGWINCH` during a tty operation can wedge the process. A bare Escape cancels 25ms of wall time after the byte, so a resize wake cannot postpone that deadline.

Result rows wrap at the terminal width instead of truncating, so a long candidate occupies as many physical rows as it needs within the `--lines` budget; later candidates are dropped from view when the budget is full. Unlike fzy, which clips a row at the right edge, wrapping keeps the tail of entries such as long process argument lists readable on narrow terminals. Wide and combining characters count by display width when wrapping.

The binary uses a custom entry point to skip Rust runtime startup. It keeps the default `SIGPIPE` disposition and flushes its own stdout before returning.
