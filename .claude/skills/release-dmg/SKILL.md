---
name: release-dmg
description: Produces a Tadidy release — version bump in the three manifests, signed universal build (Apple Silicon + Intel), DMG, optional notarization, smoke test of the bundle, tag vX.Y.Z on main and GitHub release notes. Use when the human says "release", "make a DMG", "ship 0.2.0", or for a ticket about packaging or updates.
---

# release-dmg — ship a build without surprises

Signing: [docs/runbooks/signing-identity.md](../../../docs/runbooks/signing-identity.md). Delivery rules: [docs/spec/06-delivery.md#permissions-signing](../../../docs/spec/06-delivery.md#permissions-signing). The agent prepares everything; **the human pushes the tag and publishes**.

## 1. Preconditions

- On `main`, up to date, clean (`git status --short` empty), every ticket of the release `done` and merged.
- `scripts/check.sh --cov` green on `main`.
- `APPLE_SIGNING_IDENTITY` set (`security find-identity -v -p codesigning` lists it). For notarization: `APPLE_ID`, `APPLE_PASSWORD` (app-specific), `APPLE_TEAM_ID` in the environment — never in the repository or the conversation.

## 2. Version (a ticket branch, like any change)

- Semantic version: `0.y.z` until v1.0 (phase 2 done).
- Same version in `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json` (+ `Cargo.lock` updated by `cargo check`).
- `CHANGELOG.md`: a section `## X.Y.Z — YYYY-MM-DD` listing the tickets (`TAD-xxx — title`) merged since the previous tag (`git log --oneline vPREV..HEAD | grep -oE 'TAD-[0-9]{3}' | sort -u`).
- Pull request `TAD-xxx — Release X.Y.Z`, reviewed and merged by the human.

## 3. Build

```bash
git checkout main && git pull --rebase
pnpm install --frozen-lockfile
pnpm tauri build --target universal-apple-darwin
```
Outputs in `src-tauri/target/universal-apple-darwin/release/bundle/` (`macos/Tadidy.app`, `dmg/Tadidy_X.Y.Z_universal.dmg`).

## 4. Verify the bundle

```bash
APP=src-tauri/target/universal-apple-darwin/release/bundle/macos/Tadidy.app
lipo -archs "$APP/Contents/MacOS/tadidy"                 # arm64 x86_64
codesign --verify --deep --strict --verbose=2 "$APP"     # valid, not adhoc
codesign -dv "$APP" 2>&1 | grep Authority
plutil -extract LSUIElement raw "$APP/Contents/Info.plist"   # true
spctl --assess --type execute -vv "$APP"                 # accepted only once notarized
```
Smoke test (manual, from the DMG on a clean user account if possible): the tray icon appears, no Dock icon; copy a text → it is in the popover; ⌘⇧V → Quick Panel; Enter pastes into TextEdit; quit and relaunch → history intact. Note the results.

## 5. Notarize (only if the build is shared)

```bash
xcrun notarytool submit <dmg> --apple-id "$APPLE_ID" --password "$APPLE_PASSWORD" --team-id "$APPLE_TEAM_ID" --wait
xcrun stapler staple <dmg>
spctl --assess --type open --context context:primary-signature -vv <dmg>
```

## 6. Tag and publish (the human)

Give the human the exact commands and stop:
```bash
git tag -a vX.Y.Z -m "Tadidy X.Y.Z" && git push origin vX.Y.Z
gh release create vX.Y.Z <dmg> --title "Tadidy X.Y.Z" --notes-file <notes extracted from CHANGELOG.md>
```

## Rollback

A bad release: `gh release delete vX.Y.Z` (keep the tag for history, or delete it only if never downloaded), fix in a ticket, release `X.Y.(Z+1)`. Never move a published tag.
