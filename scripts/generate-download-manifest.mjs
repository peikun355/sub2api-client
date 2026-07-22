import { createHash } from "node:crypto";
import { readdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";

const [directory = "release-assets", output = "client-download-manifest.json"] = process.argv.slice(2);
const version = (process.env.RELEASE_VERSION || "").replace(/^(?:community-)?v/, "");
const channel = process.env.RELEASE_CHANNEL === "community" ? "community" : "stable";
const cdnBase = (process.env.CDN_BASE_URL || "").replace(/\/$/, "");
const repository = process.env.GITHUB_REPOSITORY || "peikun355/sub2api-client";
const tag = process.env.RELEASE_TAG || (version ? `v${version}` : "");
if (!version || !tag) throw new Error("RELEASE_VERSION and RELEASE_TAG are required");
const githubReleaseBase = `https://github.com/${repository}/releases/download`;
const primaryBase = cdnBase || githubReleaseBase;

const definitions = [
  [/Windows\.msi$/i, "windows", "x64", "MSI", true],
  [/Windows-Portable\.zip$/i, "windows", "x64", "Portable ZIP", true],
  [/Windows-arm64\.msi$/i, "windows", "arm64", "MSI", false],
  [/Windows-arm64-Portable\.zip$/i, "windows", "arm64", "Portable ZIP", false],
  [/macOS\.dmg$/i, "macos", "universal", "DMG", true],
  [/Linux-x86_64\.AppImage$/i, "linux", "x86_64", "AppImage", true],
  [/Linux-x86_64\.deb$/i, "linux", "x86_64", "DEB", true],
  [/Linux-arm64\.AppImage$/i, "linux", "arm64", "AppImage", false],
  [/Linux-arm64\.deb$/i, "linux", "arm64", "DEB", false],
];

const files = await readdir(directory);
const assets = [];
for (const [pattern, os, arch, kind] of definitions) {
  const file = files.find((candidate) => pattern.test(candidate));
  if (!file) continue;
  const digest = createHash("sha256").update(await readFile(path.join(directory, file))).digest("hex");
  assets.push({
    os,
    arch,
    kind,
    url: `${primaryBase}/${tag}/${encodeURIComponent(file)}`,
    fallback_url: `${githubReleaseBase}/${tag}/${encodeURIComponent(file)}`,
    sha256: digest,
  });
}

for (const [pattern, os, arch, kind, required] of definitions) {
  if (required && !assets.some((asset) => asset.os === os && asset.arch === arch && asset.kind === kind)) {
    throw new Error(`Missing required stable asset matching ${pattern}`);
  }
}

await writeFile(output, `${JSON.stringify({ version, channel, published_at: new Date().toISOString(), assets }, null, 2)}\n`);
