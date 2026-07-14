import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  appendFileSync,
  copyFileSync,
  createReadStream,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { basename, join, resolve } from "node:path";

const SEMVER_PATTERN = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/;

function readJson(filePath) {
  return JSON.parse(readFileSync(filePath, "utf8"));
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

export function buildAssetNames(version) {
  const base = `cursor-skin-manager-${version}-windows-x64`;

  return {
    artifactName: base,
    setupName: `${base}-setup.exe`,
    portableName: `${base}-portable.exe`,
    checksumName: `${base}-sha256.txt`,
    releaseNotesName: "release-notes.md",
  };
}

export function validateVersionMap(versionMap, { mode, refName = "" }) {
  if (!["dry-run", "tag"].includes(mode)) {
    throw new Error(`Unsupported release mode: ${mode}`);
  }

  const entries = Object.entries(versionMap);
  if (entries.length === 0) {
    throw new Error("No release versions were provided.");
  }

  for (const [source, version] of entries) {
    if (typeof version !== "string" || !SEMVER_PATTERN.test(version)) {
      throw new Error(`${source} has an invalid semantic version: ${version}`);
    }
  }

  const version = entries[0][1];
  const mismatches = entries.filter(([, candidate]) => candidate !== version);
  if (mismatches.length > 0) {
    const details = entries.map(([source, candidate]) => `${source}=${candidate}`).join(", ");
    throw new Error(`Release versions do not match: ${details}`);
  }

  if (mode === "tag" && refName !== `v${version}`) {
    throw new Error(`Release tag ${refName || "<empty>"} does not match version v${version}.`);
  }

  return version;
}

export function extractChangelogSection(markdown, heading) {
  const normalized = markdown.replace(/\r\n/g, "\n");
  const lines = normalized.split("\n");
  const headingPattern = new RegExp(
    `^## \\[${escapeRegExp(heading)}\\](?:\\s+-\\s+\\d{4}-\\d{2}-\\d{2})?\\s*$`
  );
  const start = lines.findIndex((line) => headingPattern.test(line));

  if (start === -1) {
    throw new Error(`CHANGELOG.md is missing the [${heading}] section.`);
  }

  let end = lines.length;
  for (let index = start + 1; index < lines.length; index += 1) {
    if (lines[index].startsWith("## ")) {
      end = index;
      break;
    }
  }

  const section = lines
    .slice(start + 1, end)
    .join("\n")
    .trim();
  if (!section) {
    throw new Error(`CHANGELOG.md section [${heading}] is empty.`);
  }

  return section;
}

function loadCargoVersion(rootDir, packageName) {
  const metadata = JSON.parse(
    execFileSync(
      "cargo",
      [
        "metadata",
        "--manifest-path",
        "src-tauri/Cargo.toml",
        "--locked",
        "--no-deps",
        "--format-version",
        "1",
      ],
      { cwd: rootDir, encoding: "utf8" }
    )
  );
  const packageMetadata = metadata.packages.find((candidate) => candidate.name === packageName);

  if (!packageMetadata) {
    throw new Error(`Cargo metadata does not contain package ${packageName}.`);
  }

  return packageMetadata.version;
}

export function loadReleaseContext(rootDir, { mode, refName = "" }) {
  const root = resolve(rootDir);
  const packageJson = readJson(join(root, "package.json"));
  const packageLock = readJson(join(root, "package-lock.json"));
  const tauriConfig = readJson(join(root, "src-tauri", "tauri.conf.json"));
  const cargoVersion = loadCargoVersion(root, packageJson.name);
  const versions = {
    "package.json": packageJson.version,
    "package-lock.json": packageLock.version,
    "package-lock.json root package": packageLock.packages?.[""]?.version,
    "src-tauri/Cargo.toml + Cargo.lock": cargoVersion,
    "src-tauri/tauri.conf.json": tauriConfig.version,
  };
  const version = validateVersionMap(versions, { mode, refName });
  const changelog = readFileSync(join(root, "CHANGELOG.md"), "utf8");
  const changelogHeading = mode === "tag" ? version : "未发布";

  if (mode === "tag") {
    const datedHeading = new RegExp(
      `^## \\[${escapeRegExp(version)}\\]\\s+-\\s+\\d{4}-\\d{2}-\\d{2}\\s*$`,
      "m"
    );
    if (!datedHeading.test(changelog.replace(/\r\n/g, "\n"))) {
      throw new Error(`CHANGELOG.md release section [${version}] must include a YYYY-MM-DD date.`);
    }
  }

  const changelogSection = extractChangelogSection(changelog, changelogHeading);
  const releaseNotes =
    mode === "dry-run"
      ? `> Dry Run only. No public GitHub Release was created.\n\n${changelogSection}\n`
      : `${changelogSection}\n`;

  return {
    root,
    mode,
    version,
    tag: `v${version}`,
    packageName: packageJson.name,
    productName: tauriConfig.productName,
    versions,
    releaseNotes,
    names: buildAssetNames(version),
  };
}

export function resolveBuiltAssetPaths(context) {
  return {
    installerPath: join(
      context.root,
      "src-tauri",
      "target",
      "release",
      "bundle",
      "nsis",
      `${context.productName}_${context.version}_x64-setup.exe`
    ),
    portablePath: join(
      context.root,
      "src-tauri",
      "target",
      "release",
      `${context.packageName}.exe`
    ),
  };
}

export async function hashFile(filePath) {
  return new Promise((resolveHash, rejectHash) => {
    const hash = createHash("sha256");
    const stream = createReadStream(filePath);
    stream.on("error", rejectHash);
    stream.on("data", (chunk) => hash.update(chunk));
    stream.on("end", () => resolveHash(hash.digest("hex")));
  });
}

function assertUsableAsset(filePath, label) {
  if (!existsSync(filePath)) {
    throw new Error(`${label} was not generated: ${filePath}`);
  }
  if (statSync(filePath).size === 0) {
    throw new Error(`${label} is empty: ${filePath}`);
  }
}

export async function assembleReleaseAssets({
  installerPath,
  portablePath,
  outputDir,
  version,
  releaseNotes,
}) {
  assertUsableAsset(installerPath, "NSIS installer");
  assertUsableAsset(portablePath, "Portable executable");

  const output = resolve(outputDir);
  const names = buildAssetNames(version);
  mkdirSync(output, { recursive: true });

  const setupOutput = join(output, names.setupName);
  const portableOutput = join(output, names.portableName);
  copyFileSync(installerPath, setupOutput);
  copyFileSync(portablePath, portableOutput);

  const assets = [setupOutput, portableOutput].sort((left, right) =>
    basename(left).localeCompare(basename(right))
  );
  const hashes = [];
  for (const asset of assets) {
    hashes.push({ name: basename(asset), sha256: await hashFile(asset) });
  }

  const checksumOutput = join(output, names.checksumName);
  const checksumContents = `${hashes.map(({ name, sha256 }) => `${sha256}  ${name}`).join("\n")}\n`;
  writeFileSync(checksumOutput, checksumContents, "ascii");
  writeFileSync(join(output, names.releaseNotesName), releaseNotes.replace(/\r\n/g, "\n"), "utf8");

  return {
    outputDir: output,
    names,
    hashes,
  };
}

export async function verifyReleaseAssets({ inputDir, version }) {
  const input = resolve(inputDir);
  const names = buildAssetNames(version);
  const expectedFiles = [
    names.setupName,
    names.portableName,
    names.checksumName,
    names.releaseNotesName,
  ].sort();
  const actualFiles = readdirSync(input, { withFileTypes: true })
    .filter((entry) => entry.isFile())
    .map((entry) => entry.name)
    .sort();

  if (JSON.stringify(actualFiles) !== JSON.stringify(expectedFiles)) {
    throw new Error(
      `Release artifact file set is invalid. Expected [${expectedFiles.join(", ")}], received [${actualFiles.join(", ")}].`
    );
  }

  const checksumPath = join(input, names.checksumName);
  const checksumLines = readFileSync(checksumPath, "ascii").trim().split(/\r?\n/);
  const expectedHashes = new Map();
  for (const line of checksumLines) {
    const match = /^([0-9a-f]{64}) {2}(.+)$/.exec(line);
    if (!match || expectedHashes.has(match[2])) {
      throw new Error(`Invalid SHA-256 entry: ${line}`);
    }
    expectedHashes.set(match[2], match[1]);
  }

  const executableNames = [names.setupName, names.portableName].sort();
  if (JSON.stringify([...expectedHashes.keys()].sort()) !== JSON.stringify(executableNames)) {
    throw new Error("SHA-256 file must contain exactly the setup and portable executables.");
  }

  const assets = [];
  for (const name of executableNames) {
    const filePath = join(input, name);
    assertUsableAsset(filePath, name);
    const sha256 = await hashFile(filePath);
    if (sha256 !== expectedHashes.get(name)) {
      throw new Error(`SHA-256 mismatch: ${name}`);
    }
    assets.push({ name, bytes: statSync(filePath).size, sha256 });
  }

  return { inputDir: input, files: actualFiles, assets };
}

export function writeGitHubOutputs(context, outputPath = process.env.GITHUB_OUTPUT) {
  if (!outputPath) {
    return;
  }

  const values = {
    version: context.version,
    tag: context.tag,
    artifact_name: context.names.artifactName,
    setup_name: context.names.setupName,
    portable_name: context.names.portableName,
    checksum_name: context.names.checksumName,
  };
  const contents = Object.entries(values)
    .map(([key, value]) => `${key}=${value}`)
    .join("\n");
  appendFileSync(outputPath, `${contents}\n`, "utf8");
}
