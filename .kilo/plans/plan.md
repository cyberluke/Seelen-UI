# Seelen UI Build & Release Verification Plan

## 1. Verified facts (from inspection)

- HEAD: `f1b0672c8f98d516800ff449717ea6733fdb1031` (v2.8.6), clean tree.
- Cargo workspace: `libs/{core,positioning,slu-ipc,slu-macros,utils}` + `src` + `src/hook_dll` (no `libs/selu`).
- 3 binaries in `src/Cargo.toml`: `seelen-ui` (background), `slu` (CLI), `slu-service` (pipe RPC, token `__local__`).
- Release profile: opt-level z, LTO, codegen-units=1 — **passes** (8.6 s + 14.6 s earlier). Debug:
  `LLVM ERROR: unable to inline function` on some crates → always `--profile release`.
- `src/build.rs`: emits `../SHA256SUMS` + `.sig` (release: minisign via `TAURI_SIGNING_PRIVATE_KEY`/`_PASSWORD`; debug:
  placeholder `NOT SIGNED NEEDED FOR DEBUG`); bakes `SUL_GIT_SHA`, `SUL_GIT_DIRTY`, `SUL_BUILD_TIME`, `SUL_TARGET`,
  `SUL_STATIC_HASH`, `SUL_FRONTEND_HASH`. Dist dirs checked: `src/static/dist`, root `dist`.
- `src/tauri.conf.json` `beforeDevCommand`: `npm run tauri -- dev --config ...`; `beforeBuildCommand`:
  `npm run build:ui`; after `tauri build` copies `src/static/` → `../build/`; binary
  `../build/{target-triple}/seelen-ui.exe` + `slu`/`slu-service` clones + `../{static,gen}`.
- `scripts/build.ts` = `npm i --no-fund && npm run build:ui` (ui build = tsx scripts/build.ts; `preinstall` also runs
  `build:lib` via deno for `@seelen-ui/lib`).
- Provenance/commands in `../.kilo` style docs absent; runtime commands exist in binaries (`slu runtime provenance`,
  `slu update --wait`).

## Plan

### Phase 1 — UI bundles (no heavy build gate; needed as tauri input)

1. `npm install --no-fund` (repo root; runs `preinstall` → `build:lib` deno).
2. `npm run build:ui` (tsx → esbuild svelte bundles into `src/static/dist`).

- Verify: diff of generated dist; `dist` exists; no TS errors from `npm run type-check` (optional if time; targeted
  svelte-check is enough).

### Phase 2 — Rust binaries (HEAVY build gate, exact HEAD SHA)

3. `cargo build --profile release` (from workspace root; §8 gate for this SHA). If a prior valid receipt for this SHA +
   profile + same toolchain exists in `.git`-local metadata (`.kilo`/verification-state), reuse instead of rebuild.

- Verify: `build/x86_64-pc-windows-gnu/seelen-ui.exe`, `slu.exe`, `slu-service.exe` exist; `slu runtime provenance`
  prints SHA `f1b0672c8f98d516800ff449717ea6733fdb1031`, dirty flag, target; `SHA256SUMS`/`SHA256SUMS.sig` present in
  target dir.

### Phase 3 — Tauri production build

4. `npm run tauri -- build` — runs `build:ui`, then `cargo tauri build`; copies `src/static` → `build/`, renames exe
   into `build/{triple}/`.

- Verify: `build/x86_64-pc-windows-gnu/{seelen-ui.exe,slu.exe,slu-service.exe}` present, `build/static` + `gen`
  populated.

### Phase 4 — Runtime smoke

5. Launch `seelen-ui.exe` (background); probe with `slu runtime provenance | version | get-right-components` and
   `slu update --wait` (bounded `--max-time` 10 per probe — bounded network probes allowed; no long loops). Named-pipe
   RPC: `slu-service` with `SLU_SERVICE_CONNECTION_TOKEN=__local__`.

- Expected: provenance matches HEAD SHA + dirty state; right-components list returns JSON; updater reports version
  2.8.6.
- Stop server/binaries after probe is complete (§7: dev servers are shared infra — reuse existing, one instance).

### Phase 5 — Git/release gate (§8/§9)

6. Only if CTO workflow requires: `git add` intended files; commit with repo convention (`feat:`/`fix:`); `git push`
   after build gate for exact final SHA; on GHCR (`.github/workflows/`): build from committed SHA, tag with
   `f1b0672c...`, verify digest via `docker buildx imagetools inspect`/`gh` if requested.

## Caching/reuse

- Phase 2 uses cached release receipts per (SHA, `--profile release`, toolchain signature, clean tree). 16/463 crates
  already cached from earlier runs; rest compiled fresh.
- Phase 1 dist hashes are part of `SUL_FRONTEND_HASH`; unchanged dist → same hash → valid evidence without re-running
  heavy build.

## Verification matrix (commands actually run)

- `cargo build --profile release` → success terminal evidence.
- `slu runtime provenance` → SHA/dirty/target match.
- `slu runtime get-right-components` / `update` → JSON + version 2.8.6.
- `build/{triple}/` artifact listing.
- No heavy builds without this matrix; targeted checks first per §8 gate.

## Troubleshooting knowledge (from this workspace)

- debug fails → use release profile.
- `TAURI_SIGNING_PRIVATE_KEY: not found` → export key (minisign base64) + `_PASSWORD`.
- `cargo tauri: command not found` → use `npm run tauri -- <cmd>` (tauri installed as npm dep 2.11.2).
- `npm error code 2` in `build:ui` → fix per nested error; rerun same command.

## Acceptance

- Phase 2 binaries built for HEAD SHA; provenance verified; Phase 3 `tauri build` completes and its copied artifacts
  match; Phase 4 probes return expected values. Cached receipts reused where valid (no duplicate heavy builds).
