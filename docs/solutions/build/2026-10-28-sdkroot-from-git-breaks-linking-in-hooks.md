# Linking fails only in git hooks: `tapi error: malformed file … unknown architecture`

- **Date**: 2026-10-28 · **Ticket**: TAD-001

## Symptom

`scripts/check.sh --cov` passes in a terminal but fails in the `pre-push` hook while linking a build script:

```
ld: multiple errors: tapi error: malformed file
/Library/Developer/CommandLineTools/SDKs/MacOSX27.0.sdk/usr/lib/libSystem.B.tbd:4:20: error: unknown architecture
                     arm64e.x1-macos, arm64e.x1-maccatalyst ]
```

## Cause

`/usr/bin/git` is an `xcrun` shim: it exports `SDKROOT` (the Command Line Tools SDK, here macOS 27) to every command it runs, hooks included. `xcode-select` points to an older Xcode whose `ld` (`ld-1221`) does not know the architectures listed in the newer SDK's `.tbd` files. In a terminal `SDKROOT` is unset, so the toolchain uses its own matching SDK and the link succeeds.

Reproduce: `touch src-tauri/build.rs && SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk cargo build --manifest-path src-tauri/Cargo.toml`.

## Fix

`scripts/check.sh` runs `unset SDKROOT` at the start, so the gate behaves the same from a hook and from a terminal. Aligning Xcode and the Command Line Tools on the same major version also removes the mismatch.
