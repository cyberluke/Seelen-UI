# Top 10 New NAI OS Platform Features

## 1. Semantic Desktop Graph

Build a machine-readable graph of the live desktop so agents stop guessing and voice commands become precise.

Example:

> Open the mail thread related to XeOm and put it next to the project.

The graph resolves project, thread, current window identities and layout.

## 2. NAI Activities + Context Capsules

Reinvent virtual desktops as persistent cognitive environments containing windows, browser contexts, mail filters,
repos, models, notifications, layout, QoS, audio and agent permissions.

## 3. Toastovač Ambient Voice Kernel

Toastovač/Jarvis is not an app. It is the multimodal intent ingress of the OS.

Capabilities:

- continuous voice,
- interruption,
- context-aware references,
- shell/media control,
- HA Voice PE support,
- local NPU intent pre-routing.

Example:

> Tohle video dej do PiP a vrať mě do TaskQoS.

## 4. Agent Cockpit + Live Presence

Every running agent gets visible presence in Fancy Toolbar/Overview.

Show:

```text
Kelvin: editing repo
BrowserOS neo: researching
Mail agent: drafting reply
Media agent: building Shorts queue
```

Inspect current step, targets, permissions, resource use, pause/cancel/replay.

## 5. Browser Fabric

Two explicit roles:

```text
BrowserOS = human browsing + AI copilot
BrowserOS neo = dedicated logged-in browser for agents
```

NAI adds tab identity, project association, task identity, replay links and voice handoff.

## 6. AI-Native Mail and Commitment Graph

Fork Velo and turn mail into people, commitments, deadlines, attachments, projects, invoices, promises and follow-ups.

> Co po mně chce Cassandra?

returns structured commitments rather than a pile of messages.

## 7. Visual Media Intelligence + Shorts Lens

Create a YouTube-first vertical media surface with PiP, voice queue, semantic visual reranking, OCR/logo/team context,
local taste model and current-video context in Jarvis.

## 8. NAI Store: Apps + Agents + Skills + Models

One store searches WinGet, UniGetUI-backed managers, Chocolatey, Scoop, VSIX, MCP servers, skills, models, themes and
NAI adapters.

The unit is a capability package, not merely an installer.

## 9. Document Object Runtime over LibreOffice

Use UNO to expose semantic Writer/Calc/Impress objects.

> V tomhle dokumentu přepiš druhou kapitolu, tabulku nech být.

No ribbon clicking if structured APIs exist.

## 10. Predictive QoS + Model Residency Scheduler

Merge TaskQoS with model residency and Activity context.

Keep latency-critical apps hot, keep embeddings/rerank on NPU, load/evict BeeLlama models by need, prewarm likely next
contexts, and never terminate healthy long-running agents merely because of arbitrary wall-clock timeouts.
