import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, realpathSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { evaluate, isGated, normalizePath, parseLcov } from "./coverage-changed.mjs";

// The temporary repositories must not see the developer's global git config (hooks, signing).
process.env.GIT_CONFIG_GLOBAL = "/dev/null";

const ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const SCRIPT = join(ROOT, "scripts", "coverage-changed.mjs");

const record = (path, found, hit) => `SF:${path}\nLF:${found}\nLH:${hit}\nend_of_record\n`;

test("parseLcov reads LF and LH of several records", () => {
  const lcov = "TN:\n" + record("src/lib/api.ts", 4, 3) + record("src-tauri/src/error.rs", 10, 10);
  const coverage = parseLcov(lcov, "/repo");
  assert.deepEqual(coverage.get("src/lib/api.ts"), { found: 4, hit: 3 });
  assert.deepEqual(coverage.get("src-tauri/src/error.rs"), { found: 10, hit: 10 });
});

test("parseLcov counts DA lines when a record has no LF/LH", () => {
  const lcov = "SF:src/lib/api.ts\nDA:1,1\nDA:2,0\nDA:3,5\nend_of_record\n";
  assert.deepEqual(parseLcov(lcov, "/repo").get("src/lib/api.ts"), { found: 3, hit: 2 });
});

test("parseLcov merges the lines of a file listed twice", () => {
  const lcov = "SF:a.rs\nDA:1,1\nDA:2,0\nend_of_record\nSF:a.rs\nDA:2,3\nDA:3,0\nend_of_record\n";
  assert.deepEqual(parseLcov(lcov, "/repo").get("a.rs"), { found: 3, hit: 2 });
});

test("an absolute SF path inside the repository matches the relative path from git diff", () => {
  assert.equal(normalizePath("/repo/src-tauri/src/watcher.rs", "/repo"), "src-tauri/src/watcher.rs");
  assert.equal(normalizePath("src/lib/api.ts", "/repo"), "src/lib/api.ts");
  assert.equal(normalizePath("./src/lib/api.ts", "/repo"), "src/lib/api.ts");
  const coverage = parseLcov(record("/repo/src-tauri/src/watcher.rs", 10, 9), "/repo");
  assert.deepEqual(coverage.get("src-tauri/src/watcher.rs"), { found: 10, hit: 9 });
});

test("isGated keeps Rust and front sources", () => {
  for (const path of ["src-tauri/src/watcher.rs", "src-tauri/src/storage/clips.rs", "src/lib/keyboard.ts", "src/lib/List.svelte"]) {
    assert.equal(isGated(path), true, path);
  }
});

test("isGated skips the macOS layer, generated, test and non-source files", () => {
  for (const path of [
    "src-tauri/src/macos/pasteboard.rs",
    "src-tauri/src/main.rs",
    "src-tauri/src/lib.rs",
    "src/lib/bindings/Settings.ts",
    "src/lib/api.test.ts",
    "src/test/setup.ts",
    "e2e/smoke.spec.ts",
    "src-tauri/tests/config.rs",
    "src/vite-env.d.ts",
    "src/main.ts",
    "src/App.svelte",
    "scripts/tickets.mjs",
    "README.md",
  ]) {
    assert.equal(isGated(path), false, path);
  }
});

test("a file at 80 % passes", () => {
  const coverage = new Map([["src-tauri/src/watcher.rs", { found: 10, hit: 8 }]]);
  const result = evaluate(["src-tauri/src/watcher.rs"], coverage, 80);
  assert.equal(result.ok, true);
  assert.deepEqual(result.files, [{ path: "src-tauri/src/watcher.rs", found: 10, hit: 8, pct: 80, ok: true }]);
});

test("a file at 70 % fails", () => {
  const coverage = new Map([["src-tauri/src/watcher.rs", { found: 10, hit: 7 }]]);
  const result = evaluate(["src-tauri/src/watcher.rs"], coverage, 80);
  assert.equal(result.ok, false);
  assert.equal(result.files[0].pct, 70);
  assert.equal(result.files[0].ok, false);
});

test("a changed source file missing from every report counts as 0 %", () => {
  const result = evaluate(["src/lib/keyboard.ts"], new Map(), 80);
  assert.equal(result.ok, false);
  assert.deepEqual(result.files, [{ path: "src/lib/keyboard.ts", found: 0, hit: 0, pct: 0, ok: false }]);
});

test("a reported file without executable lines passes", () => {
  const result = evaluate(["src/lib/types.ts"], new Map([["src/lib/types.ts", { found: 0, hit: 0 }]]), 80);
  assert.equal(result.ok, true);
});

test("excluded files are not checked", () => {
  const changed = ["src-tauri/src/macos/pasteboard.rs", "src/lib/bindings/Settings.ts", "src/lib/api.test.ts", "e2e/smoke.spec.ts"];
  assert.deepEqual(evaluate(changed, new Map(), 80), { ok: true, files: [] });
});

function repo() {
  const root = realpathSync(mkdtempSync(join(tmpdir(), "coverage-")));
  const git = (...args) => execFileSync("git", args, { cwd: root, encoding: "utf8" });
  git("init", "-q", "-b", "main");
  git("config", "user.email", "test@example.com");
  git("config", "user.name", "Test");
  mkdirSync(join(root, "src-tauri", "src"), { recursive: true });
  writeFileSync(join(root, "README.md"), "# test\n");
  git("add", ".");
  git("commit", "-q", "-m", "chore(TAD-000): init");
  git("checkout", "-q", "-b", "tad-999-something");
  return { root, git };
}

const run = (root, ...args) => spawnSync("node", [SCRIPT, ...args], { cwd: root, encoding: "utf8" });

test("the CLI prints a changed file under the minimum and exits 1", () => {
  const { root, git } = repo();
  writeFileSync(join(root, "src-tauri", "src", "watcher.rs"), "pub fn watch() {}\n");
  git("add", ".");
  git("commit", "-q", "-m", "feat(TAD-999): watch");
  writeFileSync(join(root, "rust.lcov"), record(join(root, "src-tauri", "src", "watcher.rs"), 10, 7));
  const result = run(root, "--lcov", "rust.lcov");
  assert.equal(result.status, 1, result.stderr);
  assert.match(result.stdout, /src-tauri\/src\/watcher\.rs\s+7\/10\s+70\.0 %/);
});

test("the CLI checks working-tree changes and passes at the minimum", () => {
  const { root } = repo();
  writeFileSync(join(root, "src-tauri", "src", "watcher.rs"), "pub fn watch() {}\n");
  execFileSync("git", ["add", "."], { cwd: root });
  writeFileSync(join(root, "rust.lcov"), record("src-tauri/src/watcher.rs", 10, 8));
  const result = run(root, "--lcov", "rust.lcov");
  assert.equal(result.status, 0, result.stdout + result.stderr);
  assert.match(result.stdout, /src-tauri\/src\/watcher\.rs\s+8\/10\s+80\.0 %/);
});

test("the CLI exits 0 when no source file changed", () => {
  const { root, git } = repo();
  writeFileSync(join(root, "README.md"), "# changed\n");
  git("commit", "-q", "-am", "docs(TAD-999): readme");
  const result = run(root, "--lcov", "missing.lcov");
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /no changed source file/);
});

test("the pre-push hook runs check.sh --cov, which runs the gate", () => {
  const lefthook = readFileSync(join(ROOT, "lefthook.yml"), "utf8");
  const prePush = lefthook.slice(lefthook.indexOf("pre-push:"));
  assert.match(prePush, /run: scripts\/check\.sh --cov/);
  const check = readFileSync(join(ROOT, "scripts", "check.sh"), "utf8");
  assert.match(check, /if \[ "\$COV" = 1 \] && \[ -f scripts\/coverage-changed\.mjs \]; then[^]*?node scripts\/coverage-changed\.mjs/);
});
