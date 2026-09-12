# Contributing to oterm

Thanks for your interest! oterm is a small Rust project, so the contribution flow is light.

## Development setup

1. Install Rust 1.75+ via [rustup](https://rustup.rs/).
2. Clone the repository and run:

   ```bash
   cargo build
   cargo run
   ```

3. The TOML config is read from `%APPDATA%\oterm\config.toml` (Windows) or `~/.config/oterm/config.toml` (Unix). When the file is missing, the bundled `src/config/default.toml` is used.

## Before opening a PR

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo build --release
```

CI runs the same checks.

## Code style

- Default to `cargo fmt`. The project keeps `rustfmt.toml` minimal.
- Prefer descriptive names over comments.
- Keep modules focused: `pty/` only knows about PTY I/O, `pane/` only manages the tree, `ui/` only renders.

## Reporting issues

Please include:
- OS and terminal you launched oterm from.
- Output of `oterm --version` (when available) or the commit hash.
- The contents of `%TEMP%\oterm.log` (Windows) / `/tmp/oterm.log` (Unix) if it has anything relevant.

## License

By contributing you agree that your contributions are licensed under the project's dual MIT / Apache-2.0 license.
