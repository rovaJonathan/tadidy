import { test } from "node:test";
import assert from "node:assert/strict";

import { checkMessage } from "./commit-msg.mjs";

test("accepts conventional commits scoped to a ticket", () => {
  for (const msg of [
    "test(TAD-010): ignore clips written by Tadidy",
    "feat(TAD-010): ignore clips written by Tadidy\n\nBody line.",
    "fix(TAD-003)!: hide the popover on focus loss",
    "wip(TAD-007): keychain spike",
    "chore(TAD-000): set up the development environment",
  ]) {
    assert.equal(checkMessage(msg), null, msg);
  }
});

test("accepts git-generated merge, revert, fixup and squash messages", () => {
  for (const msg of ["Merge pull request #4 from x/tad-004-y", "Merge branch 'main' into tad-004-y", 'Revert "feat(TAD-004): x"', "fixup! feat(TAD-004): x", "squash! test(TAD-004): x"]) {
    assert.equal(checkMessage(msg), null, msg);
  }
});

test("ignores comment lines left by the editor", () => {
  assert.equal(checkMessage("# Please enter the commit message\nfeat(TAD-001): scaffold the app\n# comment"), null);
});

test("refuses a missing ticket scope, an unknown type, a long or empty subject", () => {
  assert.match(checkMessage("feat: scaffold the app"), /TAD-xxx/);
  assert.match(checkMessage("feature(TAD-001): scaffold"), /type/);
  assert.match(checkMessage("feat(TAD-1): scaffold"), /TAD-xxx/);
  assert.match(checkMessage(`feat(TAD-001): ${"x".repeat(80)}`), /72/);
  assert.match(checkMessage("feat(TAD-001): "), /subject/);
  assert.match(checkMessage(""), /empty/);
});
