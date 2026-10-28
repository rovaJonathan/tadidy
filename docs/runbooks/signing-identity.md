# Signing identity (development)

## Scope and symptoms

macOS ties the **Accessibility** permission (auto-paste) and the **Keychain** item (database key) to the app's code signature. An ad-hoc signature changes at every build: after a rebuild, auto-paste silently stops (`copied_only`) and the database key prompt or `StorageError::KeyMissing` appears. A stable signing identity avoids both.

## Prerequisites

- Xcode command line tools (`xcode-select -p`).
- Either an Apple Developer account ("Developer ID Application" certificate, also needed to notarize) or nothing (self-signed certificate, personal use only).

## Steps — self-signed certificate (personal use)

Diagnosis (read-only): `security find-identity -v -p codesigning` lists the identities already usable.

1. Open **Keychain Access** → menu *Keychain Access* → *Certificate Assistant* → *Create a Certificate…*
2. Name `Tadidy Dev`, Identity Type *Self-Signed Root*, Certificate Type **Code Signing**, keep *Let me override defaults* unchecked → *Create*. It lands in the **login** keychain.
3. Check: `security find-identity -v -p codesigning` shows `"Tadidy Dev"`.
4. In your shell profile (never in the repository): `export APPLE_SIGNING_IDENTITY="Tadidy Dev"`.
5. Build: `pnpm tauri build` (TAD-008 wires `signingIdentity` to this variable).

## Steps — Developer ID (distribution)

1. In Xcode → *Settings* → *Accounts* → *Manage Certificates* → **+** → *Developer ID Application*.
2. `export APPLE_SIGNING_IDENTITY="Developer ID Application: <Name> (<TEAMID>)"`.
3. Notarization variables (`APPLE_ID`, `APPLE_PASSWORD` app-specific password, `APPLE_TEAM_ID`) go in the shell or a password manager, never in the repository; see skill `release-dmg`.

## Verification

- `codesign -dv --verbose=2 src-tauri/target/universal-apple-darwin/release/bundle/macos/Tadidy.app` shows `Authority=Tadidy Dev` (or your Developer ID), not `adhoc`.
- Grant Accessibility once, rebuild, relaunch: *System Settings → Privacy & Security → Accessibility* still lists Tadidy enabled, and auto-paste works.

## Rollback

- Delete the certificate in Keychain Access, unset `APPLE_SIGNING_IDENTITY`; builds go back to ad-hoc signing.
- If permissions get confused: remove Tadidy from the Accessibility list (`−`), or `tccutil reset Accessibility app.tadidy`, then grant again.
