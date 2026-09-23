# NAI Core Architecture

## 1. NAI Core daemon

Create one native Rust service/daemon responsible for persistent semantic state and machine-local orchestration.

Do not put authoritative state inside Seelen Svelte stores.

Suggested modules:

```text
nai-core/
├── identity/
├── graph/
├── capability/
├── event/
├── workspace/
├── app_adapters/
├── package/
├── agent/
├── policy/
├── qos/
├── model_runtime/
├── media/
├── search/
├── storage/
├── telemetry/
└── ipc/
```

## 2. Internal transport

Primary trusted transport:

- Windows named pipe,
- user-session scoped ACL,
- typed/versioned messages.

Optional adapters:

- local REST for development,
- MCP for LLM/tool clients,
- WebSocket/SSE for live views,
- A2A / ACP / harness adapters where useful.

Do not make MCP the internal kernel protocol. MCP is an adapter.

## 3. Semantic Desktop Graph

Nodes:

```text
Application
Window
Workspace
Activity
Document
File
Repository
Terminal
BrowserWindow
BrowserTab
MailThread
Contact
CalendarEvent
MediaItem
Creator
Device
HomeEntity
Model
Agent
Conversation
MastodonPost
Package
Capability
Notification
ClipboardItem
```

Edges:

```text
belongs_to
opened_by
associated_with
mentions
created_from
last_used_with
visible_on
focused_after
depends_on
controlled_by
related_to
pinned_to
installed_by
generated_by
```

Use a hybrid:

- durable relational identity/state,
- Qdrant for semantic vectors,
- in-memory indexes for hot runtime traversal,
- append-only flight recorder for shell/agent actions.

Do not force graph semantics into Qdrant.

## 4. Stable identity

Runtime ids:

```text
HWND
PID
WebView label
browser tab id
UNO object handle
```

Logical ids:

```text
vscode-insiders:workspace:XeOm
edge-stable:profile:Default:window:1
mail:thread:<provider-id>
document:<content-hash-or-provider-id>
mastodon:status:<server>:<id>
```

Logical identity owns persistence. Runtime identity is rebound.

## 5. Capability Registry

Every adapter publishes capabilities.

```yaml
provider: velo
capabilities:
  - mail.search
  - mail.thread.get
  - mail.reply.draft
  - mail.send
  - contact.lookup

provider: libreoffice
capabilities:
  - document.open
  - document.export.pdf
  - writer.selection.get
  - writer.selection.replace
  - calc.range.read
  - calc.range.write

provider: browseros-neo
capabilities:
  - browser.task.run
  - browser.tabs.list
  - browser.page.read
  - browser.page.act

provider: seelen
capabilities:
  - window.list
  - window.focus
  - window.focus_maximize
  - workspace.activate
  - tray.activate
```

## 6. Capability descriptors

```yaml
id: mail.send
risk: external_side_effect
interactive_confirmation: contextual
offline: false
undo: limited
input_schema: ...
output_schema: ...
latency_class: interactive
provider_priority: [velo]
```

The router selects by capability, not brand.

## 7. Event Fabric

```text
window.created
window.destroyed
window.focused
window.title_changed
workspace.activated
mail.received
mail.thread.changed
calendar.starting
browser.tab.changed
browser.task.started
browser.task.finished
media.changed
youtube.player.state
mastodon.notification
document.changed
clipboard.changed
model.loaded
model.evicted
agent.action.started
agent.action.completed
qos.changed
package.updated
```

Use native events. Do not poll state already available as events.

## 8. Context Capsules

```yaml
id: capsule:xeom
name: XeOm
activity: development

objects:
  - vscode-insiders:workspace:XeOm
  - terminal:XeOm
  - browser-context:XeOm
  - qdrant:XeOm
  - repo:XeOm

layout:
  monitor_roles:
    primary: editor
    secondary: browser

runtime:
  model_profile: coding
  qos_profile: hot
```

Activate by CLI, command field or voice.

## 9. Universal command surface

Voice, Win+Space, Kelvin Workstation, Kelvin Clyne, MCP, keyboard shortcuts and scripts all call the same capability
router.

No duplicate business logic.

## 10. Activities

Go beyond virtual desktops.

Activity includes:

- windows,
- capsules,
- notifications,
- media policy,
- QoS,
- model profile,
- browser context,
- allowed agents,
- shell accent.

Examples:

```text
Development
Research
Operations
Media
Communication
Presentation
Deep Focus
```

## 11. Undo and transactions

Multi-step desktop actions record pre-state and an undo token.

```text
nai undo last
```

should reverse reversible shell state changes.

## 12. Flight recorder

Every action records source, actor, capability, target, result, latency and undo metadata.

This powers replay, trust and debugging.
