#!/usr/bin/env node
// commit-msg hook: Conventional Commits scoped to a ticket (AGENTS.md §15).
// Usage: node scripts/commit-msg.mjs <path to the commit message file>

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const TYPES = ["test", "feat", "fix", "refactor", "docs", "chore", "build", "perf", "wip"];
const GENERATED = /^(Merge |Revert "|fixup! |squash! |amend! )/;

export function checkMessage(message) {
  const lines = message.split("\n").filter((line) => !line.startsWith("#"));
  const subject = (lines[0] ?? "").trimEnd();
  if (subject.trim() === "") return "empty commit message";
  if (GENERATED.test(subject)) return null;

  const match = /^([a-z]+)\(([^)]*)\)(!)?: (.*)$/.exec(subject);
  if (!match) return `subject must look like "feat(TAD-042): imperative summary" (type(TAD-xxx): …)`;
  const [, type, scope, , summary] = match;
  if (!TYPES.includes(type)) return `unknown type "${type}": use one of ${TYPES.join(", ")}`;
  if (!/^TAD-\d{3}$/.test(scope)) return `scope must be the ticket id TAD-xxx (got "${scope}")`;
  if (summary.trim() === "") return "the subject after the colon is empty";
  if (subject.length > 72) return `subject is ${subject.length} characters long: keep it ≤ 72`;
  return null;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const error = checkMessage(readFileSync(process.argv[2], "utf8"));
  if (error) {
    console.error(`✖ commit message: ${error}`);
    process.exitCode = 1;
  }
}
