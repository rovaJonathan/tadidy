import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
  parseTicket,
  serializeValue,
  setFields,
  validateTickets,
  loadTickets,
  nextWork,
  readyTickets,
} from "./tickets.mjs";

const SECTIONS = [
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

function ticketText(id, fields = {}, { title = "Do something", body } = {}) {
  const header = {
    id,
    title,
    status: "todo",
    phase: 1,
    priority: "P0",
    estimate: "M",
    area: "core",
    blockedBy: [],
    spec: [],
    prototype: [],
    branch: "",
    pr: "",
    created: "2026-10-28",
    started: "",
    completed: "",
    ...fields,
  };
  const lines = Object.entries(header).map(([k, v]) => `${k}: ${serializeValue(v)}`);
  const sections = body ?? SECTIONS.map((s) => `## ${s}\n\n- x\n`).join("\n");
  return `---\n${lines.join("\n")}\n---\n\n# ${id} — ${header.title}\n\n${sections}`;
}

function makeRepo() {
  const root = mkdtempSync(join(tmpdir(), "tickets-"));
  mkdirSync(join(root, "tickets"));
  mkdirSync(join(root, "docs/spec"), { recursive: true });
  mkdirSync(join(root, "docs/prototype"), { recursive: true });
  writeFileSync(join(root, "docs/spec/02-features.md"), '<a id="capture"></a>\n## Capture\n| <a id="cap-01"></a>CAP-01 | x |\n');
  writeFileSync(join(root, "docs/prototype/prototype.html"), '<div id="popover"><input id="search-input"></div>');
  return root;
}

function write(root, id, fields, opts) {
  writeFileSync(join(root, "tickets", `${id}.md`), ticketText(id, fields, opts));
}

function git(root, ...args) {
  return execFileSync("git", ["-C", root, ...args], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
}

test("parseTicket reads scalars, quoted strings, numbers and inline arrays", () => {
  const t = parseTicket(
    ticketText("TAD-004", {
      blockedBy: ["TAD-001", "TAD-002"],
      prototype: ["#popover — #search-input"],
      pr: "https://github.com/x/y/pull/4",
    }),
  );
  assert.equal(t.header.id, "TAD-004");
  assert.equal(t.header.phase, 1);
  assert.deepEqual(t.header.blockedBy, ["TAD-001", "TAD-002"]);
  assert.deepEqual(t.header.prototype, ["#popover — #search-input"]);
  assert.equal(t.header.branch, "");
  assert.equal(t.header.pr, "https://github.com/x/y/pull/4");
  assert.ok(t.sections.includes("Acceptance criteria"));
  assert.equal(t.h1, "TAD-004 — Do something");
});

test("parseTicket ignores trailing comments and refuses a missing header", () => {
  const t = parseTicket("---\nid: TAD-001 # the id\nstatus: todo   # todo|done\n---\n# TAD-001 — X\n");
  assert.equal(t.header.id, "TAD-001");
  assert.equal(t.header.status, "todo");
  assert.throws(() => parseTicket("# no header\n"), /header/);
});

test("setFields edits only the named header lines and validates values", () => {
  const before = ticketText("TAD-004");
  const after = setFields(before, { status: "review", pr: "https://github.com/x/y/pull/4", started: "2026-10-29" });
  const t = parseTicket(after);
  assert.equal(t.header.status, "review");
  assert.equal(t.header.pr, "https://github.com/x/y/pull/4");
  assert.equal(t.header.started, "2026-10-29");
  assert.equal(after.split("---\n")[2], before.split("---\n")[2], "body unchanged");
  assert.throws(() => setFields(before, { status: "finished" }), /status/);
  assert.throws(() => setFields(before, { owner: "me" }), /unknown field/);
  assert.deepEqual(parseTicket(setFields(before, { blockedBy: "TAD-001,TAD-002" })).header.blockedBy, ["TAD-001", "TAD-002"]);
});

test("a well-formed backlog validates", () => {
  const root = makeRepo();
  write(root, "TAD-001", { status: "done", pr: "none", completed: "2026-10-28" });
  write(root, "TAD-002", {
    blockedBy: ["TAD-001"],
    spec: ["docs/spec/02-features.md#cap-01"],
    prototype: ["#popover — #search-input"],
  });
  assert.deepEqual(validateTickets(loadTickets(root), root), []);
});

test("validation reports header, reference and status errors", () => {
  const root = makeRepo();
  write(root, "TAD-001", { status: "started", priority: "P9" });
  write(root, "TAD-002", { blockedBy: ["TAD-099"], spec: ["docs/spec/02-features.md#nope", "docs/spec/missing.md"] });
  write(root, "TAD-003", { prototype: ["#popover — #ghost"] });
  write(root, "TAD-004", { status: "review", branch: "" });
  write(root, "TAD-005", { status: "done", pr: "https://x/pull/5", completed: "" });
  writeFileSync(join(root, "tickets", "TAD-006.md"), ticketText("TAD-007"));
  write(root, "TAD-008", {}, { body: "## Context\n\nonly one section\n" });
  const errors = validateTickets(loadTickets(root), root).join("\n");
  assert.match(errors, /TAD-001.*status/);
  assert.match(errors, /TAD-001.*priority/);
  assert.match(errors, /TAD-002.*TAD-099/);
  assert.match(errors, /TAD-002.*#nope/);
  assert.match(errors, /TAD-002.*missing\.md/);
  assert.match(errors, /TAD-003.*#ghost/);
  assert.match(errors, /TAD-004.*branch/);
  assert.match(errors, /TAD-004.*pr/);
  assert.match(errors, /TAD-005.*completed/);
  assert.match(errors, /TAD-006.*file name/);
  assert.match(errors, /TAD-008.*Acceptance criteria/);
});

test("validation reports dependency cycles", () => {
  const root = makeRepo();
  write(root, "TAD-001", { blockedBy: ["TAD-002"] });
  write(root, "TAD-002", { blockedBy: ["TAD-001"] });
  assert.match(validateTickets(loadTickets(root), root).join("\n"), /cycle/);
});

test("ready tickets are ordered by phase, priority, then number", () => {
  const root = makeRepo();
  write(root, "TAD-001", { status: "done", pr: "none", completed: "2026-10-28" });
  write(root, "TAD-002", { phase: 1, priority: "P1", blockedBy: ["TAD-001"] });
  write(root, "TAD-003", { phase: 1, priority: "P0" });
  write(root, "TAD-004", { phase: 0, priority: "P2" });
  write(root, "TAD-005", { phase: 0, blockedBy: ["TAD-006"] });
  write(root, "TAD-006", { phase: 1, status: "blocked" });
  const ids = readyTickets(loadTickets(root)).map((t) => t.header.id);
  assert.deepEqual(ids, ["TAD-004", "TAD-003", "TAD-002"]);
});

test("next lists active tickets and still proposes the next ready one", () => {
  const root = makeRepo();
  write(root, "TAD-001", { status: "review", branch: "tad-001-x", pr: "https://x/pull/1", started: "2026-10-28" });
  write(root, "TAD-002", {});
  const { active, next } = nextWork(loadTickets(root));
  assert.deepEqual(active.map((t) => t.header.id), ["TAD-001"]);
  assert.equal(next?.header.id, "TAD-002");
});

test("the status on a local tad-xxx branch wins over the working tree", () => {
  const root = makeRepo();
  write(root, "TAD-001", {});
  write(root, "TAD-002", { blockedBy: ["TAD-001"] });
  git(root, "init", "-q", "-b", "main");
  git(root, "-c", "user.email=t@t", "-c", "user.name=t", "add", ".");
  git(root, "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-qm", "init");
  git(root, "checkout", "-qb", "tad-001-first");
  write(root, "TAD-001", { status: "done", branch: "tad-001-first", pr: "https://x/pull/1", completed: "2026-10-28" });
  git(root, "-c", "user.email=t@t", "-c", "user.name=t", "commit", "-qam", "close");
  git(root, "checkout", "-q", "main");

  const tickets = loadTickets(root, { branches: true });
  const first = tickets.find((t) => t.header.id === "TAD-001");
  assert.equal(first.header.status, "done");
  assert.equal(first.source, "tad-001-first");
  assert.equal(readFileSync(join(root, "tickets/TAD-001.md"), "utf8").includes("status: todo"), true);
  assert.equal(nextWork(tickets).next?.header.id, "TAD-002");
});
