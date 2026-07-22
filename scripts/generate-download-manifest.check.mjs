import assert from "node:assert/strict";
import { mkdtemp, readFile, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

const directory = await mkdtemp(path.join(os.tmpdir(), "sub2api-manifest-"));
for (const file of [
  "Sub2API-Client-v0.1.0-Windows.msi",
  "Sub2API-Client-v0.1.0-Windows-Portable.zip",
  "Sub2API-Client-v0.1.0-macOS.dmg",
  "Sub2API-Client-v0.1.0-Linux-x86_64.AppImage",
  "Sub2API-Client-v0.1.0-Linux-x86_64.deb",
]) await writeFile(path.join(directory, file), file);

const output = path.join(directory, "manifest.json");
const result = spawnSync(process.execPath, [new URL("./generate-download-manifest.mjs", import.meta.url).pathname, directory, output], {
  env: { ...process.env, RELEASE_VERSION: "v0.1.0", RELEASE_TAG: "v0.1.0", CDN_BASE_URL: "https://cdn.example/client", GITHUB_REPOSITORY: "example/client" },
  encoding: "utf8",
});
assert.equal(result.status, 0, result.stderr);
const manifest = JSON.parse(await readFile(output, "utf8"));
assert.equal(manifest.assets.length, 5);
assert.equal(manifest.channel, "stable");
assert.match(manifest.assets[0].sha256, /^[a-f0-9]{64}$/);
assert.equal(manifest.assets[0].url.startsWith("https://cdn.example/client/v0.1.0/"), true);

const communityOutput = path.join(directory, "community-manifest.json");
const community = spawnSync(process.execPath, [new URL("./generate-download-manifest.mjs", import.meta.url).pathname, directory, communityOutput], {
  env: { ...process.env, RELEASE_CHANNEL: "community", RELEASE_VERSION: "community-v0.1.0-preview.1", RELEASE_TAG: "community-v0.1.0-preview.1", CDN_BASE_URL: "", GITHUB_REPOSITORY: "example/client" },
  encoding: "utf8",
});
assert.equal(community.status, 0, community.stderr);
const communityManifest = JSON.parse(await readFile(communityOutput, "utf8"));
assert.equal(communityManifest.version, "0.1.0-preview.1");
assert.equal(communityManifest.channel, "community");
assert.equal(communityManifest.assets[0].url.startsWith("https://github.com/example/client/releases/download/community-v0.1.0-preview.1/"), true);
