#!/usr/bin/env node
// Ticket tracking for Tadidy: tickets/TAD-xxx.md files with a YAML header (ADR 0003).
// Usage: node scripts/tickets.mjs <list|next|ready|show|set|check> [...]
// Rules: docs/guides/tickets.md. No dependencies: the header is a small YAML subset
// (key: value, quoted strings, integers, inline arrays).

import { execFileSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { parseWorktrees } from "./worktree.mjs";

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const TICKET_FILE = /^TAD-\d{3}\.md$/;
const PROTOTYPE = "docs/prototype/prototype.html";

export const STATUSES = ["todo", "in-progress", "review", "done", "blocked", "canceled"];
const ACTIVE = ["in-progress", "review"];
const CLOSED = ["done", "canceled"];
const DATE = /^\d{4}-\d{2}-\d{2}$/;

const FIELDS = {
  id: (v) => /^TAD-\d{3}$/.test(v) || "must look like TAD-042",
  title: (v) => (typeof v === "string" && v.trim() !== "") || "must not be empty",
  status: (v) => STATUSES.includes(v) || `must be one of ${STATUSES.join(", ")}`,
  phase: (v) => [0, 1, 2, 3].includes(v) || "must be 0, 1, 2 or 3",
  priority: (v) => ["P0", "P1", "P2"].includes(v) || "must be P0, P1 or P2",
  estimate: (v) => ["S", "M", "L", "XL"].includes(v) || "must be S, M, L or XL",
  area: (v) => ["core", "ui", "full", "repo"].includes(v) || "must be core, ui, full or repo",
  blockedBy: (v) => (Array.isArray(v) && v.every((x) => /^TAD-\d{3}$/.test(x))) || "must be a list of TAD-xxx",
  spec: (v) => Array.isArray(v) || "must be a list",
  prototype: (v) => Array.isArray(v) || "must be a list",
  branch: (v) => v === "" || /^tad-\d{3}-[a-z0-9-]+$/.test(v) || "must look like tad-042-short-slug",
  pr: (v) => v === "" || v === "none" || /^https?:\/\//.test(v) || "must be a URL or none",
  created: (v) => DATE.test(v) || "must be YYYY-MM-DD",
  started: (v) => v === "" || DATE.test(v) || "must be YYYY-MM-DD or empty",
  completed: (v) => v === "" || DATE.test(v) || "must be YYYY-MM-DD or empty",
};
const LIST_FIELDS = ["blockedBy", "spec", "prototype"];

export const REQUIRED_SECTIONS = [
  "Context",
  "Scope",
  "Out of scope",
  "Steps",
  "Files",
  "Acceptance criteria",
  "Tests",
  "Verification",
  "Definition of done",
  "References",
  "Notes",
];

// ---------- parsing ----------

function stripComment(raw) {
  let quoted = false;
  for (let i = 0; i < raw.length; i++) {
    if (raw[i] === '"') quoted = !quoted;
    if (raw[i] === "#" && !quoted && (i === 0 || /\s/.test(raw[i - 1]))) return raw.slice(0, i).trimEnd();
  }
  return raw.trimEnd();
}

function parseScalar(raw) {
  const s = raw.trim();
  if (s.startsWith('"') && s.endsWith('"') && s.length >= 2) return JSON.parse(s);
  if (/^-?\d+$/.test(s)) return Number(s);
  return s;
}

function parseList(raw) {
  const inner = raw.trim().slice(1, -1).trim();
  if (inner === "") return [];
  const items = [];
  let current = "";
  let quoted = false;
  for (const ch of inner) {
    if (ch === '"') quoted = !quoted;
    if (ch === "," && !quoted) {
      items.push(parseScalar(current));
      current = "";
    } else current += ch;
  }
  items.push(parseScalar(current));
  return items;
}

function parseValue(raw) {
  const s = stripComment(raw).trim();
  return s.startsWith("[") && s.endsWith("]") ? parseList(s) : parseScalar(s);
}

export function parseTicket(text) {
  const match = /^---\n([\s\S]*?)\n---\n?([\s\S]*)$/.exec(text);
  if (!match) throw new Error("missing YAML header between --- lines");
  const header = {};
  for (const line of match[1].split("\n")) {
    if (line.trim() === "" || line.trim().startsWith("#")) continue;
    const kv = /^([A-Za-z]+):\s?(.*)$/.exec(line);
    if (!kv) throw new Error(`unreadable header line: ${line}`);
    header[kv[1]] = parseValue(kv[2]);
  }
  const body = match[2];
  const h1 = /^# (.+)$/m.exec(body)?.[1]?.trim() ?? "";
  const sections = [...body.matchAll(/^## (.+)$/gm)].map((m) => m[1].trim());
  return { header, body, h1, sections };
}

export function serializeValue(value) {
  if (Array.isArray(value)) return `[${value.map(serializeValue).join(", ")}]`;
  if (typeof value === "number") return String(value);
  if (value === "" || /[:#,"\[\]{}]|^\s|\s$|^[-?!&*|>'%@`]/.test(value) || /^-?\d+$/.test(value)) {
    return JSON.stringify(value);
  }
  return value;
}

function coerce(key, raw) {
  if (typeof raw !== "string") return raw;
  if (LIST_FIELDS.includes(key)) {
    if (raw.trim().startsWith("[")) return parseList(raw);
    return raw.trim() === "" ? [] : raw.split(",").map((s) => s.trim());
  }
  if (key === "phase" && /^\d+$/.test(raw)) return Number(raw);
  return raw;
}

export function setFields(text, fields) {
  const match = /^---\n([\s\S]*?)\n---\n/.exec(text);
  if (!match) throw new Error("missing YAML header between --- lines");
  let headerText = match[1];
  for (const [key, raw] of Object.entries(fields)) {
    if (!(key in FIELDS)) throw new Error(`unknown field "${key}"`);
    const value = coerce(key, raw);
    const verdict = FIELDS[key](value);
    if (verdict !== true) throw new Error(`${key} ${verdict}`);
    const line = new RegExp(`^${key}:.*$`, "m");
    if (!line.test(headerText)) throw new Error(`field "${key}" missing from the header`);
    headerText = headerText.replace(line, () => `${key}: ${serializeValue(value)}`);
  }
  return `---\n${headerText}\n---\n${text.slice(match[0].length)}`;
}

// ---------- loading ----------

function git(root, args) {
  try {
    return execFileSync("git", ["-C", root, ...args], { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] });
  } catch {
    return null;
  }
}

function statusOf(text) {
  try {
    return parseTicket(text).header.status;
  } catch {
    return null;
  }
}

// A ticket closed on main stays closed: an old local branch may still say `review`.
function branchVersions(root) {
  const current = git(root, ["rev-parse", "--abbrev-ref", "HEAD"])?.trim();
  const refs = git(root, ["for-each-ref", "--format=%(refname:short)", "refs/heads/"]);
  const main = git(root, ["rev-parse", "--verify", "--quiet", "origin/main"]) ? "origin/main" : "main";
  const versions = new Map();
  for (const branch of (refs ?? "").split("\n").filter(Boolean)) {
    const num = /^tad-(\d{3})-/.exec(branch)?.[1];
    if (!num || branch === current) continue;
    const onMain = git(root, ["show", `${main}:tickets/TAD-${num}.md`]);
    if (onMain !== null && CLOSED.includes(statusOf(onMain))) {
      versions.set(`TAD-${num}`, { branch: "main", text: onMain });
      continue;
    }
    const text = git(root, ["show", `${branch}:tickets/TAD-${num}.md`]);
    if (text !== null) versions.set(`TAD-${num}`, { branch, text });
  }
  return versions;
}

// Branch → folder where it is checked out (the main folder or a worktree).
function checkedOutBranches(root) {
  const porcelain = git(root, ["worktree", "list", "--porcelain"]) ?? "";
  return new Map(parseWorktrees(porcelain).filter((w) => w.branch).map((w) => [w.branch, w.path]));
}

export function loadTickets(root = REPO_ROOT, { branches = false } = {}) {
  const dir = join(root, "tickets");
  const overrides = branches ? branchVersions(root) : new Map();
  const checkedOut = branches ? checkedOutBranches(root) : new Map();
  return readdirSync(dir)
    .filter((f) => TICKET_FILE.test(f))
    .sort()
    .map((file) => {
      const path = join(dir, file);
      const id = file.slice(0, -3);
      const override = overrides.get(id);
      try {
        const ticket = parseTicket(override?.text ?? readFileSync(path, "utf8"));
        const where = (ticket.header.branch && checkedOut.get(ticket.header.branch)) || null;
        return { ...ticket, file, path, source: override?.branch ?? "working tree", checkedOut: where };
      } catch (error) {
        return { header: {}, body: "", h1: "", sections: [], file, path, source: "working tree", error: error.message };
      }
    });
}

// ---------- validation ----------

function anchorExists(root, ref) {
  const [path, anchor] = ref.split("#");
  const file = join(root, path);
  if (!existsSync(file)) return `${path} does not exist`;
  if (!anchor) return true;
  const text = readFileSync(file, "utf8");
  return new RegExp(`id=["']${anchor.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}["']`).test(text) || `#${anchor} not found in ${path}`;
}

function findCycle(byId) {
  const state = new Map();
  const visit = (id, path) => {
    if (state.get(id) === "done") return null;
    if (state.get(id) === "visiting") return [...path.slice(path.indexOf(id)), id];
    state.set(id, "visiting");
    for (const dep of byId.get(id)?.header.blockedBy ?? []) {
      if (!byId.has(dep)) continue;
      const cycle = visit(dep, [...path, id]);
      if (cycle) return cycle;
    }
    state.set(id, "done");
    return null;
  };
  for (const id of byId.keys()) {
    const cycle = visit(id, []);
    if (cycle) return cycle;
  }
  return null;
}

export function validateTickets(tickets, root = REPO_ROOT) {
  const errors = [];
  const byId = new Map();
  const prototypePath = join(root, PROTOTYPE);
  const prototype = existsSync(prototypePath) ? readFileSync(prototypePath, "utf8") : null;

  for (const t of tickets) {
    const name = t.file.slice(0, -3);
    const err = (msg) => errors.push(`${t.file}: ${msg}`);
    if (t.error) {
      err(t.error);
      continue;
    }
    const h = t.header;
    if (h.id !== name) err(`id ${h.id} does not match the file name ${name}`);
    byId.set(h.id ?? name, t);

    for (const [key, check] of Object.entries(FIELDS)) {
      if (!(key in h)) {
        err(`missing header field "${key}"`);
        continue;
      }
      const verdict = check(h[key]);
      if (verdict !== true) err(`${key} ${verdict} (got ${JSON.stringify(h[key])})`);
    }
    for (const key of Object.keys(h)) if (!(key in FIELDS)) err(`unknown header field "${key}"`);

    if (t.h1 !== `${h.id} — ${h.title}`) err(`first heading must be "# ${h.id} — ${h.title}"`);
    for (const section of REQUIRED_SECTIONS) if (!t.sections.includes(section)) err(`missing section "## ${section}"`);

    if (ACTIVE.includes(h.status) && !h.branch) err(`status ${h.status} needs a branch`);
    if (h.branch && h.id && !h.branch.startsWith(`${h.id.toLowerCase()}-`)) err(`branch must start with ${h.id.toLowerCase()}-`);
    if (["review", "done"].includes(h.status) && !h.pr) err(`status ${h.status} needs a pr`);
    if (h.status === "done" && !h.completed) err("status done needs a completed date");

    for (const ref of Array.isArray(h.spec) ? h.spec : []) {
      const verdict = anchorExists(root, ref);
      if (verdict !== true) err(`spec ${verdict}`);
    }
    for (const ref of Array.isArray(h.prototype) ? h.prototype : []) {
      if (prototype === null) {
        err(`prototype ${PROTOTYPE} does not exist`);
        break;
      }
      for (const id of ref.match(/#[\w-]+/g) ?? []) {
        if (!new RegExp(`id=["']${id.slice(1)}["']`).test(prototype)) err(`prototype ${id} not found in ${PROTOTYPE}`);
      }
    }
  }

  for (const t of tickets) {
    for (const dep of t.header.blockedBy ?? []) {
      if (dep === t.header.id) errors.push(`${t.file}: blocked by itself`);
      else if (!byId.has(dep)) errors.push(`${t.file}: blockedBy ${dep} does not exist`);
    }
  }
  const cycle = findCycle(byId);
  if (cycle) errors.push(`dependency cycle: ${cycle.join(" → ")}`);
  return errors;
}

// ---------- planning ----------

const num = (t) => Number(t.header.id.slice(4));
const order = (a, b) =>
  a.header.phase - b.header.phase || a.header.priority.localeCompare(b.header.priority) || num(a) - num(b);

export function readyTickets(tickets) {
  const status = new Map(tickets.map((t) => [t.header.id, t.header.status]));
  return tickets
    .filter((t) => !t.error && t.header.status === "todo")
    .filter((t) => (t.header.blockedBy ?? []).every((dep) => status.get(dep) === "done"))
    .sort(order);
}

// An in-progress ticket whose branch is checked out belongs to another session: it is `taken`.
export function nextWork(tickets) {
  const isTaken = (t) => t.header.status === "in-progress" && Boolean(t.checkedOut);
  const inFlight = tickets.filter((t) => ACTIVE.includes(t.header.status)).sort(order);
  return {
    active: inFlight.filter((t) => !isTaken(t)),
    taken: inFlight.filter(isTaken),
    next: readyTickets(tickets)[0] ?? null,
  };
}

// ---------- CLI ----------

function row(t) {
  const h = t.header;
  const where = t.source === "working tree" ? "" : `  [${t.source}]`;
  return `${h.id}  ${h.status.padEnd(11)} ph${h.phase} ${h.priority} ${h.estimate.padEnd(2)} ${h.area.padEnd(4)}  ${h.title}${where}`;
}

function parseFlags(args) {
  const flags = {};
  const rest = [];
  for (let i = 0; i < args.length; i++) {
    if (args[i].startsWith("--")) flags[args[i].slice(2)] = args[i + 1]?.startsWith("--") ? true : (args[++i] ?? true);
    else rest.push(args[i]);
  }
  return { flags, rest };
}

function main(argv) {
  const [command, ...args] = argv;
  const { flags, rest } = parseFlags(args);
  const root = flags.root ? resolve(flags.root) : REPO_ROOT;
  const tickets = loadTickets(root, { branches: command !== "check" });
  const json = (value) => console.log(JSON.stringify(value, null, 2));
  const summary = (t) => ({ ...t.header, source: t.source, checkedOut: t.checkedOut ?? null, path: t.path });

  switch (command) {
    case "list": {
      const shown = tickets
        .filter((t) => !t.error)
        .filter((t) => !flags.status || t.header.status === flags.status)
        .filter((t) => !flags.phase || t.header.phase === Number(flags.phase));
      if (flags.json) return json(shown.map(summary));
      shown.forEach((t) => console.log(row(t)));
      return;
    }
    case "ready": {
      const ready = readyTickets(tickets);
      if (flags.json) return json(ready.map(summary));
      ready.forEach((t) => console.log(row(t)));
      if (ready.length === 0) console.log("No ready ticket.");
      return;
    }
    case "next": {
      const { active, taken, next } = nextWork(tickets);
      if (flags.json) return json({ active: active.map(summary), taken: taken.map(summary), next: next && summary(next) });
      if (active.length) {
        console.log("Active:");
        active.forEach((t) => console.log(`  ${row(t)}${t.header.pr ? `  ${t.header.pr}` : ""}`));
      }
      if (taken.length) {
        console.log("Taken by another session:");
        taken.forEach((t) => console.log(`  ${row(t)}  in ${t.checkedOut}`));
      }
      console.log(next ? `Next ready:\n  ${row(next)}` : "No ready ticket.");
      return;
    }
    case "show": {
      const t = tickets.find((x) => x.header.id === rest[0]);
      if (!t) throw new Error(`unknown ticket ${rest[0]}`);
      return json(summary(t));
    }
    case "set": {
      const [id, ...pairs] = rest;
      if (!/^TAD-\d{3}$/.test(id ?? "") || pairs.length === 0) throw new Error("usage: set TAD-042 key=value [key=value…]");
      const path = join(root, "tickets", `${id}.md`);
      const fields = Object.fromEntries(
        pairs.map((p) => {
          const at = p.indexOf("=");
          if (at < 1) throw new Error(`expected key=value, got ${p}`);
          return [p.slice(0, at), p.slice(at + 1)];
        }),
      );
      writeFileSync(path, setFields(readFileSync(path, "utf8"), fields));
      console.log(`${id}: ${Object.entries(fields).map(([k, v]) => `${k}=${v}`).join(" ")}`);
      return;
    }
    case "check": {
      const errors = validateTickets(tickets, root);
      if (errors.length) {
        errors.forEach((e) => console.error(`✖ ${e}`));
        process.exitCode = 1;
      } else console.log(`✓ ${tickets.length} tickets valid`);
      return;
    }
    default:
      console.error("usage: node scripts/tickets.mjs <list|next|ready|show|set|check> [--json] [--status s] [--phase n]");
      process.exitCode = 2;
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    main(process.argv.slice(2));
  } catch (error) {
    console.error(`✖ ${error.message}`);
    process.exitCode = 1;
  }
}
