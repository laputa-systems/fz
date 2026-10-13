# fz

`fz` is a small Rust port of John Hawthorn's **fzy** fuzzy selector. It reads candidates from stdin, lets you filter and choose one in your terminal, and writes the selection to stdout.

## Install

Download the matching ZIP from [GitHub Releases](https://github.com/laputa-systems/fz/releases). The manual release workflow publishes prereleases for these targets:

| Platform | Archive |
| --- | --- |
| macOS arm64 | `fz-aarch64-apple-darwin.zip` |
| Linux x86_64, static | `fz-x86_64-unknown-linux-musl-static.zip` |
| Linux x86_64, dynamic | `fz-x86_64-unknown-linux-musl.zip` |
| Linux arm64, static | `fz-aarch64-unknown-linux-musl-static.zip` |
| Linux arm64, dynamic | `fz-aarch64-unknown-linux-musl.zip` |

Choose a static Linux archive unless your system provides the musl loader.

Extract the archive, then put its executable on your `PATH`. For macOS arm64:

```sh
mkdir -p ~/.local/bin
unzip fz-aarch64-apple-darwin.zip
install -m 755 fz-aarch64-apple-darwin ~/.local/bin/fz
```

Add `~/.local/bin` to `PATH` if it is not already there.

## Use

Pipe candidates into `fz` for interactive selection:

```sh
printf 'alpha\nbeta\n' | fz
```

By default, input records are separated by newlines and empty records are skipped; carriage returns are preserved. Enter writes the selected candidate followed by a newline. If nothing matches, Enter writes the query. Escape or Ctrl-C, Ctrl-D, or Ctrl-G cancels with status 1 and no stdout.

Use `--show-matches` to filter without opening a terminal:

```sh
fz --show-matches=mr < paths.txt
```

This prints matching candidates in score order and exits with status 0, including when there are no matches. Ties keep input order. `--read-null` reads NUL-separated records, which can contain newlines; selected output still ends with a newline.

## Options

| Option | Effect |
| --- | --- |
| `-l`, `--lines=LINES` | Number of result rows (default 10; minimum 3; or `max`) |
| `-p`, `--prompt=PROMPT` | Prompt text (default `> `) |
| `-q`, `--query=QUERY` | Start with a query |
| `-e`, `--show-matches=QUERY` | Print sorted matches without a terminal |
| `-t`, `--tty=TTY` | Terminal device (default `/dev/tty`) |
| `-s`, `--show-scores` | Show match scores |
| `-0`, `--read-null` | Read NUL-separated candidates |
| `-j`, `--workers=NUM` | Maximum workers (default up to four; small searches use one) |
| `-i`, `--show-info` | Show the match count |
| `-h`, `--help` | Show help |
| `-v`, `--version` | Show version |

Up/Down or Ctrl-P/N (also Ctrl-K/J) moves through results. Left/Right, Home/End, and Ctrl-A/E move the query cursor. Backspace deletes a character, Ctrl-W deletes the preceding word, Ctrl-U deletes before the cursor, Tab completes the selected candidate, and PageUp/PageDown moves by a page.

Matching follows fzy's byte-oriented behavior: lowercase ASCII query letters match either case, uppercase letters match uppercase only, and non-ASCII bytes match exactly. Query editing moves by UTF-8 code point; matching does not normalize or case-fold Unicode.

For help, use `fz --help` or visit the [issue tracker](https://github.com/laputa-systems/fz/issues). The project is MIT licensed; see [LICENSE](LICENSE).
