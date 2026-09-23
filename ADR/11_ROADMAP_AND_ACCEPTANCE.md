# Binding roadmap notice

All original phases below are required. The expanded full delivery and acceptance criteria are in
\`14_FULL_SCOPE_BINDING_ADDENDUM.md\` and \`15_DELIVERY_ACCEPTANCE_MATRIX.md\`. A phase is a dependency gate, never a
scope reduction. First-party fork renaming, NANOTRIK.AI SSO, launcher, store, Candy Sugar, Jarvis overlay, AI telemetry
and video gateway are mandatory.

# NAI OS Roadmap and Acceptance Gates

## Phase 0 — Stabilize Seelen fork

Deliver:

- one-instance enforcement,
- canonical dev/release lanes,
- WebView boot telemetry,
- separate mount/runtime watchdogs,
- grouped preview lifecycle,
- stable Fancy Toolbar tray,
- stable Task Switcher,
- no shell-wide liveness cascade.

Exit criteria:

```text
5 minutes idle + active
0 core widget liveness failures
0 random dock disappearance
0 black zombie windows
0 duplicate GUI instance
```

## Phase 1 — Semantic shell kernel

Deliver NAI Core daemon, semantic graph, unified identity/order, Activities, Context Capsules, named-pipe API and MCP
gateway.

## Phase 2 — Design system

Deliver `@nai/icons`, OLED palette, Candy Sugar glyphs, Overview, Command Field, agent presence, notification capsules
and stable Task Switcher.

## Phase 3 — Voice

Deliver Toastovač adapter, context resolver, focused-object references, shell/media control, Activity switching and
local NPU intent lane.

## Phase 4 — Browser Fabric

Deliver BrowserOS human adapter, BrowserOS neo agent adapter, session/replay objects, capsule tab association and Agent
Cockpit integration.

## Phase 5 — Mail + Office

Deliver Velo fork adapter, commitment graph, LibreOffice UNO bridge, semantic cross-app drag/drop and document context
actions.

## Phase 6 — Media

Deliver YouTube candidate search, vertical surface, PiP, local taste model, visual-semantic reranking and Jarvis media
commands.

## Phase 7 — Store

Deliver package aggregator, NAI manifests, capability registry, permissions, install transactions and models/skills/MCP
catalog.

## Phase 8 — Social + V271

Deliver V271 context actions, conversation/artifact nodes, Mastodon adapter and social capsule.

## Phase 9 — Developer ecosystem

Deliver SDK, manifests, docs, examples and stable protocol versioning.

# Performance budgets

```text
native action core dispatch p50 < 1 ms
native action core dispatch p95 < 5 ms
cached preview first paint target < 16 ms
local command result first paint target < 50 ms
local voice acknowledgment target < 150 ms
```

Do not fake speed by acknowledging actions not accepted by the target.

# Reliability budgets

One broken provider must not crash NAI Core, Weg, Fancy Toolbar, Task Switcher or unrelated providers.

Every provider uses health state, bounded restart/backoff and visible degraded mode. No infinite silent crash loops.

# Release train

```text
canary
dev
beta
stable
```

Use a fork-specific package identity. Keep stock Seelen as recovery until the NAI fork is stable enough to replace it.

# Required megalomaniac demo

1. Say: `Jarvisi, vrať mě do XeOm.`
2. XeOm Activity restores editor, browser context and terminal.
3. Ask: `Najdi v mailu, co potřebuje Libor.`
4. Velo returns structured commitments.
5. Say: `Otevři poslední Valhalla research.`
6. BrowserOS context focuses.
7. Say: `Dej do nea úkol porovnat tři zdroje.`
8. BrowserOS neo task appears in Agent Cockpit.
9. Say: `Pusť mi do PiP Fubon Angels Shorts.`
10. Media engine opens vertical queue + PiP.
11. Say: `Pošli aktuální research do V271.`
12. V271 receives structured context.
13. Say: `Udělej z toho krátký post na club.`
14. Mastodon draft appears for approval.
15. All windows retain persistent spatial identity throughout.

That is NAI OS.

Not a chatbot. Not a skin. A programmable desktop runtime.
