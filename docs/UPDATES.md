# Update Availability

RenderNorth Industrial checks the official public GitHub Releases API:

`https://api.github.com/repos/Maxdelta/rendernorth-industrial/releases?per_page=20`

The check runs shortly after startup only when the configured interval has elapsed. It never blocks startup, requires no GitHub authentication, and sends only standard public HTTP headers plus `User-Agent: RenderNorth-Industrial/<installed version>`.

## User controls

- **Settings → Updates → Check Now** performs a manual check even when automatic checks are disabled.
- Automatic checks default to **Daily** and can be changed to **Weekly** or **Never**.
- Pre-release versions default to enabled for the Open Beta build.
- **Remind Me Later** suppresses the prominent notice for 24 hours.
- **Skip This Version** suppresses exactly that version. A newer semantic version still appears.
- **Restore Skipped Version Notifications** clears both skip and reminder state.

Update downloads always open the official GitHub installer or release page in the default browser. RenderNorth Industrial does not download, execute, install, restart, or replace application files.

## Failure and offline behavior

Offline, timeout, invalid-response, rate-limit, private-repository, and missing-asset failures are shown honestly. They are never rendered as **Up to Date**. The last successful release details remain cached locally and the rest of the application remains fully usable.

## Release requirements

Public releases use semantic tags such as `v0.1.1` or `v0.2.0-beta.1`. Draft releases are ignored. The release workflow publishes these deterministic asset names:

- `RenderNorth-Industrial-<version>-Windows-Installer.exe`
- `RenderNorth-Industrial-<version>-Windows-Portable.zip`

The installer is the primary Download Update target. If it is missing, the release page opens instead. The portable ZIP is offered separately when present.

## Mock and local testing

Runtime builds use the official endpoint above. For a dedicated test build, set the compile-time environment variable `RENDERNORTH_RELEASES_API_URL` to a local HTTP endpoint before running `cargo build` or `npm run tauri dev`. Serve a JSON array matching the GitHub Releases API response. This override is not exposed in the normal application UI.

Automated tests use deterministic mock JSON and status/transport classifications; they do not call live GitHub.
