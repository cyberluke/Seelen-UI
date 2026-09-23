# Research Notes and External Sources

Research date: 2026-09-19.

This file separates current external facts from proposed NAI OS design.

## Seelen UI fork

Repository:

- https://github.com/cyberluke/Seelen-UI
- branch: `kelvin-compact-weg`
- known pushed SHA during planning: `03b939eaefc21f34287ba2f6c8e2f887134a3c80`

Observed architecture:

- Tauri v2,
- Svelte/React widgets,
- Win32 shell integration,
- per-session named single-instance mutex,
- widget liveness watchdog,
- internal WebView2,
- SeelenWeg / Fancy Toolbar / Task Switcher,
- Tauri devtools feature enabled.

## BrowserOS / BrowserOS neo

Repository:

- https://github.com/browseros-ai/BrowserOS

Current project description:

- BrowserOS neo is positioned as a secondary browser dedicated to AI agents.
- It runs locally and exposes MCP/API infrastructure.
- The monorepo includes a Rust neo backend, browser control primitives, MCP packages, dashboard/replay surfaces and
  harness integrations.
- BrowserOS is the human-driven AI browser in the same monorepo.

Architectural inspiration:

- dedicated agent browser,
- persistent signed-in state,
- agent sessions,
- audit/replay,
- local MCP endpoint,
- explicit harness integrations.

## Velo

Repository:

- https://github.com/avihaymenahem/velo

Current architecture/features:

- Tauri v2 + React + Rust,
- local-first SQLite,
- Gmail API and IMAP/SMTP,
- threaded mail,
- full-text search,
- built-in AI provider support,
- calendar,
- system tray,
- Apache-2.0.

Strong fork candidate for NAI Mail.

## LibreOffice

Official API:

- https://api.libreoffice.org/

Current 26.8 SDK documents UNO as the core Office API with Python, Java and C++ bindings.

Python help:

- https://help.libreoffice.org/latest/en-GB/text/sbasic/python/

External Python can connect to a running LibreOffice process through a pipe/socket.

Use published UNO APIs for semantic document automation.

## KDE Plasma

Reference:

- https://kde.org/plasma-desktop/

Relevant inspiration:

- configurable desktop,
- KRunner extensible launcher,
- Discover app management,
- flexible panels,
- composable shell philosophy.

Borrow principles, not visual design.

## UniGetUI

Current project:

- https://github.com/Devolutions/UniGetUI

Current 2026 releases continue multi-package-manager workflows and richer operation/maintenance behavior.

NAI Store extends the category to AI capabilities, models, MCP, skills and semantic permissions.

## Mastodon

Official API:

- https://docs.joinmastodon.org/client/intro/
- https://docs.joinmastodon.org/methods/streaming/
- https://docs.joinmastodon.org/methods/statuses/

Mastodon exposes REST plus streaming methods suitable for a first-class social adapter.

## YouTube

Official Data API search:

- https://developers.google.com/youtube/v3/docs/search/list

Current behavior:

- `videoDuration=short` means less than four minutes.
- It is not a dedicated Shorts identity/filter.

Policies:

- https://developers.google.com/youtube/terms/developer-policies
- https://developers.google.com/youtube/terms/api-services-terms-of-service

NAI Media must preserve API/player requirements and add independent semantic value rather than replicate YouTube.

## Visual people-search caveat

The useful product need is to find relevant Shorts even when a named creator/person is absent from
title/description/comments.

Do not implement a general biometric face-identification database.

Use creator/channel provenance, event/team associations, OCR, logos, uniforms, scene/stage context, visual-semantic
similarity and transient VLM inspection.
