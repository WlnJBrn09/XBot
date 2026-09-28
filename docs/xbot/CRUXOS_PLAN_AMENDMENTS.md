# Changes to copy into the main CruxOS plan

The main CruxOS plan is not in this repository. Apply these amendments when the monorepo is available.

1. Preinstalled agent CLIs are quiet until opened: no notifications, no autostart. Exception: XBot, the native system fixer, starts with the user session (`xbotd` systemd user service) and may show crash/error pop-ups, subject to its rate limits and the user's Do Not Disturb setting. XBot monitoring can be turned off in Settings > AI & Agents > XBot.
2. Brave is the only browser for the user. Exception: XBot uses a separate Chromium instance (Debian `chromium` package) with an isolated profile under `~/.local/share/xbot/chromium/`. It never opens as the user's browser, never becomes a default URL handler, and never reads Brave's profile.
3. XBot phases X0 through X7 span CruxOS phases 0 through 5 as mapped in the XBot implementation plan section 22.
