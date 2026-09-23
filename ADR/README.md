# NAI OS 2027 — Mega Specification

**Project:** NANOTRIK.AI OS / NAI OS\
**Substrate:** Windows 11, not a Linux distribution\
**Primary shell fork:** NAI OS, already renamed from the Seelen UI fork; inspect current branch and SHA locally **Design
direction:** SAP North Star AI Vision × KDE Plasma/Wayland-grade composability × OLED candy-glow luxury UI ×
deterministic agentic desktop runtime\
**Core principle:** Windows becomes the compatibility/hardware substrate. NAI OS becomes the semantic desktop, agent
runtime, identity layer, application fabric, media fabric, package/store fabric, voice layer, and user experience.

This is a binding, full-scope implementation package for the already-renamed NAI OS fork and every first-party fork
under D:\_SATIN, plus Toastovač at D:\_SATIN_AI\Toastovac\jarvis\. All requirements are mandatory and must be
implemented in depth. Read 14_FULL_SCOPE_BINDING_ADDENDUM.md and 15_DELIVERY_ACCEPTANCE_MATRIX.md before any other
section; they resolve conflicts in earlier wording.

## Read in this order

0. `14_FULL_SCOPE_BINDING_ADDENDUM.md` and `15_DELIVERY_ACCEPTANCE_MATRIX.md` — binding full scope and proof; these
   supersede narrower phrasing below.

1. `00_P0_RECOVERY_BOOT_LIVENESS.md` — stop the current Seelen/WebView crash loop before doing anything else.
2. `01_VISION_AND_PRODUCT_DOGMA.md` — what NAI OS is and what it refuses to become.
3. `02_CORE_ARCHITECTURE.md` — NAI Core, semantic desktop graph, capability bus, identity, event fabric.
4. `03_TOP_10_PLATFORM_FEATURES.md` — the ten biggest product bets.
5. `04_50_DESIGN_FEATURES.md` — fifty concrete UX/design features.
6. `05_APP_INTEGRATIONS.md` — Velo, Toastovač/Jarvis, BrowserOS, BrowserOS neo, LibreOffice.
7. `06_YOUTUBE_SHORTS_VISUAL_ENGINE.md` — Shorts-first YouTube surface, PiP, semantic visual discovery.
8. `07_NAI_APP_STORE.md` — unified AI app/store/package ecosystem over WinGet, UniGetUI, Chocolatey, Scoop, VSIX, MCP,
   skills and models.
9. `08_V271_MASTODON_SOCIAL_FABRIC.md` — v271.cz agentic chat + club.v271.cz social layer.
10. `09_AGENT_RUNTIME_SECURITY_QOS.md` — agent permissions, execution, QoS, model routing, local-first guarantees.
11. `10_DEVELOPER_PLATFORM_APIS.md` — SDK, MCP, A2A/ACP adapters, app manifests, shell surfaces.
12. `11_ROADMAP_AND_ACCEPTANCE.md` — phased implementation plan and hard acceptance gates.
13. `12_RESEARCH_NOTES_AND_SOURCES.md` — historical external references, revalidate during implementation.
14. `13_CODING_AGENT_MASTER_INSTRUCTIONS.md` — binding execution instructions.
15. `14_FULL_SCOPE_BINDING_ADDENDUM.md` — expanded mandatory contracts.
16. `15_DELIVERY_ACCEPTANCE_MATRIX.md` — completion evidence.
17. `16_CURRENT_SOURCE_ANCHORS.md` — current-source anchors.

## Non-negotiables

- Do not use computer-use vision/mouse for actions that have a native semantic API.
- Do not let UI state become the authoritative desktop state.
- Do not let one widget crash take down the shell.
- Do not silently fall back between AI providers.
- Do not hide failures behind longer watchdog timeouts.
- Do not use `NEXT_PUBLIC_*` for secrets or sensitive runtime selection.
- Do not force every feature through Seelen's generic plugin sandbox when a first-class native component is warranted.
- Do not make the user relearn window order every time focus changes.
- Do not turn the desktop into a giant chat window.
- Do not build an "AI skin". Build an **AI-native desktop object model**.

## Product sentence

> **NAI OS is a semantic, voice-first, agent-programmable desktop runtime that keeps Windows compatibility while
> replacing the user's mental model of apps, windows, files and services with persistent identities, live capabilities
> and spatial workspaces.**

## Reference topology

```text
Windows 11
│
├── Drivers / Win32 / DirectX / CUDA / WebView2 / WinUI / audio / input
│
└── NAI OS
    ├── NAI Core daemon
    │   ├── Semantic Desktop Graph
    │   ├── Capability Registry
    │   ├── Event Fabric
    │   ├── Agent Runtime
    │   ├── Policy + Permission Engine
    │   ├── QoS / Residency Scheduler
    │   ├── Memory / Qdrant adapters
    │   └── Package / Store broker
    │
    ├── NAI Shell
    │   ├── Seelen UI fork
    │   ├── SeelenWeg
    │   ├── Fancy Toolbar
    │   ├── Task Switcher
    │   ├── Activities / Spaces
    │   ├── Overview / Search
    │   └── notification + media surfaces
    │
    ├── Voice: Toastovač / Jarvis
    ├── Workstation: Kelvin Workstation AI
    ├── Developer Agent: Kelvin Clyne VSIX / harness adapters
    ├── Browser Fabric: BrowserOS + BrowserOS neo
    ├── Mail: Velo fork
    ├── Office: LibreOffice + UNO bridge
    ├── Media: YouTube / Shorts / PiP / semantic visual discovery
    ├── Social: club.v271.cz Mastodon
    └── Agentic cloud/chat: v271.cz
```
