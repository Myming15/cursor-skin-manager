#!/usr/bin/env node

import { parseArgs } from "node:util";
import {
  assembleReleaseAssets,
  loadReleaseContext,
  resolveBuiltAssetPaths,
  verifyReleaseAssets,
  writeGitHubOutputs,
} from "./release-tools.mjs";

const { positionals, values } = parseArgs({
  allowPositionals: true,
  options: {
    mode: { type: "string" },
    ref: { type: "string", default: "" },
    root: { type: "string", default: process.cwd() },
    output: { type: "string", default: "release-output" },
  },
});

async function main() {
  const command = positionals[0];
  if (!command || !["validate", "prepare", "verify"].includes(command)) {
    throw new Error(
      "Usage: node scripts/release.mjs <validate|prepare|verify> --mode <dry-run|tag> [--ref vX.Y.Z]"
    );
  }
  if (!values.mode) {
    throw new Error("The --mode option is required.");
  }

  const context = loadReleaseContext(values.root, { mode: values.mode, refName: values.ref });
  writeGitHubOutputs(context);

  if (command === "validate") {
    console.log(
      JSON.stringify(
        {
          mode: context.mode,
          version: context.version,
          tag: context.tag,
          artifactName: context.names.artifactName,
          versions: context.versions,
        },
        null,
        2
      )
    );
    return;
  }

  if (command === "verify") {
    const result = await verifyReleaseAssets({ inputDir: values.output, version: context.version });
    console.log(JSON.stringify(result, null, 2));
    return;
  }

  const builtAssets = resolveBuiltAssetPaths(context);
  const result = await assembleReleaseAssets({
    ...builtAssets,
    outputDir: values.output,
    version: context.version,
    releaseNotes: context.releaseNotes,
  });
  console.log(JSON.stringify(result, null, 2));
}

main().catch((error) => {
  console.error(`[release] ${error instanceof Error ? error.message : String(error)}`);
  process.exitCode = 1;
});
