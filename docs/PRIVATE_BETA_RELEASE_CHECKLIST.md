# RenderNorth Industrial Private Beta Release Checklist

## Build provenance

- [ ] Product `main` is clean and synchronized with `origin/main`.
- [ ] Version matches in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.
- [ ] About displays the intended version, build timestamp, Git commit, SQLite version, migration version, and Rust version.
- [ ] `cargo build`, `cargo test`, `npm run build`, and `git diff --check` pass.

## Windows artifacts

- [ ] Run `npm run package:beta` on Windows.
- [ ] Confirm the NSIS installer exists under `src-tauri/target/release/bundle/nsis/`.
- [ ] Confirm the portable ZIP exists under `release-artifacts/`.
- [ ] Scan both artifacts with Windows Security.
- [ ] Record whether the build is code-signed. Unsigned private-beta builds must disclose the Windows SmartScreen warning.

## Fresh-machine acceptance

- [ ] Install on a Windows account that has never run RenderNorth Industrial.
- [ ] Confirm first launch opens the wizard instead of an empty dashboard.
- [ ] Save a CCP Client ID.
- [ ] Browse to and validate the official extracted CCP JSONL SDE.
- [ ] Import static data and confirm the reported build/version.
- [ ] Authorize a character and verify the exact requested read-only scopes.
- [ ] Refresh assets, locations, and blueprints.
- [ ] Refresh market data and confirm market, timestamp, age, and stale/current state.
- [ ] Finish setup, import an EFT doctrine, and analyze fleet readiness.
- [ ] Restart and confirm the wizard stays completed and all local state remains available.
- [ ] Export diagnostics and inspect the JSON for accidental tokens, Client IDs, or secrets.
- [ ] Repeat the core launch workflow with the portable ZIP.

## GitHub release

- [ ] Create a version tag only after live verification.
- [ ] Let `.github/workflows/private-beta-release.yml` create a draft release.
- [ ] Attach the NSIS installer and portable ZIP.
- [ ] Include SHA-256 hashes, known limitations, supported Windows versions, and unsigned-build disclosure.
- [ ] Link onboarding instructions and the issue tracker.
- [ ] Keep the release draft/private until the tester list and release notes are approved.

## Website release

- [ ] Publish the exact version and Git commit.
- [ ] Provide separate Installer and Portable ZIP download choices.
- [ ] Explain Windows SmartScreen/code-signing status before download.
- [ ] Publish privacy, local-storage, CCP read-only scope, and no-gameplay-automation statements.
- [ ] Link the onboarding guide, GitHub repository, issue form, and known limitations.
- [ ] Verify every download checksum against the GitHub release artifact.
