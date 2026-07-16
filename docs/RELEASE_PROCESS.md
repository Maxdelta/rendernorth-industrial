# RenderNorth Industrial Release Process

For every release:

1. Verify implementation and live behavior.
2. Update the semantic version in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.
3. Update `CHANGELOG.md`.
4. Update `src/data/releases.json`, keeping releases newest first.
5. Confirm About, What's New, diagnostics, package metadata, and installer metadata show the same version.
6. Run `cargo build`, `cargo test`, `npm run build`, and `git diff --check`.
7. Commit and push only after approval.
8. Rebuild the Windows installer and portable ZIP from the final commit.
9. Generate SHA-256 hashes for both artifacts.
10. Publish the GitHub Release.
11. Update the RenderNorth website.
12. Post the Discord announcement.
13. Perform Memory Review only when durable product state changes.

## Release template

```markdown
## [X.Y.Z] — YYYY-MM-DD

### Release Name

One-paragraph user-facing summary.

### Added
- New user capability.

### Changed
- User-visible behavior change.

### Fixed
- Corrected defect.

### Known Issues
- Verified limitation or open defect.

### Security
- Optional security change.

### Deprecated
- Optional deprecated capability.

### Removed
- Optional removed capability.
```

The matching `src/data/releases.json` entry may include an `internalCheckpoint` for developer traceability. The application deliberately does not render that field.
