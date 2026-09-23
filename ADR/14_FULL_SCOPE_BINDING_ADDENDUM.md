# NAI OS 2027 — Binding Full-Scope Implementation Brief

This brief supersedes scope-reducing statements in the prior master instructions and roadmap. **Every deliverable in the
original 00–12 documents and in this addendum is mandatory and must be implemented in depth.** Dependency gates set
execution order, not exclusion. A prototype, design, stub, catalog card, screenshot, static check, or endpoint that
merely returns 200 is not delivery. Preserve working behavior while finishing a complete, installable Windows 11 suite.

## 1. Ground truth and product inventory

- First inspect `D:\_SATIN` and all nested fork/worktree roots. Toastovač/Jarvis is at `D:\_SATIN_AI\Toastovac\jarvis\`
  and is a Python app compiled to an EXE. Record each repository path, remote, SHA, branch, dirty state, upstream
  version, license, tech stack, entry point, Windows installer/update scheme, app ID, existing auth, settings migration,
  runtime and functional baseline.
- The Seelen UI fork has **already been renamed NAI OS**, and its MCP plus REST API **already exist**. Inspect and
  extend the real implementations. Do not rename it back, duplicate those APIs, or claim the rename was newly done.
- Build and release **all** first-party forks found in the inventory, including human BrowserOS, the agent browser, the
  mail fork, office integration, Kelvin products as applicable, and Toastovač. Missing named sources are a blocking
  inventory gap to resolve, not permission to omit the product.
- First-party names: **NAI OS**, **NAI E-Mail**, **NAI Browser**, **NAI Browser Agent**, **NAI Office**, **NAI Voice**
  (Toastovač/Jarvis), plus consistent **NAI {Product}** names for the other discovered forks. Preserve recognizable
  technical identities and user data during migrations. Rename title, executables where safe, AUMID, protocol handlers,
  tray IDs, installers, notifications, deep links, store metadata and update channels with explicit compatibility
  aliases.
- Inventory results become `product-inventory.json` plus a human-readable release plan. Keep all repository commits
  separate and emit one suite release manifest with each immutable Git SHA, package hash, schema version and migration
  revision.
- Windows 11 remains the kernel, driver and compatibility substrate. NAI Core owns the semantic graph, activity state,
  capability registry, identity session binding, package broker, telemetry and event fabric. NAI OS shell, launcher, Weg
  and Fancy Toolbar render it. Existing MCP/REST are external adapters; ACL-restricted versioned named pipes are the
  preferred trusted per-user IPC.

## 2. Complete, dependency-ordered delivery

**A. Stabilize the shell:** finish original `00_P0_RECOVERY_BOOT_LIVENESS.md`, prove real WebViews reach Ready, preserve
a known-good installed version and keep Windows sign-in independent of NAI cloud sign-in.

**B. Shared contracts:** implement durable logical identities, typed object/capability/event schemas, provider
discovery, session-bound IPC, subscriptions, migration/version negotiation, audit and idempotent commands. Rust native
core, C++ native forks, TypeScript/Electron/Tauri apps, Python Toastovač and UNO/.NET bridges use generated contracts
where appropriate.

**C. Cloud identity and all fork connectors:** deploy NAI SSO at `auth.nanotrik.ai` (or the explicit NANOTRIK.AI service
subdomain defined by deployment); wire launcher and every first-party fork. No first-party fork is exempt because its
stack differs. LibreOffice can be a brokered NAI Office wrapper/UNO bridge if native LibreOffice sign-in is not viable.
Details below.

**D. Launcher, store and design:** first paint is the NAI apps hero. Left navigation contains **AI Apps** with the
store. Candy Sugar is embedded as the default icon pack. The launcher UI and top bar are fully reworked with
lavender/teal/azure gradient system. The store installs, updates, launches and uninstalls actual packages; extension is
JSON-driven. Details below.

**E. Voice, telemetry and media:** Toastovač controls NAI OS and store apps, transcribes render output in a Windows
overlay with language detection, handles Korean YouTube/Shorts, and exposes vision/text API contracts. The top bar
measures device/model-specific AI compute and memory honestly.

**F. All original platform commitments:** semantic shell, 50 design features, Activities, Context Capsules, Browser
Agent, Mail commitment graph, Office document objects, V271, club/Mastodon, media queue, QoS/model residency, developer
SDK, agent permissions, API gateway and cross-app workflows in 01–12 are all delivery requirements. A later phase does
not turn them into optional ideas.

The agent should continue through gates until the full suite is delivered; preserve the ability to release each gate and
roll it back. Report concrete blockers with evidence rather than reducing scope.

## 3. NAI SSO: cloud service and local clients

Implement an actual OpenID Connect issuer over HTTPS under NANOTRIK.AI. Use a maintained open-source OIDC identity
engine after evaluating its current version, deployment footprint, administration API and license; avoid writing
OAuth/OIDC cryptography from scratch. The identity service includes: discovery document, JWKS with staged key rotation,
Authorization Code + PKCE, refresh rotation/reuse detection, logout, account recovery, session/device inventory and
revocation, scopes/audiences, audit records, rate limits, user and service administration, migrations, backup/restore,
metrics/traces, and health endpoints. Create infrastructure-as-code, pinned images/digests, TLS, DNS, secrets, database,
backup policy, restore drill, monitoring and deployment/rollback scripts. Verify actual endpoint URLs, issuer and client
registration in the running deployment.

Native public clients (NAI OS, NAI E-Mail, NAI Browser, NAI Browser Agent, Python/EXE Toastovač and any other
first-party fork) use the system browser Authorization Code + PKCE with per-client state/nonce, exact redirect
registration, issuer/audience validation and safe token storage bound to the Windows user. No embedded webview password
entry, hard-coded client secrets in executables, shared refresh token copied between apps, tokens in URL logs, or
trusting SSO because a local process says it is logged in. BrowserOS must separate NAI account identity from third-party
website sessions and preserve browser profile isolation. Agent browser receives scoped capabilities rather than an
unrestricted copy of the human browser's tokens.

Use a local **NAI session broker** for consented, per-app token acquisition. Each app remains its own registered OIDC
client and validates claims; the broker coordinates existing issuer session and provides short-lived scoped results
through authenticated IPC. Token exchange/delegation is permitted only where actually supported and correctly
registered. Server API validates token audience/scopes and authorization on every call. Logout revokes/clears session
and connector state consistently, even if one app is offline; replay local invalidations on reconnect. Multi-user
Windows sessions cannot share tokens.

**Offline and failure behavior:** cloud SSO is never Windows logon. Installed shell, launcher, local applications, local
documents, settings, Jarvis shell control and locally authorized capabilities remain accessible during internet/issuer
outage or loss of the user's phone. Cache an explicitly bounded, signed offline entitlement only for non-sensitive local
features that can be validated on the device; show expiry and which online functions are unavailable. For sensitive
cloud actions, fail visibly, queue drafts locally if appropriate, and never imply that server-held identity alone
provides offline access. Provide independently usable recovery codes stored offline, a second enrolled factor where
supported, and verified account recovery. Test device loss, dead battery, network loss, service outage and expired
entitlement. No NAI SSO outage may lock a user out of Windows or their local files.

**Per-stack adapter contract:**

| Stack                                      | Required implementation                                                                                                                                    |
| ------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Rust/Tauri Seelen/NAI OS and NAI E-Mail    | Shared OIDC Rust client/broker IPC, native browser login, protected token store, scoped API client, logout events                                          |
| C++ fork/native service                    | Native OIDC client library/broker shim, caller authentication, explicit API audience checks, secure memory cleanup                                         |
| TypeScript/Electron/Chromium Browser forks | Main-process broker, system-browser auth, isolated profiles, renderer receives session status only; strict IPC/origin checks                               |
| Python Toastovač EXE                       | Broker client with Windows-user ACL, ephemeral capability credentials, no bundled refresh token or model key                                               |
| LibreOffice                                | NAI Office shell wrapper and UNO bridge with NAI session identity, document capability scoping and offline local editing; no fake native LibreOffice login |
| Web V271/club and other hosted products    | OIDC relying-party integration only after checking existing identity compatibility and migration; no silent account takeover                               |

Acceptance: sign in once and launch each fork with the expected identity; isolated provider tokens; switch Windows user;
revoke a device; log out; restore local access offline; restore issuer backup; verify no unscoped token crosses a
process boundary.

## 4. NAI OS launcher and visual system

Replace the existing apps-menu/launcher experience with a full first-class product surface built on the existing shell.
The **first main hero** presents all discovered/installed first-party NAI apps with launch, resume, update state,
capability peek, pinned order, agent status and last Activity. It must resolve actual installed executable/protocol
paths from the package broker and canonical app identity; show missing first-party apps with a working install/source
resolution path. Provide full keyboard and voice navigation, typeahead, focus management, touch targets, screen-reader
labels and reduced motion.

Left navigation: **Home / NAI Apps**, **AI Apps** (store), **Activities**, **Agents**, **Media**, **Settings**. The home
hero and AI Apps are distinct. Keep Weg, Task Switcher, Fancy Toolbar, previews and existing Seelen extension/plugin
store functional. Plugin and icon-pack management should be surfaced coherently alongside the new store without losing
existing extension compatibility.

**SAP North Star Vision AI palette:** OLED ink `#070710`, lavender core `#8D7CFF`, indigo `#6557FF`, iris `#A778FF`,
teal mint `#56F5C2`, plasma cyan `#4DEBFF` and controlled azure gradient stops. Implement dark/light and SDR/HDR token
variants, contrast governor and focus/agent/attention states. Mix lavender→teal→azure in restrained hero illumination,
active item edge and data accents; maintain quiet, high-density typography. No color-only status. Persist per-user
accessibility controls and per-monitor scale. Audit actual brand tokens from current fork; migrate the old theme rather
than scattering color literals.

**Candy Sugar default pack:** source the upstream Candy Icons/Sugar theme, pin its source commit/archive hash, inspect
GPL-3.0 packaging/attribution obligations and preserve license/source notices. Convert/link its SVG and symbolic
variants into the Seelen-supported icon pack manifest at appropriate sizes; map NAI app IDs, Windows known apps, tray,
launcher and task switcher with fallback to original app icon when mapping absent. Embed the pack in the NAI OS
installer and select it by default on new installs, preserving existing user's explicit chosen icon pack on upgrade.
Icon lookup/cache must cover DPI variants, dark/light, launch/notification and update invalidation. The original Seelen
icon-pack extension flow continues to work.

## 5. JSON-backed AI Apps Store

Use a versioned JSON catalog as the editable source of truth; validate against a checked-in JSON Schema at build/load,
sign the shipped catalog, and separate curated identity metadata from live package availability. Every entry must have:
stable `id`, `name`, `picture` (bundled URI + digest or verified HTTPS URL), `description`, `downloadUrl`,
`distributionType`, publisher, categories, tags, platform/architecture, source package ID and source name,
license/status, version channel, install/upgrade/uninstall/launch strategies, executable discovery, permissions,
optional NAI adapter/capabilities and provenance. `distributionType` is an extensible discriminant: `winget`,
`chocolatey`, `scoop`, `unigetui` (integration when supported), `msstore`, `direct_signed`, `nai`, `vsix`, `model`,
`mcp`, and third-party plugin types. The broker refuses unknown executors while keeping JSON forward-compatible. Do not
confuse a marketing URL with a downloadable installer.

The catalog defines at least **31 candidate apps** (the six examples plus 25 additional options), with live source and
package-ID validation before an entry is marked installable:

| Six requested                                                                              | 25 additional candidates                                                                                                                                                                                                                                                    |
| ------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Perplexity; Microsoft Copilot; Microsoft 365 Copilot; ChatGPT; LM Studio; LM Studio Bionic | Jan; GPT4All; Ollama; AnythingLLM; Open WebUI; LibreChat; Flowise; Dify; n8n; ComfyUI; InvokeAI; Stable Diffusion WebUI; Pinokio; Open Interpreter; Continue; Cline; Roo Code; Aider; Goose; VS Code; VS Code Insiders; VSCodium; Obsidian; Whisper Desktop; NVIDIA ChatRTX |

Maintain the exact user-facing distinction between separate products/editions (especially Copilot vs M365 and LM Studio
vs Bionic). **Candidate** means discovery work required: if no current legitimate installer/manager source exists,
expose a verified official install/open path and visibly mark it non-automated; do not substitute a similarly named app
or invent a WinGet ID. VSIX/MCP/model entries use their own install protocols. The first-party NAI suite appears in the
Home hero and also has catalog entries with NAI-integrated compatibility level.

Install flow: resolve precise source/version → inspect publisher, checksum/signature and permissions → dependency and
disk/architecture check → explicit authorization where required → durable journal → execute with captured exit code,
bounded timeout, progress and restart handling → verify install and launch target → publish semantic capabilities →
health check. Prevent duplicate concurrent install, use cancellation and idempotency keys, log provenance and handle
partial rollback accurately (never promise a Windows restore point will revert every installer). Update/uninstall
preserves user data unless the user chooses removal. Remote catalog refresh uses signature verification, ETag/version
pin and cached last-good JSON. Outage leaves installed apps launchable. Record limitations of Chocolatey community
packaging, Store availability and package-specific licensing; prioritize free/open-source recommendations and label
proprietary/freemium products clearly.

## 6. AI-first Fancy Toolbar and telemetry

The supplied Komorebi YAST screenshot is a compact layout reference: CPU percentage/frequency, RAM used/total, iGPU
load/memory, dGPU load/VRAM and temperature in one strip. Rebuild it as a thinner, less noisy **NAI Fancy Toolbar**
using meaningful AI residency, not raw taskbar counters. The default compact view displays Activity/voice indicator,
CPU, system RAM, available NPU activity where measurable, iGPU shared-memory usage, dGPU dedicated VRAM used/total, and
agent/model status. Expanding an item reveals per-process/model attribution, graphs, provider/driver version and
sampling confidence.

- CPU and memory: Windows performance counters/PDH or ETW/process APIs, physical committed/working set with explicit
  definitions, CPU total/frequency at modest refresh interval. Do not label all committed memory as AI.
- Intel Arc/Xe and NVIDIA: discover adapters by LUID and PCI identity; correlate DXGI/WDDM per-engine/per-process
  dedicated and shared allocations and NVIDIA NVML data where supported. Show NVML-reported dedicated VRAM and
  per-process data when actually available, including WDDM visibility limits.
- Intel NPU/Snapdragon NPU: implement vendor/OS telemetry adapters with capability detection and version-gated
  availability; show `NPU: unavailable/unsupported` rather than claiming generic CPU/NPU equivalence. Verify access and
  support on real Intel Core Ultra and Copilot+ Snapdragon devices when available.
- AI workload accounting: adapters from LM Studio, Ollama, NVIDIA/Triton/TensorRT-LLM if used, OpenVINO, Qwen3-VL,
  Whisper and Toastovač publish model ID, process PID, provider, backend, loaded/unloaded, context/KV cache bytes when
  runtime can report them, allocated device/shared host memory and inference throughput. Correlate with OS counters by
  process/session, deduplicate shared weights, report `unattributed`/`estimated` distinctly. GPU driver counters alone
  cannot reliably identify KV cache.
- Update UI from native events plus bounded sampled metrics; adaptive sampling 0.5–2 s when expanded, 3–5 s
  compact/idle; offscreen suspend; no UI-thread blocking, polling leak or telemetry crash taking down Fancy Toolbar. CPU
  overhead and memory cost are measured on actual devices. Settings control display density and privacy. Show source and
  timestamp of last valid sample.

## 7. Toastovač/Jarvis, Windows audio and caption overlay

Preserve the existing Whisper, LLM, STT and TTS paths and HA Voice PE support. Add a named-pipe NAI Voice adapter to the
compiled EXE, with identity, health, capability registration and typed event stream. It consumes the NAI semantic graph
for deictic references and calls the same command router as keyboard/MCP/REST. Voice actions include launch/focus
first-party apps; search/install/launch AI store apps (install policy applies); switch Activities; manage Weg/window
layout; dictate into target; control media/Shorts/PiP; query telemetry; pause/cancel agents. Provide explicit
acknowledgments tied to actual accepted actions, error reason, interrupt/barge-in, audio privacy and cancellation.

Separate **capture sources**: (1) selected Windows default recording input for speech commands and microphone captions,
(2) **WASAPI render-endpoint loopback** for all system audio, (3) Windows-supported process-loopback for browser process
tree when available. Default input cannot capture YouTube output by itself. Device change and default endpoint migration
must rebind without losing an in-progress session. Implement echo/reference gating and source tags so TTS/YouTube output
cannot be mistaken for Jarvis commands. Detect silence and do not record background audio continuously without a visible
enable state.

Build a Windows 11 caption overlay: always-on-top but click-through or interaction-safe when passive, per-monitor
placement, font/opacity/line count, high contrast, privacy indicator, keyboard toggle, transcribed language badge and
optional translation track. Low-latency chunked Whisper STT with VAD, punctuation, timestamp alignment, language
auto-detection, and Korean support for influencers/YouTube Shorts; verify Korean proper nouns, code switching and short
clips with real samples. Preserve original and translated text separately with confidence and copy/export controls. All
captions stay local by default. Respect YouTube/player and OS capture rules; use explicit visible playback, no hidden
downloader. When source isolation or language confidence is weak, label it rather than silently switching STT provider.

## 8. YouTube, Shorts and model gateway

Complete original `06_YOUTUBE_SHORTS_VISUAL_ENGINE.md` as functional playback/queue/PiP/voice integration. Candidate
search must not equate a short-duration search result with a native Shorts classification. Maintain provenance of
video/channel and queue reason, user feedback, typed media object and timestamps. Browser-based observation of currently
playing video samples frames at configurable `N` or time interval with bounded capture resolution, frame
queue/backpressure and ephemeral frame lifetime. Use a visible, authorized player surface and documented capture
boundary. Never covertly download streams or turn this into a biometric identity database.

Define the NAI multimodal model gateway now, even if the existing OpenVINO Qwen3-VL runtime is wired in the **next
task**:

- Authenticated loopback service or named-pipe adapter; OpenAI-compatible `/v1/models` and `/v1/chat/completions`,
  streaming SSE for text when backend supports it, `messages[].content` text plus image URL/data input with size/type
  policy, timeout/cancel, error/status shape, request ID and model enumeration.
- Explicit capability metadata: `text`, `vision`, max image count/bytes/resolution, accepted MIME, context/output
  bounds, streaming support, backend and version. A separate typed `media.analyze_frames` service accepts
  `videoId/source`, timestamps, image references, sampling policy and returns timecoded OCR/scene/answer/evidence. Do
  not pretend the OpenAI chat-completions schema itself contains a standardized video analysis API.
- Provide a provider interface and contract fixture for the existing **OpenVINO Qwen3-VL** integration; do not claim it
  is connected until it executes real inference. Authentication, rate/concurrency controls, memory budget, cancellation,
  metrics, traces and cleanup apply. No silent provider/model fallback.
- NVIDIA-first options, after verifying architecture, license and real hardware fit: Triton Inference Server for
  NVIDIA-capable serving, TensorRT-LLM for supported models/backends, NeMo for relevant speech/model workflows and NVML
  for dGPU telemetry. They are optional implementation technologies, **not** exceptions to the mandated feature. Avoid
  enterprise-only/paywalled or free-trial-only runtime requirements; commercial packaging requires dependency/license
  review. OpenVINO remains the existing Qwen3-VL backend and Intel/Snapdragon lanes remain first-class.

## 9. Reliability, security, delivery evidence

Each provider has health state, typed failures, bounded retry/backoff, graceful restart and event replay. NAI Core and
shell boot with SSO cloud down, network down, store source down, device unplugged or a model server unavailable. No
silent AI fallback. User settings and local documents survive every upgrade/downgrade path. Roll back schema-compatible
releases via signed known-good packages; incompatible migrations need forward compatibility or restoration plan. Feature
flags and canary channel isolate new providers. Track structured logs, traces, error rates, boot/Ready timelines,
end-to-end command latency, AI memory estimation confidence, installer success and per-app login health. Secrets are
never in catalog JSON, renderer state, URL logs or support bundles.

**Mandatory evidence:** repository matrix with starting/ending SHAs; build commands and artifacts for every fork;
packaged installed desktop run; identity deployment/backup restore and OIDC discovery; cross-app sign-in and offline
tests; actual app installation/update/uninstall from catalog; named app launch by UI and Jarvis; default icon pack on
clean install and upgrade preservation; toolbar counters compared to OS/vendor sources; YouTube Korean caption sample
with timestamps; sampled frame metadata and Qwen3-VL gateway contract (clearly say if model wiring awaits the next
task); failure injection; accessibility/performance; screenshots/video only as supplementary evidence. Keep a row-by-row
status table for the original 00–12 and this brief; mark unimplemented rows explicitly. Completion requires every row,
except the explicitly deferred physical connection of the existing Qwen3-VL runtime, to run in the shipped suite.
