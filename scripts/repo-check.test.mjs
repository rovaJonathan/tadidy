import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, dirname } from "node:path";

import { checkRepo } from "./repo-check.mjs";

function repo(files) {
  const root = mkdtempSync(join(tmpdir(), "repo-check-"));
  for (const [path, content] of Object.entries(files)) {
    mkdirSync(join(root, dirname(path)), { recursive: true });
    writeFileSync(join(root, path), content);
  }
  return root;
}

const skill = (name) => `---\nname: ${name}\ndescription: Does ${name}.\n---\n\n# ${name}\n`;
const GOOD = {
  "CLAUDE.md": "@AGENTS.md\n",
  "AGENTS.md": "Run `pnpm view <pkg> version` (npm tag `latest`), `cargo add <crate>`.\n\n## 7. Tracking\n\n| Status | Meaning |\n|---|---|\n| `todo` | to do |\n\n## 16. Skills\n\n| Skill | When |\n|---|---|\n| `tdd` | always |\n\n## 17. Never\n\nSee [the guide](docs/guide.md).\n",
  "docs/guide.md": "Back to [AGENTS](../AGENTS.md) and [web](https://example.com) and [anchor](#x).\n",
  ".claude/skills/tdd/SKILL.md": skill("tdd"),
  "docs/adr/README.md": "| [0001](0001-first.md) | First |\n",
  "docs/adr/0001-first.md": "# 0001\n",
};

test("a consistent repository passes", () => {
  assert.deepEqual(checkRepo(repo(GOOD)), []);
});

test("CLAUDE.md must sit next to AGENTS.md and only import it", () => {
  const errors = checkRepo(repo({ ...GOOD, "CLAUDE.md": "Some rules\n", "src/CLAUDE.md": "@AGENTS.md\n" })).join("\n");
  assert.match(errors, /CLAUDE\.md.*only contain @AGENTS\.md/);
  assert.match(errors, /src\/CLAUDE\.md.*AGENTS\.md/);
});

test("AGENTS.md must keep the latest-stable rule", () => {
  const errors = checkRepo(repo({ ...GOOD, "AGENTS.md": GOOD["AGENTS.md"].replace("cargo add", "cargo install") })).join("\n");
  assert.match(errors, /cargo add/);
});

test("skills must match their folder and the AGENTS.md table", () => {
  const errors = checkRepo(
    repo({ ...GOOD, ".claude/skills/tdd/SKILL.md": skill("tdd-x"), ".claude/skills/extra/SKILL.md": "no header" }),
  ).join("\n");
  assert.match(errors, /tdd\/SKILL\.md.*name/);
  assert.match(errors, /extra\/SKILL\.md.*header/);
  assert.match(errors, /extra.*AGENTS\.md/);
});

test("only the Skills section of AGENTS.md lists skills", () => {
  assert.doesNotMatch(checkRepo(repo(GOOD)).join("\n"), /todo/);
});

test("every ADR is listed in the ADR index", () => {
  const errors = checkRepo(repo({ ...GOOD, "docs/adr/0002-second.md": "# 0002\n" })).join("\n");
  assert.match(errors, /0002-second\.md.*README/);
});

test("relative markdown links must point to existing files", () => {
  const errors = checkRepo(repo({ ...GOOD, "docs/guide.md": "See [gone](missing.md#a).\n" })).join("\n");
  assert.match(errors, /docs\/guide\.md.*missing\.md/);
});
