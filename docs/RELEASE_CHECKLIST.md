# Windows Release Checklist

This checklist is a release gate, not a promise that the current internal candidate is public-ready. Never remove, reset, or replace the application database to make a release test pass.

## Automated source and build gates

Run from a clean checkout on the intended Windows build host:

```powershell
npm ci
npm run licenses:verify
npm run check
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo check --manifest-path src-tauri/Cargo.toml --locked
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo clippy --manifest-path src-tauri/Cargo.toml --locked -- -D warnings
npm run release:build
```

`release:build` compiles with workspace and Cargo-home path remapping and then invokes `release:verify`. Verification must finish with `ARTIFACT_SCAN_CLEAN`.

For a public build, require a clean source tree and valid Windows signatures:

```powershell
npm run release:verify:public
```

Archive `src-tauri/target/release/release-manifest.json` with the exact executable and installer it describes. Record the manifest SHA-256 printed by the verifier in the release log.

## Dependency and legal gates

- Run `npm audit` and `cargo audit`; triage every advisory for the Windows target rather than ignoring aggregate output.
- Regenerate `THIRD_PARTY_NOTICES.txt` after every intentional dependency change, review the collected license text, and verify that its embedded lockfile hashes are current.
- Confirm that the installed `THIRD_PARTY_NOTICES.txt` exactly matches the reviewed source file.
- Open the Chinese About dialog and confirm the displayed package version, local-only data statement, and third-party notice filename match the reviewed build.
- Confirm the Human-selected Focus Garden license/EULA and publisher identity.
- Do not publish an artifact reported as `NotSigned`.

## Upgrade and data-safety gates

- Exercise supported upgrade paths in an isolated Windows environment, including the oldest supported installed version and the immediately preceding candidate.
- Before and after installation, record the application database hash and run read-only migration, integrity, foreign-key, session/plant relationship, and open-session checks.
- Launch, close, relaunch, and re-check the database. A release must not silently regenerate, delete, downgrade, or repair canonical data.
- Verify one uninstall entry, one install directory, and one Start Menu shortcut after an in-place upgrade.
- Test uninstall/reinstall retention only with an explicit disposable copy or Human-approved data-safety plan.

## Human Windows acceptance gates

- Verify the signed installer and installed application on supported Windows 10 and Windows 11 systems.
- Complete the Chinese start, pause, resume, abandon, recovery, completion, Garden, detail, close, and restart journeys.
- Verify keyboard-only operation, focus restoration, Narrator announcements, Windows High Contrast, reduced motion, and 200% display scaling.
- Review the title bar, installer language, startup-failure message, icon, typography, plant variation, and low-saturation visual system.
- Confirm final product wording, license/EULA presentation, publisher name, and release notes.

V1.0 may be marked complete only after every applicable automated, data-safety, signing, legal, platform, accessibility, and Human product gate above has evidence.
