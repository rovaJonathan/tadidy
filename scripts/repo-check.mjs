#!/usr/bin/env node
// Repository consistency checks run by scripts/check.sh (there is no CI, ADR 0004).

import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const SKIP_DIRS = new Set([".git", "node_modules", "target", "dist", "coverage", ".claude/worktrees", "docs/archive"]);

function walk(root, dir = root, out = []) {
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    const rel = relative(root, path);
    if (SKIP_DIRS.has(name) || SKIP_DIRS.has(rel)) continue;
    if (statSync(path).isDirectory()) walk(root, path, out);
    else out.push(rel);
  }
  return out;
}

function checkClaudeFiles(root, files, errors) {
  for (const file of files.filter((f) => f === "CLAUDE.md" || f.endsWith("/CLAUDE.md"))) {
    const agents = join(dirname(file), "AGENTS.md");
    if (!existsSync(join(root, agents))) errors.push(`${file}: no ${agents} next to it`);
    if (readFileSync(join(root, file), "utf8").trim() !== "@AGENTS.md") errors.push(`${file}: must only contain @AGENTS.md`);
  }
}

function checkLatestStableRule(root, errors) {
  const agents = existsSync(join(root, "AGENTS.md")) ? readFileSync(join(root, "AGENTS.md"), "utf8") : "";
  for (const needle of ["pnpm view", "tag `latest`", "cargo add"]) {
    if (!agents.includes(needle)) errors.push(`AGENTS.md: must keep the latest-stable rule (mention ${needle})`);
  }
}

function checkSkills(root, errors) {
  const dir = join(root, ".claude/skills");
  if (!existsSync(dir)) return;
  const agents = existsSync(join(root, "AGENTS.md")) ? readFileSync(join(root, "AGENTS.md"), "utf8") : "";
  const skillsSection = /^## [^\n]*Skills[^\n]*\n([\s\S]*?)(?=^## |(?![\s\S]))/m.exec(agents)?.[1] ?? "";
  const listed = new Set([...skillsSection.matchAll(/^\| `([a-z0-9-]+)` \|/gm)].map((m) => m[1]));
  const present = new Set();
  for (const name of readdirSync(dir)) {
    const file = join(dir, name, "SKILL.md");
    if (!existsSync(file)) continue;
    present.add(name);
    if (!listed.has(name)) errors.push(`.claude/skills/${name}: not listed in the AGENTS.md skills table`);
    const header = /^---\n([\s\S]*?)\n---/.exec(readFileSync(file, "utf8"))?.[1];
    if (!header) {
      errors.push(`.claude/skills/${name}/SKILL.md: missing YAML header`);
      continue;
    }
    if (/^name:\s*(.+)$/m.exec(header)?.[1]?.trim() !== name) errors.push(`.claude/skills/${name}/SKILL.md: name must be ${name}`);
    if (!/^description:\s*\S/m.test(header)) errors.push(`.claude/skills/${name}/SKILL.md: missing description`);
  }
  for (const name of listed) if (!present.has(name)) errors.push(`AGENTS.md: skill \`${name}\` is listed but .claude/skills/${name}/SKILL.md does not exist`);
}

function checkAdrIndex(root, errors) {
  const dir = join(root, "docs/adr");
  if (!existsSync(dir)) return;
  const index = existsSync(join(dir, "README.md")) ? readFileSync(join(dir, "README.md"), "utf8") : "";
  for (const file of readdirSync(dir).filter((f) => /^\d{4}-.+\.md$/.test(f))) {
    if (!index.includes(`(${file})`)) errors.push(`docs/adr/${file}: not listed in docs/adr/README.md`);
  }
}

function checkLinks(root, files, errors) {
  for (const file of files.filter((f) => f.endsWith(".md") && !f.startsWith("tickets/_template"))) {
    const text = readFileSync(join(root, file), "utf8").replace(/```[\s\S]*?```/g, "").replace(/`[^`\n]*`/g, "");
    for (const [, target] of text.matchAll(/\]\(([^)\s]+)\)/g)) {
      if (/^(https?:|mailto:|#)/.test(target)) continue;
      const path = decodeURIComponent(target.split("#")[0]);
      if (!existsSync(join(root, dirname(file), path))) errors.push(`${file}: link to ${target} points to a missing file`);
    }
  }
}

export function checkRepo(root = REPO_ROOT) {
  const errors = [];
  const files = walk(root);
  checkClaudeFiles(root, files, errors);
  checkLatestStableRule(root, errors);
  checkSkills(root, errors);
  checkAdrIndex(root, errors);
  checkLinks(root, files, errors);
  return errors;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const errors = checkRepo();
  errors.forEach((e) => console.error(`✖ ${e}`));
  if (errors.length) process.exitCode = 1;
  else console.log("✓ repository consistent");
}
