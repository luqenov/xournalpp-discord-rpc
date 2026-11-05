# Contributing

Thanks for taking the time to improve the project.

## Development

Requirements: Linux and Rust 1.71+.

```bash
git clone https://github.com/luqenov/xournalpp-discord-rpc.git
cd xournalpp-discord-rpc
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

## Branches and pull requests

- `main` is the stable branch.
- `dev` is the integration branch.
- Create feature branches from `dev` and open pull requests against `dev`.
- Keep changes focused and describe how you tested them.
- CI must pass before a pull request is merged.

## Commits

Use short, imperative messages, optionally with a scope:

```text
feat: add Hyprland support
fix: handle missing compositor socket
docs: clarify configuration precedence
```
