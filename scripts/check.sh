#!/usr/bin/env bash
# Full local gate. There is no CI (ADR 0004): this script and the git hooks are the only gate.
# Run by the pre-push hook and before every pull request (skill verification-before-completion).
# Usage: scripts/check.sh [--cov]   --cov also enforces 80 % line coverage on changed files (TAD-002).
set -euo pipefail
cd "$(dirname "$0")/.."

COV=0
[ "${1:-}" = "--cov" ] && COV=1
step() { printf '\n\033[1m▶ %s\033[0m\n' "$*"; }

# A hook run from a worktree exports an absolute GIT_DIR: the tests' temporary repositories would
# then write to this repository. Git finds the repository from the current folder anyway.
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_COMMON_DIR GIT_OBJECT_DIRECTORY GIT_PREFIX

step "Repository scripts (node:test)"
node --test scripts/*.test.mjs

step "Tickets"
node scripts/tickets.mjs check

step "Repository consistency"
node scripts/repo-check.mjs

if [ -f src-tauri/Cargo.toml ]; then
  step "Rust format"
  cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
  step "Rust lint"
  cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
  if [ "$COV" = 1 ]; then
    step "Rust tests with coverage"
    mkdir -p coverage # cargo llvm-cov does not create the folder of --output-path
    cargo llvm-cov --manifest-path src-tauri/Cargo.toml --lcov --output-path coverage/rust.lcov
  else
    step "Rust tests"
    cargo test --manifest-path src-tauri/Cargo.toml
  fi
  if [ -d src/lib/bindings ]; then
    step "TypeScript bindings match the Rust types"
    if [ -n "$(git status --porcelain -- src/lib/bindings)" ]; then
      git status --short -- src/lib/bindings
      echo "✖ src/lib/bindings changed when the Rust tests exported them: commit the regenerated bindings." >&2
      exit 1
    fi
  fi
fi

if [ -f package.json ]; then
  step "Front lint"
  pnpm lint
  step "Front typecheck"
  pnpm typecheck
  if [ "$COV" = 1 ]; then
    step "Front tests with coverage"
    pnpm test:cov
  else
    step "Front tests"
    pnpm test
  fi
fi

if [ "$COV" = 1 ] && [ -f scripts/coverage-changed.mjs ]; then
  step "Coverage ≥ 80 % on changed files"
  node scripts/coverage-changed.mjs
fi

printf '\n\033[1;32m✓ check passed\033[0m\n'
