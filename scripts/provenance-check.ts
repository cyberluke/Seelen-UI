// Production provenance / artifact-skew gate.
// Run after `tauri build`; exits non-zero when the release artifacts are not
// a coherent `custom-protocol` set.

import fs from "node:fs";
import path from "node:path";
import process from "node:process";

interface Provenance {
  buildId?: string;
  tauriMode?: string;
  profile?: string;
}

function readJson(file: string): Provenance | null {
  if (!fs.existsSync(file)) return null;
  try {
    return JSON.parse(fs.readFileSync(file, "utf8")) as Provenance;
  } catch {
    return null;
  }
}

const targets = [
  "./target/release",
  "./target/x86_64-pc-windows-msvc/release",
  "./target/aarch64-pc-windows-msvc/release",
];
const found: { dir: string; prov: Provenance }[] = [];
for (const dir of targets) {
  const prov = readJson(path.join(dir, "provenance.json"));
  if (prov) found.push({ dir, prov });
}

if (found.length === 0) {
  console.error("FATAL: no provenance.json found; run the build via `npm run build` (tauri build).");
  process.exit(1);
}

const distIdentity = readJson("./dist/_build-id.json");
const staticIdentity = readJson("./src/static/_build-id.json");

let failed = false;
for (const { dir, prov } of found) {
  const profile = prov.profile ?? "unknown";
  const isRelease = profile === "release";
  console.info(`[${dir}] profile=${profile} tauriMode=${prov.tauriMode} buildId=${prov.buildId}`);
  if (isRelease) {
    if (prov.tauriMode !== "custom-protocol") {
      console.error(
        `FATAL: production runtime is using Tauri development asset mode. Expected custom-protocol. (${dir})`,
      );
      failed = true;
      continue;
    }
    if (!distIdentity || !staticIdentity) {
      console.error("FATAL: missing _build-id.json in dist/ or src/static/ (stale frontend?).");
      failed = true;
      continue;
    }
    if (distIdentity.buildId !== staticIdentity.buildId) {
      console.error(
        `FATAL: artifact skew: frontend=${distIdentity.buildId} static=${staticIdentity.buildId}`,
      );
      failed = true;
      continue;
    }
    if (prov.buildId && prov.buildId !== distIdentity.buildId) {
      console.error(
        `FATAL: artifact skew: native=${prov.buildId} frontend=${distIdentity.buildId}`,
      );
      failed = true;
      continue;
    }
    console.info(`[${dir}] OK: native/frontend/static share buildId=${prov.buildId}`);
  }
}

if (failed) process.exit(1);
console.info("Provenance gate: green (tauriMode=custom-protocol, single build identity).");
