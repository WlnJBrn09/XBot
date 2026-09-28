# XBot

XBot is the CruxOS system fixer. This repository starts with the X0 scaffold from the implementation plan. The daemon answers `Status` on the session D-Bus; detection, chat, privileged actions, and desktop control are not active yet.

Build and test with `cargo build --workspace` and `cargo test --workspace`. In a graphical session, run `cargo run -p xbotd`, then `cargo run -p crux -- xbot status --json` in another terminal.

The workspace is laid out as `crates/xbot/` so it can be moved into the CruxOS monorepo. The small `crux` CLI bridge here provides the X0 status command until the main CruxOS CLI is available. See [X0 status](docs/xbot/X0.md) and [decisions](docs/xbot/DECISIONS.md).
