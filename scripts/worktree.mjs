#!/usr/bin/env node
// Parallel sessions: a session that starts while the main folder is busy works in a git worktree
// next to it, in ../tadidy-worktrees/ (docs/guides/parallel-sessions.md).
// Usage: node scripts/worktree.mjs <status|add <branch>|review <N>|remove <name>|prune>

import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { basename, dirname, isAbsolute, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ENDED_PR_STATES = new Set(["MERGED", "CLOSED"]);

// Git hooks export the repository location (GIT_DIR is absolute inside a worktree): drop it so
// git always acts on `cwd`.
const REPOSITORY_VARS = new Set(["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE", "GIT_COMMON_DIR", "GIT_OBJECT_DIRECTORY", "GIT_PREFIX"]);

function gitEnv() {
  return Object.fromEntries(Object.entries(process.env).filter(([key]) => !REPOSITORY_VARS.has(key)));
}

function git(cwd, ...args) {
  return execFileSync("git", args, { cwd, env: gitEnv(), encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
}

function gitOrEmpty(cwd, ...args) {
  try {
    return git(cwd, ...args);
  } catch {
    return "";
  }
}

export function parseWorktrees(porcelain) {
  const worktrees = [];
  for (const block of porcelain.split("\n\n")) {
    const fields = Object.fromEntries(
      block
        .split("\n")
        .filter(Boolean)
        .map((line) => [line.split(" ", 1)[0], line.slice(line.indexOf(" ") + 1)]),
    );
    if (!fields.worktree) continue;
    worktrees.push({ path: fields.worktree, head: fields.HEAD, branch: fields.branch?.replace("refs/heads/", "") ?? null });
  }
  return worktrees;
}

export function isBusy({ branch, dirty }) {
  return branch !== "main" || dirty;
}

export function worktreeDir(mainRoot, name) {
  return join(dirname(mainRoot), `${basename(mainRoot)}-worktrees`, name);
}

export function hooksPathProblem(hooksPath) {
  if (!hooksPath || isAbsolute(hooksPath)) return null;
  return [
    `core.hooksPath is relative (${hooksPath}): inside a worktree it would point at nothing and every hook would be skipped.`,
    `Fix it once: git config --local core.hooksPath "$(git rev-parse --path-format=absolute --git-common-dir)/hooks"`,
  ].join("\n");
}

export function prunable(worktrees, mainRoot, prState) {
  const home = worktreeDir(mainRoot, "");
  return worktrees
    .filter((w) => w.path.startsWith(home))
    .filter((w) => basename(w.path).startsWith("review-pr-") || (w.branch && ENDED_PR_STATES.has(prState(w.branch))))
    .map((w) => w.path);
}

export function status(cwd) {
  const [main] = parseWorktrees(git(cwd, "worktree", "list", "--porcelain"));
  const branch = git(main.path, "branch", "--show-current") || null;
  const dirty = git(main.path, "status", "--porcelain") !== "";
  return { main: main.path, branch, dirty, busy: isBusy({ branch, dirty }) };
}

function checkHooksPath(main) {
  const problem = hooksPathProblem(gitOrEmpty(main, "config", "--get", "core.hooksPath"));
  if (problem) throw new Error(problem);
}

function install(dir) {
  if (existsSync(join(dir, "package.json"))) execFileSync("pnpm", ["install", "--frozen-lockfile"], { cwd: dir, stdio: "inherit" });
}

export function addWorktree(cwd, branch, { install: withDependencies = true } = {}) {
  const { main } = status(cwd);
  checkHooksPath(main);
  const dir = worktreeDir(main, branch);
  if (gitOrEmpty(main, "remote").split("\n").includes("origin")) git(main, "fetch", "-q", "origin", "main");
  const base = gitOrEmpty(main, "rev-parse", "--verify", "-q", "refs/remotes/origin/main") ? "origin/main" : "main";
  if (gitOrEmpty(main, "rev-parse", "--verify", "-q", `refs/heads/${branch}`)) git(main, "worktree", "add", dir, branch);
  else git(main, "worktree", "add", "--no-track", "-b", branch, dir, base);
  if (withDependencies) install(dir);
  return dir;
}

export function reviewWorktree(cwd, number, { install: withDependencies = true } = {}) {
  const { main } = status(cwd);
  checkHooksPath(main);
  const dir = worktreeDir(main, `review-pr-${number}`);
  git(main, "fetch", "-q", "origin", `pull/${number}/head`);
  const head = git(main, "rev-parse", "FETCH_HEAD");
  if (existsSync(dir)) git(dir, "checkout", "-q", "--detach", head);
  else git(main, "worktree", "add", "--detach", dir, head);
  if (withDependencies) install(dir);
  return dir;
}

function prState(main, branch) {
  try {
    return execFileSync("gh", ["pr", "view", branch, "--json", "state", "-q", ".state"], { cwd: main, encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] }).trim();
  } catch {
    return null;
  }
}

function prune(cwd) {
  const { main } = status(cwd);
  const worktrees = parseWorktrees(git(main, "worktree", "list", "--porcelain"));
  for (const path of prunable(worktrees, main, (branch) => prState(main, branch))) {
    try {
      git(main, "worktree", "remove", path);
      console.log(`removed ${path}`);
    } catch {
      console.log(`kept ${path}: it has uncommitted changes`);
    }
  }
  git(main, "worktree", "prune");
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [command, arg] = process.argv.slice(2);
  const cwd = process.cwd();
  try {
    if (command === "status") console.log(JSON.stringify(status(cwd), null, 2));
    else if (command === "add" && arg) console.log(addWorktree(cwd, arg));
    else if (command === "review" && /^\d+$/.test(arg ?? "")) console.log(reviewWorktree(cwd, arg));
    else if (command === "remove" && arg) git(status(cwd).main, "worktree", "remove", worktreeDir(status(cwd).main, arg));
    else if (command === "prune") prune(cwd);
    else {
      console.error("usage: node scripts/worktree.mjs <status|add <branch>|review <N>|remove <name>|prune>");
      process.exitCode = 2;
    }
  } catch (error) {
    console.error(`✖ ${error.stderr?.toString().trim() || error.message}`);
    process.exitCode = 1;
  }
}
