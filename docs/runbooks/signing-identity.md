# Signing identity (development)

## Scope and symptoms

macOS ties the **Accessibility** permission (auto-paste) and the **Keychain** item (database key) to the app's code signature. An ad-hoc signature changes at every build: after a rebuild, auto-paste silently stops (`copied_only`) and the database key prompt or `StorageError::KeyMissing` appears. A stable signing identity avoids both.

## Prerequisites

- Xcode command line tools (`xcode-select -p`).
- Either an Apple Developer account ("Developer ID Application" certificate, also needed to notarize) or nothing (self-signed certificate, personal use only).

## Steps — self-signed certificate (personal use)

Diagnosis (read-only): `security find-identity -v -p codesigning` lists the identities already usable.

1. Open **Keychain Access** and select the **login** keychain in the sidebar (not *System*). *Certificate Assistant* is in the **menu bar at the top of the screen**, not in the window: menu *Keychain Access* → *Certificate Assistant* → *Create a Certificate…* (or `⌘⌥⌃C`).
2. Name `Tadidy Dev`, Identity Type *Self-Signed Root*, Certificate Type **Code Signing**, keep *Let me override defaults* unchecked → *Create*. It lands in the **login** keychain.
3. **Trust it for code signing.** macOS does not trust a self-signed certificate by default, and `codesign` ignores an untrusted identity. In *My Certificates*, double-click `Tadidy Dev`, expand *Trust*, set *Code Signing* to **Always Trust**, close the window and enter your password. Or from a terminal (asks for your password):
   ```
   security find-certificate -c "Tadidy Dev" -p ~/Library/Keychains/login.keychain-db > /tmp/tadidy-dev.pem
   security add-trusted-cert -p codeSign -k ~/Library/Keychains/login.keychain-db /tmp/tadidy-dev.pem
   rm /tmp/tadidy-dev.pem
   ```
4. Check: `security find-identity -v -p codesigning` lists `"Tadidy Dev"` under *valid identities*. Without `-v` the list also shows expired or revoked certificates and an untrusted `Tadidy Dev` (`CSSMERR_TP_NOT_TRUSTED`): that means step 3 is missing.
5. In your shell profile (never in the repository): `export APPLE_SIGNING_IDENTITY="Tadidy Dev"`, then open a new terminal.
6. Build: `pnpm build:universal` (or `pnpm tauri build`); `signingIdentity` is `null` in `tauri.conf.json`, so Tauri reads the variable. Without it the build is signed ad-hoc.

## Steps — Developer ID (distribution)

1. In Xcode → *Settings* → *Accounts* → *Manage Certificates* → **+** → *Developer ID Application*.
2. `export APPLE_SIGNING_IDENTITY="Developer ID Application: <Name> (<TEAMID>)"`.
3. Notarization variables (`APPLE_ID`, `APPLE_PASSWORD` app-specific password, `APPLE_TEAM_ID`) go in the shell or a password manager, never in the repository; see skill `release-dmg`.

## Verification

- `codesign -dv --verbose=2 src-tauri/target/universal-apple-darwin/release/bundle/macos/Tadidy.app` shows `Authority=Tadidy Dev` (or your Developer ID), not `adhoc`; `codesign --verify --deep --strict` on it prints nothing.
- `codesign -d -r- <same path>` prints `designated => identifier "app.tadidy" and certificate leaf = H"…"`: the same line after every rebuild, because it names the certificate, not the build.
- `lipo -archs <same path>/Contents/MacOS/*` prints `x86_64 arm64`.
- A build without notarization variables prints `skipping app notarization`: expected for a development build.
- Grant Accessibility once, rebuild, relaunch: *System Settings → Privacy & Security → Accessibility* still lists Tadidy enabled, and auto-paste works.

## Rollback

- Delete the certificate in Keychain Access, unset `APPLE_SIGNING_IDENTITY`; builds go back to ad-hoc signing.
- If permissions get confused: remove Tadidy from the Accessibility list (`−`), or `tccutil reset Accessibility app.tadidy`, then grant again.
