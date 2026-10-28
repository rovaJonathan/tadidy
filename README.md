# Tadidy

A macOS clipboard-history manager that lives in the menu bar. It records everything you copy (text, rich text, images, files, links) in an encrypted local database, finds it again in a few keystrokes and pastes it back where you were. Rust + Tauri 2 + Svelte 5. Local only: nothing leaves your Mac.

- **Specification**: [docs/spec/](docs/spec/README.md) ([one-page HTML](docs/spec/spec.html))
- **Prototype**: [docs/prototype/prototype.html](docs/prototype/prototype.html), the reference for every screen
- **How we work** (agents and humans): [AGENTS.md](AGENTS.md)
- **Tickets**: [tickets/](tickets/README.md) · decisions: [PROGRESS.md](PROGRESS.md), [docs/adr/](docs/adr/README.md)

## Set up a clone

Prerequisites: macOS 13+, Xcode command line tools, Homebrew, [rustup](https://rustup.rs), Node 24 (`.nvmrc`) and pnpm 10.

```bash
brew install lefthook gitleaks cargo-llvm-cov
rustup show                                 # installs the toolchain, components and targets of rust-toolchain.toml
git config --local core.hooksPath "$(git rev-parse --path-format=absolute --git-common-dir)/hooks"   # absolute: hooks must also run in worktrees
lefthook install --force                    # the hooks are the only quality gate (no CI)
scripts/check.sh                            # everything green before you start
```

For auto-paste to keep working across rebuilds, sign with a stable identity: [docs/runbooks/signing-identity.md](docs/runbooks/signing-identity.md).

## Everyday commands

| What | Command |
|---|---|
| What to work on | ask the agent "What should I do today?" — or `node scripts/tickets.mjs next` |
| Run the app | `pnpm tauri dev` (after TAD-001) |
| Full local gate | `scripts/check.sh` (`--cov` adds the coverage gate; run by `pre-push`) |
| Tickets | `node scripts/tickets.mjs list` · `ready` · `check` |
| Rebuild the spec HTML | `mise exec pandoc@3.1.11.1 -- scripts/spec/build.sh` |
