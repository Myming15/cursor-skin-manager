import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import {
  assembleReleaseAssets,
  buildAssetNames,
  extractChangelogSection,
  validateVersionMap,
  verifyReleaseAssets,
} from "./release-tools.mjs";

test("validates synchronized dry-run versions", () => {
  const version = validateVersionMap(
    {
      package: "0.1.12",
      lock: "0.1.12",
      cargo: "0.1.12",
      tauri: "0.1.12",
    },
    { mode: "dry-run" }
  );

  assert.equal(version, "0.1.12");
});

test("rejects mismatched versions and tags", () => {
  assert.throws(
    () => validateVersionMap({ package: "0.1.12", cargo: "0.1.11" }, { mode: "dry-run" }),
    /versions do not match/
  );
  assert.throws(
    () =>
      validateVersionMap(
        { package: "0.1.12", cargo: "0.1.12" },
        { mode: "tag", refName: "v0.1.13" }
      ),
    /does not match version/
  );
});

test("extracts one changelog section", () => {
  const markdown =
    "# Changes\n\n## [未发布]\n\n- New item\n\n## [0.1.11] - 2026-07-13\n\n- Old item\n";
  assert.equal(extractChangelogSection(markdown, "未发布"), "- New item");
  assert.equal(extractChangelogSection(markdown, "0.1.11"), "- Old item");
});

test("assembles stable asset names and SHA-256 checksums", async () => {
  const root = mkdtempSync(join(tmpdir(), "cursor-skin-manager-release-"));

  try {
    const installer = join(root, "installer.exe");
    const portable = join(root, "portable.exe");
    const output = join(root, "output");
    writeFileSync(installer, "installer-bytes");
    writeFileSync(portable, "portable-bytes");

    const result = await assembleReleaseAssets({
      installerPath: installer,
      portablePath: portable,
      outputDir: output,
      version: "0.1.12",
      releaseNotes: "Release notes\n",
    });
    const names = buildAssetNames("0.1.12");
    const checksumFile = readFileSync(join(output, names.checksumName), "ascii");
    const installerHash = createHash("sha256").update("installer-bytes").digest("hex");
    const portableHash = createHash("sha256").update("portable-bytes").digest("hex");

    assert.equal(result.names.artifactName, "cursor-skin-manager-0.1.12-windows-x64");
    assert.match(checksumFile, new RegExp(`${installerHash}  ${names.setupName}`));
    assert.match(checksumFile, new RegExp(`${portableHash}  ${names.portableName}`));
    assert.equal(readFileSync(join(output, names.releaseNotesName), "utf8"), "Release notes\n");

    const verified = await verifyReleaseAssets({ inputDir: output, version: "0.1.12" });
    assert.equal(verified.assets.length, 2);
    assert.ok(verified.assets.every((asset) => asset.bytes > 0));

    writeFileSync(join(output, names.portableName), "changed-portable-bytes");
    await assert.rejects(
      verifyReleaseAssets({ inputDir: output, version: "0.1.12" }),
      /SHA-256 mismatch/
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
