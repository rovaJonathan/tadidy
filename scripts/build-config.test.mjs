// Guard test: the signing and bundle configuration cannot drift silently (TAD-008).
// A hard-coded signing identity would bind every build to one machine; a missing key would silently
// change the app's behaviour (Dock icon, minimum macOS version).
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (path) => readFileSync(join(root, path), "utf8");

const tauriConf = JSON.parse(read("src-tauri/tauri.conf.json"));
const packageJson = JSON.parse(read("package.json"));
const infoPlist = read("src-tauri/Info.plist");

// The value following `<key>name</key>` in the plist: `<true/>`, or the text of a `<string>`.
function plistValue(plist, key) {
  const match = plist.match(new RegExp(`<key>${key}</key>\\s*(?:<(true|false)/>|<string>([^<]*)</string>)`));
  return match ? (match[1] ?? match[2]) : undefined;
}

// A signing identity is "unset" when null or absent: Tauri then reads APPLE_SIGNING_IDENTITY.
const signingIdentityUnset = (conf) => (conf.bundle?.macOS?.signingIdentity ?? null) === null;

test("the signing identity comes from APPLE_SIGNING_IDENTITY, never from the repository", () => {
  assert.ok(signingIdentityUnset(tauriConf), `signingIdentity is hard-coded: ${tauriConf.bundle.macOS.signingIdentity}`);
});

test("the guard flags a hard-coded signing identity", () => {
  assert.equal(signingIdentityUnset({ bundle: { macOS: { signingIdentity: "Tadidy Dev" } } }), false);
  assert.equal(signingIdentityUnset({ bundle: { macOS: { signingIdentity: null } } }), true);
  assert.equal(signingIdentityUnset({ bundle: { macOS: {} } }), true);
});

test("the app requires macOS 13.0", () => {
  assert.equal(tauriConf.bundle.macOS.minimumSystemVersion, "13.0");
});

test("the bundle identifier is app.tadidy", () => {
  assert.equal(tauriConf.identifier, "app.tadidy");
});

test("the bundle builds the app and the dmg", () => {
  assert.deepEqual([...tauriConf.bundle.targets].sort(), ["app", "dmg"]);
});

test("the app is not sandboxed", () => {
  assert.equal(tauriConf.bundle.macOS.entitlements, undefined);
  assert.doesNotMatch(infoPlist, /com\.apple\.security\.app-sandbox/);
});

test("the app stays a menu-bar app (LSUIElement)", () => {
  assert.equal(plistValue(infoPlist, "LSUIElement"), "true");
});

test("the app declares a copyright notice", () => {
  const copyright = plistValue(infoPlist, "NSHumanReadableCopyright");
  assert.ok(copyright && copyright.trim().length > 0, "NSHumanReadableCopyright is missing or empty");
});

test("build:universal builds for both architectures", () => {
  assert.equal(packageJson.scripts["build:universal"], "tauri build --target universal-apple-darwin");
});
