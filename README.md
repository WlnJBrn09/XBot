# XBot

XBot is the CruxOS system fixer. This repository starts with the X0 scaffold from the implementation plan. The daemon answers `Status` on the session D-Bus; detection, chat replies, privileged actions, and desktop control are not active yet.

`xbot-chat` is the XBot window. It follows the CruxOS design shared with the X apps (monochrome and light-first, one green for focus, toggles and selection, Phosphor icons) and embeds its UI, so the package installs a single binary. It shows the daemon's status and detected problems, has the chat composer, and edits `~/.config/xbot/config.toml`. Features the daemon has not reached yet are shown as not available rather than hidden.

Build and test with `cargo build --workspace` and `cargo test --workspace`. In a graphical session, run `cargo run -p xbotd`, then `cargo run -p crux -- xbot status --json` or `cargo run -p xbot-chat` in another terminal. Building `xbot-chat` needs `libgtk-3-dev` and `libwebkit2gtk-4.1-dev`.

The workspace is laid out as `crates/xbot/` so it can be moved into the CruxOS monorepo. The small `crux` CLI bridge here provides the X0 status command until the main CruxOS CLI is available. See [X0 status](docs/xbot/X0.md) and [decisions](docs/xbot/DECISIONS.md).
