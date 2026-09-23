# P0 — Recover Seelen UI Before Any NAI OS Feature Work

This is the first task. Do not add design work, YouTube integration, store code, BrowserOS adapters, or new shell
visuals until the current Seelen fork reliably boots all core WebViews and survives grouped-window interaction.

## Current proven failure class

The observed runtime starts the Rust/Tauri backend successfully, starts the automation server and creates native widget
windows, but multiple unrelated WebViews remain in `Mounting`, fail liveness, get reloaded, and eventually hit
`too many times, giving up`.

A healthy backend or MCP endpoint is **not** evidence that the Svelte/React widgets are healthy.

```text
Rust/Tauri backend
    └── MCP/REST/CLI may be healthy

WebView UI plane
    └── can simultaneously be dead
```

Do not use `GET /ping`, MCP, `slu`, or native window-registry success as the WebView acceptance test.

## 1. Enforce one GUI instance, fail closed

The repository already implements a per-session named mutex in `src/background/utils/integrity/mod.rs`:

```text
Local\NAI-OS-Instance-{session_id}
```

and `main()` calls `acquire_instance_mutex()` before creating Tauri (`is_already_running()` is kept as a thin projection
of its outcome).

Keep this design, but harden it.

### Required changes

- Change mutex creation failure from fail-open to fail-closed in production.
- Log session id, mutex name, current PID, whether this process is primary, and exact acquisition failure.
- If another GUI instance exists, send the existing instance an IPC activation request, initialize no widgets, and exit
  immediately.
- `slu-service.exe` is not a second GUI instance and remains separate.
- Add `slu runtime instance` returning GUI PID, service PID, session id, mutex ownership and HTTP port owner.

### Runtime acceptance

```powershell
Get-Process NAI-OS -ErrorAction SilentlyContinue
```

must show exactly one GUI process in the interactive session.

Do not build a multi-instance tolerant desktop shell. The shell is intentionally singleton per user session.

## 2. Stop mixing debug and release artifacts

Never copy `target\debug\seelen-ui.exe` into `target\x86_64-pc-windows-msvc\release\seelen-ui.exe` or an equivalent
cross-profile path.

That creates an invalid hybrid where the binary's Tauri mode can disagree with the assets beside it.

### Two legal run modes only

#### Development

Use the repository's Tauri development lane:

```powershell
npm run dev
```

Requirements:

- dev UI server is running on the configured `devUrl`, currently `http://localhost:3579`,
- WebViews load from the dev server,
- source maps/devtools may be enabled.

#### Production/release

Use:

```powershell
npx tauri build --ci --verbose --no-bundle --target x86_64-pc-windows-msvc
```

Requirements:

- `beforeBuildCommand` runs `npm run build:ui -- --production`,
- `frontendDist` is fresh,
- Rust binary is built in production/custom-protocol mode,
- no dev server is required,
- artifact and resource profile are consistent.

### Forbidden

- copying debug executable into release output,
- copying release executable into debug output,
- mixing old `static` resources and new `dist`,
- launching debug Tauri without its dev server,
- treating resource checksum success as proof the frontend bundle is current.

## 3. Add an explicit boot pipeline flight recorder

`Mounting` represents too many possible failure states. Add boot stages with timestamps.

Backend:

```text
process.started
single_instance.acquired
integrity.complete
resources.complete
state.complete
widget.reconcile.complete
widget.native_window.created
widget.mounting
```

Frontend bootstrap:

```text
bootstrap.module.loaded
bootstrap.liveness.listener.registered
bootstrap.widget_definition.request.start
bootstrap.widget_definition.request.done
bootstrap.widget_definition.resolved
bootstrap.local_storage.hooked
bootstrap.widget_entry.fetch.start
bootstrap.widget_entry.fetch.done
bootstrap.widget_entry.injected
widget.module.loaded
widget.init.start
widget.init.done
widget.ready.start
widget.ready.done
```

Every record includes widget id, raw/decoded label, PID, WebView HWND, URL, build profile, monotonic timestamp and
duration from prior stage.

Add:

```text
slu widget boot
slu widget boot @seelen/weg
slu widget boot @seelen/fancy-toolbar
```

Output the last pipeline and first missing/failing stage.

## 4. Separate mount watchdog from runtime liveness

Current architecture starts runtime liveness while the widget is still `Mounting`. That conflates boot failure and
runtime hang.

Refactor to:

```text
Pending -> Creating -> Mounting -> Ready
                         |          |
                         |          -> Unresponsive -> Restarting -> Mounting
                         -> MountFailed
```

### Mount watchdog

Applies only to `Mounting` and proves the frontend reaches `Widget.ready()`.

### Runtime liveness

Start only after `SetCurrentWidgetStatus(Ready)`.

On restart:

- stop old liveness task,
- increment renderer generation,
- reload,
- set Mounting,
- wait for Ready,
- create one new liveness task.

There must never be two liveness loops for one pod generation.

## 5. Replace anonymous ping/pong with generation + nonce

Use:

```text
LivenessPing { generation, nonce, sentAt }
LivenessPong { generation, nonce, receivedAt }
```

Accept only matching generation and nonce. Record RTT.

Do not increase watchdog thresholds to hide a bug.

## 6. Make soft restart popup-safe

For `Popup` widgets such as Weg Preview, Tooltip, System Tray and context menus:

```text
native hide
-> reload
-> wait for Ready
-> remain hidden
-> show on next valid trigger
```

A dead renderer must never leave a black always-on-top rectangle.

## 7. Grouped preview ownership

Never call `Widget.self.hide()` inside `UserApplicationItem.svelte` to close Weg Preview. There `Widget.self` is the
main Weg.

Create one lifecycle bridge:

```text
weg-preview.opened
weg-preview.pointer-enter
weg-preview.pointer-leave
weg-preview.closed
```

Each trigger carries:

```text
previewSessionId
groupKey
windowSignature
```

Stale events are ignored. Hover close may hide only `@seelen/weg-preview`.

## 8. Audit collection locks before WebView operations

No collection lock may be held while calling WebView/Tauri/native operations.

Forbidden under global collection lock:

- WidgetWebview::create,
- reload/destroy,
- Tauri emit,
- Win32 calls,
- reconcile,
- pod.run,
- soft_restart,
- filesystem IO,
- arbitrary callback.

Use snapshot/Arc semantics:

```text
map guard -> clone Arc -> release guard -> perform operation
```

## 9. DevTools, not imaginary CDP 9222

Current browser args do not configure a remote debugging port. Do not assume 9222.

The repo already has Tauri `devtools` support and backend `debug_open_dev_tools(label)`.

Expose:

```text
slu widget list
slu widget devtools @seelen/weg
slu widget devtools @seelen/weg-preview
```

Optional CDP can be a dedicated diagnostic build later, with explicit port discovery.

## 10. Build provenance

Expose:

```json
{
  "gitSha": "...",
  "dirty": false,
  "profile": "release",
  "tauriMode": "custom-protocol",
  "frontendBundleHash": "...",
  "staticResourceHash": "...",
  "buildTimestamp": "...",
  "target": "x86_64-pc-windows-msvc"
}
```

CLI:

```text
slu runtime provenance
```

Never again debug an executable without knowing which frontend it contains.

## 11. Clean boot acceptance gate

```powershell
Get-Process seelen-ui -ErrorAction SilentlyContinue | Stop-Process -Force
Remove-Item .\dist -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item .\target\x86_64-pc-windows-msvc\release -Recurse -Force -ErrorAction SilentlyContinue
npx tauri build --ci --verbose --no-bundle --target x86_64-pc-windows-msvc
.\target\x86_64-pc-windows-msvc\release\seelen-ui.exe
```

Preserve user settings/data.

Within 30 seconds these enabled widgets must reach Ready:

```text
@seelen/weg
@seelen/fancy-toolbar
@seelen/flyouts
@seelen/window-manager
@seelen/apps-menu
```

For the next five minutes:

```text
0 new Liveness prove failed
0 unexpected Restarting
0 giving up
0 Mutex lock timed out
0 uncaught frontend boot errors
```

Then trigger lazy widgets: Weg Preview, System Tray, context menu, Task Switcher and Settings.

## 12. Required P0 report

Return:

- starting/ending SHA,
- clean/dirty status,
- dev/release commands,
- GUI instance count,
- provenance JSON,
- boot timeline for core widgets,
- Ready timestamps,
- liveness RTT p50/p95/p99,
- reload count,
- mount failure count,
- runtime liveness failure count,
- grouped preview click/hover result,
- installed/running evidence,
- no success claim based only on MCP/REST health.
