# XBot implementation decisions

- This repository is empty rather than the CruxOS monorepo. The workspace follows the planned `crates/xbot/` layout, with a temporary `crates/crux-cli` bridge for `crux xbot status`. Integrate that subcommand into the main CruxOS CLI when its source is available.
- The X0 binaries for later phases are explicit inactive stubs. Their D-Bus and system activation files are not installed, so no privileged or unfinished action can run.
- No suggested crates.io dependency has been replaced. X0 uses `zbus` for D-Bus.
- Debian metadata uses `xbot@invalid.example` as an explicit placeholder because no maintainer email was supplied. Replace it before publishing packages.
- The project license and copyright holder were not supplied. The package metadata marks both as pending; no license grant is asserted by this scaffold.
- The CruxOS-wide rule changes are recorded in `CRUXOS_PLAN_AMENDMENTS.md`; the main plan is not in this checkout.
