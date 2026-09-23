# YouTube / Shorts / PiP Visual Media Engine

## Goal

YouTube should feel like a native media substrate inside NAI OS, not merely a website.

Primary experiences:

```text
voice-driven Shorts
vertical media queue
PiP
semantic creator/topic discovery
current-video context
local taste model
activity-aware media
```

Example:

> Jarvisi, teď chci korejské roztleskávačky, Fubon Angels a podobný vibe.

## API reality

The official YouTube Data API does not expose a first-class Shorts resource/filter.

`search.list` currently supports `videoDuration=short`, meaning videos shorter than four minutes. Do not equate that
with Shorts.

Use multiple signals:

```text
duration
vertical geometry when lawfully observable
known Shorts URLs encountered in browser
channel/playlist context
candidate visual classification
user feedback
```

## Candidate pipeline

```text
1. text/entity query
2. official YouTube search/channel candidates
3. metadata enrichment
4. thumbnail visual embeddings
5. OCR / logos / uniforms / event context
6. optional ephemeral in-player visual inspection
7. local reranker
8. queue
```

## No general biometric identity engine

Do not build a general face-identification database.

For named creators/performers:

- use channel/entity provenance to establish candidate scope,
- use visual-semantic reranking for non-biometric attributes such as team uniform, stage, logo, colors, event context,
  outfit and scene,
- treat a visual match as content similarity, not proof of identity.

This still solves the practical problem where the person is absent from title/description/comments.

## Visual index

Store permitted derived metadata, not pirated media.

```json
{
  "videoId": "...",
  "channelId": "...",
  "durationMs": 43000,
  "thumbnailEmbedding": "...",
  "ocr": ["Fubon", "Angels"],
  "visualTags": ["cheerleading", "stadium", "blue uniform", "vertical"],
  "tasteScore": 0.86
}
```

## BrowserOS visual inspection

For candidates requiring richer understanding:

```text
BrowserOS / neo
-> open official YouTube surface
-> play candidate visibly
-> transiently inspect rendered frame
-> VLM tags scene/context
-> discard transient frame unless user lawfully saves a screenshot
```

Do not build a downloader as the discovery backend.

## Shorts Surface

```text
┌───────────────────────────────────┐
│ creator / reason / queue          │
│                                   │
│          vertical player          │
│                                   │
│   ♡   queue   PiP   more-like     │
└───────────────────────────────────┘
```

Voice:

```text
další
víc jako tohle
ne tenhle tým
dej to do PiP
ulož ten kanál
najdi další videa z toho stadionu
```

## PiP as a shell object

State:

```text
media source
video id
playback position
channel/creator
queue
Activity
geometry
monitor
always-on-top
```

Magnet zones:

```text
top-left
top-right
bottom-left
bottom-right
vertical rail
```

PiP can follow an Activity or remain global.

## Local taste model

Signals:

```text
watch completion
skip latency
replay
manual favorite
voice more-like-this
voice less-like-this
channel follow
visual cluster
time of day
Activity
audio on/off
```

Local taste is an NAI layer and must not silently mutate the user's YouTube account behavior.

## Search by vibe

> Podobné tomu předchozímu, ale stadion, ne studio.

Translate into positive/negative semantic visual tags and rerank candidates locally.

## Media semantic context

Graph node:

```text
media:youtube:<videoId>
```

Edges:

```text
played_in Activity
related_to Creator
queued_after MediaItem
bookmarked_to Collection
mentioned_in Conversation
```

Then "Pošli tohle do club.v271.cz" has an unambiguous referent.

## Compliance gate

Before shipping:

- verify current YouTube API Terms and Developer Policies,
- obey embedded-player requirements,
- preserve required attribution/controls,
- do not circumvent restrictions,
- do not mass-download media,
- do not present a deceptive YouTube clone.

NAI's value is semantic orchestration around YouTube.
