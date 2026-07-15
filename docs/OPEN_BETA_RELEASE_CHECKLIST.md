# RenderNorth Industrial Open Beta 0.1 Release Checklist

- [ ] Fresh Windows install opens the first-run wizard.
- [ ] No Client ID entry is required in normal setup.
- [ ] Official CCP login and callback complete through the built-in public application.
- [ ] Advanced custom Client ID persists and Restore Official works; no Client Secret is requested.
- [ ] Official CCP static-data link opens and ZIP, wrong-folder, incomplete, unsupported, and inaccessible states are clear.
- [ ] All four required JSONL files report Found before import is enabled.
- [ ] Assets, locations, blueprints, and market refresh complete.
- [ ] A doctrine can be imported and analyzed.
- [ ] About shows Open Beta 0.1, build metadata, bug reporting, Discord, and optional support.
- [ ] Diagnostics contain no access token, refresh token, credential contents, Client Secret, or custom Client ID.
- [ ] `cargo build`, `cargo test`, `npm run build`, and `git diff --check` pass.
- [ ] Installer and portable ZIP are rebuilt from the approved final commit and their SHA-256 hashes are published.
