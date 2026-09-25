// Main build orchestrator
// This file coordinates the build process for NAI OS applications

import { createHash } from "node:crypto";
import { execSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { parseArgs } from "./build/config.ts";
import { extractIcons } from "./build/steps/icons.ts";
import { cleanDist } from "./build/steps/cleanup.ts";
import { discoverEntryPoints, groupEntryPointsByFramework } from "./build/steps/discover.ts";
import { buildReact } from "./build/builders/react.ts";
import { buildSvelte } from "./build/builders/svelte.ts";
import { buildVanilla } from "./build/builders/vanilla.ts";
import { startDevServer } from "./build/server.ts";
import process from "node:process";

/**
 * Main build function
 * Orchestrates the entire build process:
 * 1. Parse command-line arguments
 * 2. Extract icons (in parallel with cleaning)
 * 3. Clean dist directory (in parallel with icons)
 * 4. Discover entry points
 * 5. Build all frameworks in parallel (React, Svelte, Vanilla)
 * 6. Start dev server (if --serve flag is set) - serves the entire dist folder
 */
async function main() {
  const args = await parseArgs();
  console.info(`Build mode: ${args.isProd ? "production" : "development"}`);
  console.info(`Serve: ${args.serve ? "enabled" : "disabled"}\n`);

  // Step 1 & 2: Extract icons; clean dist only on production builds
  if (args.isProd) {
    cleanDist();
  }
  await extractIcons();

  // Step 3: Discover entry points
  const entryPoints = discoverEntryPoints();
  const groupedEntryPoints = groupEntryPointsByFramework(entryPoints);

  console.info(`\nDiscovered entry points:`);
  console.info(`  React: ${groupedEntryPoints.react.length}`);
  console.info(`  Svelte: ${groupedEntryPoints.svelte.length}`);
  console.info(`  Vanilla: ${groupedEntryPoints.vanilla.length}`);
  console.info();

  // Collect all app folders for public file copying
  const allAppFolders = entryPoints.map((entry) => entry.folder);

  // Step 4: Build all frameworks in parallel
  console.time("Total build time");

  await Promise.all([
    buildReact(groupedEntryPoints.react, allAppFolders, args),
    buildSvelte(groupedEntryPoints.svelte, allAppFolders, args),
    buildVanilla(groupedEntryPoints.vanilla, allAppFolders, args),
  ]);

  console.timeEnd("Total build time");

  // Step 5: Emit the immutable build identity (production only) so native,
  // frontend and static artifacts can be proven to belong to one build.
  if (args.isProd) {
    emitBuildIdentity();
  }

  // Step 6: Start dev server if requested (serves entire dist folder)
  if (args.serve) {
    startDevServer();
  }

  console.info("\n✓ Build complete!\n");
}

/**
 * Deterministic build identity shared by every artifact of one production
 * build. Same value the Rust `build.rs` embeds (git sha + commit timestamp),
 * so `nativeBuildId == frontendBuildId == manifestBuildId` at startup.
 */
function emitBuildIdentity() {
  let gitSha = "unknown";
  let buildTime = "";
  try {
    gitSha = execSync("git rev-parse HEAD").toString().trim();
    buildTime = execSync("git log -1 --format=%cI").toString().trim();
  } catch {
    // detached/non-git context: identity falls back to content hash
  }
  const buildId = gitSha !== "unknown" ? `${gitSha}-${buildTime}` : contentHash();

  const identity = {
    buildId,
    gitSha,
    buildTimestamp: buildTime || new Date().toISOString(),
    profile: "release",
    tauriMode: "custom-protocol",
    features: ["custom-protocol"],
  };

  const encoded = JSON.stringify(identity, null, 2);
  fs.writeFileSync(path.join("./dist", "_build-id.json"), encoded);
  if (fs.existsSync("./src/static")) {
    fs.writeFileSync(path.join("./src/static", "_build-id.json"), encoded);
  }
  console.info(`\nBuild identity: ${buildId}`);
}

function contentHash(): string {
  const hash = createHash("sha256");
  for (const file of collect("./dist")) {
    hash.update(fs.readFileSync(file));
  }
  return `local-${hash.digest("hex").slice(0, 16)}`;
}

function collect(dir: string): string[] {
  if (!fs.existsSync(dir)) return [];
  const out: string[] = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) out.push(...collect(full));
    else out.push(full);
  }
  return out.sort();
}

// Run the build
main().catch((error) => {
  console.error("Build failed:", error);
  process.exit(1);
});
