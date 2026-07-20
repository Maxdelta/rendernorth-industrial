# RenderNorth Industrial Open Beta 0.1.6 Release Checklist

- [ ] Fresh Windows install opens the first-run wizard.
- [ ] No Client ID entry is required in normal setup.
- [ ] Official CCP login and callback complete through the built-in public application.
- [ ] Advanced custom Client ID persists and Restore Official works; no Client Secret is requested.
- [ ] Official CCP static-data link opens and ZIP, wrong-folder, incomplete, unsupported, and inaccessible states are clear.
- [ ] All four required JSONL files report Found before import is enabled.
- [ ] Assets, locations, blueprints, and market refresh complete.
- [ ] A doctrine can be imported and analyzed.
- [ ] About shows Open Beta 0.1.6, build metadata, bug reporting, Discord, and optional support.
- [ ] Diagnostics contain no access token, refresh token, credential contents, Client Secret, or custom Client ID.
- [ ] The release tag is valid semantic version text (`vX.Y.Z` or a valid pre-release).
- [ ] Release assets use `RenderNorth-Industrial-<version>-Windows-Installer.exe` and `RenderNorth-Industrial-<version>-Windows-Portable.zip`.
- [ ] Settings → Updates checks the official release endpoint, prefers the installer, and offers the portable ZIP separately.
- [ ] Offline, rate-limited, skipped-version, remind-later, and manual-check states are live-verified.
- [ ] `cargo build`, `cargo test`, `npm run build`, and `git diff --check` pass.
- [ ] Installer and portable ZIP are rebuilt from the approved final commit and their SHA-256 hashes are published.
