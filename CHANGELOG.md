# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project follows [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.2.0] - 2026-11-05

### Added

- CLI flags: `--help`, `--version`, `--verbose`, `--once`, `--dry-run`, `--no-document`, `--document`, `--client-id` and `--interval`
- Layered configuration through `client_id`, `config`, environment variables and CLI flags
- Support for niri, Hyprland, Sway/i3 and X11 window detection
- Focused-window PID preference and robust document title parsing
- Session timer, GitHub button and richer Discord presence
- Automatic reconnect with exponential backoff
- systemd user service, AUR `PKGBUILD` and Nix flake
- CI, release workflow, Dependabot and issue/PR templates

### Changed

- Release builds now use static musl binaries for x86_64 and aarch64
- Release archives include `README.md`, `LICENSE` and `SHA256SUMS.txt`
- Improved process start-time detection using `sysconf(_SC_CLK_TCK)`
- README rewritten; usage and troubleshooting moved to the Wiki
- Dependabot updates target `dev`

## [0.1.1] - 2026-10-17

### Changed

- Updated Discord presence handling

## [0.1.0] - 2026-10-17

### Added

- Initial release with niri window detection and Discord Rich Presence
