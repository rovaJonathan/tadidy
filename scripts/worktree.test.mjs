import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, realpathSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { addWorktree, hooksPathProblem, isBusy, parseWorktrees, prunable, status, worktreeDir } from "./worktree.mjs";

// The temporary repositories must not see the developer's global git config (hooks, signing).
process.env.GIT_CONFIG_GLOBAL = "/dev/null";

const git = (cwd, ...args) => execFileSync("git", args, { cwd, encoding: "utf8" }).trim();

function repo() {
  const base = realpathSync(mkdtempSync(join(tmpdir(), "worktree-")));
  const root = join(base, "tadidy");
  mkdirSync(root);
  git(root, "init", "-q", "-b", "main");
  git(root, "config", "user.email", "test@example.com");
  git(root, "config", "user.name", "Test");
  writeFileSync(join(root, "README.md"), "# test\n");
  git(root, "add", ".");
  git(root, "commit", "-q", "-m", "chore(TAD-000): init");
  return { base, root };
}

test("parses git worktree list --porcelain", () => {
  const porcelain = [
    "worktree /w/tadidy",
    "HEAD aaa",
    "branch refs/heads/tad-001-scaffold",
    "",
    "worktree /w/tadidy-worktrees/review-pr-3",
    "HEAD bbb",
    "detached",
    "",
  ].join("\n");
  assert.deepEqual(parseWorktrees(porcelain), [
    { path: "/w/tadidy", head: "aaa", branch: "tad-001-scaffold" },
    { path: "/w/tadidy-worktrees/review-pr-3", head: "bbb", branch: null },
  ]);
});

test("the main folder is busy when it is off main or has changes", () => {
  assert.equal(isBusy({ branch: "main", dirty: false }), false);
  assert.equal(isBusy({ branch: "tad-001-scaffold", dirty: false }), true);
  assert.equal(isBusy({ branch: "main", dirty: true }), true);
  assert.equal(isBusy({ branch: null, dirty: false }), true);
});

test("worktrees live next to the main folder", () => {
  assert.equal(worktreeDir("/w/tadidy", "tad-020-x"), "/w/tadidy-worktrees/tad-020-x");
});

test("a relative core.hooksPath is refused, an absolute or unset one is fine", () => {
  assert.match(hooksPathProblem(".git/hooks"), /core\.hooksPath.*git config --local core\.hooksPath/s);
  assert.equal(hooksPathProblem("/w/tadidy/.git/hooks"), null);
  assert.equal(hooksPathProblem(""), null);
});

test("prune selects merged or closed pull requests and review worktrees only", () => {
  const worktrees = [
    { path: "/w/tadidy", head: "a", branch: "main" },
    { path: "/w/tadidy-worktrees/tad-002-merged", head: "b", branch: "tad-002-merged" },
    { path: "/w/tadidy-worktrees/tad-003-closed", head: "c", branch: "tad-003-closed" },
    { path: "/w/tadidy-worktrees/tad-004-open", head: "d", branch: "tad-004-open" },
    { path: "/w/tadidy-worktrees/tad-005-no-pr", head: "e", branch: "tad-005-no-pr" },
    { path: "/w/tadidy-worktrees/review-pr-7", head: "f", branch: null },
    { path: "/elsewhere/tad-006-merged", head: "g", branch: "tad-006-merged" },
  ];
  const states = { "tad-002-merged": "MERGED", "tad-003-closed": "CLOSED", "tad-004-open": "OPEN", "tad-006-merged": "MERGED" };
  assert.deepEqual(prunable(worktrees, "/w/tadidy", (branch) => states[branch] ?? null), [
    "/w/tadidy-worktrees/tad-002-merged",
    "/w/tadidy-worktrees/tad-003-closed",
    "/w/tadidy-worktrees/review-pr-7",
  ]);
});

test("status reports the main folder and whether it is busy", () => {
  const { root } = repo();
  assert.deepEqual(status(root), { main: root, branch: "main", dirty: false, busy: false });
  writeFileSync(join(root, "new.txt"), "x\n");
  assert.equal(status(root).busy, true);
});

test("status ignores a GIT_DIR inherited from a git hook", () => {
  // Hooks run from a worktree export an absolute GIT_DIR: git would then act on that repository.
  const { root } = repo();
  const decoy = repo().root;
  process.env.GIT_DIR = join(decoy, ".git");
  try {
    assert.equal(status(root).main, root);
  } finally {
    delete process.env.GIT_DIR;
  }
});

test("add creates the branch in a worktree next to the main folder, from main", () => {
  const { base, root } = repo();
  const dir = addWorktree(root, "tad-999-something", { install: false });
  assert.equal(dir, join(base, "tadidy-worktrees", "tad-999-something"));
  assert.equal(git(dir, "branch", "--show-current"), "tad-999-something");
  assert.equal(git(dir, "rev-parse", "HEAD"), git(root, "rev-parse", "main"));
  assert.equal(status(dir).main, root, "status from a worktree still reports the main folder");
});

test("add refuses to run with a relative core.hooksPath", () => {
  const { base, root } = repo();
  git(root, "config", "core.hooksPath", ".git/hooks");
  assert.throws(() => addWorktree(root, "tad-999-something", { install: false }), /core\.hooksPath/);
  assert.equal(existsSync(join(base, "tadidy-worktrees", "tad-999-something")), false);
});
