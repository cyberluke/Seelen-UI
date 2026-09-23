# v271.cz + club.v271.cz Social Fabric

## Goal

NAI OS should not treat v271.cz and club.v271.cz as websites.

```text
v271.cz = agentic conversation / research / workflows
club.v271.cz = social/community/event stream over Mastodon
```

# v271.cz integration

Objects:

```text
V271Conversation
V271Artifact
V271Workflow
V271AgentRun
V271Citation
V271Dataset
```

Capabilities:

```text
v271.chat.create
v271.chat.send
v271.context.attach
v271.artifact.open
v271.workflow.run
v271.run.status
v271.share
```

Command Field:

```text
ask v271 ...
send current page to v271
attach current document to v271
open last research artifact
```

Context actions:

```text
Send to V271
Ask V271 about this
Create V271 workflow from selection
```

Toastovač example:

> Jarvisi, pošli tuhle tabulku do V271 a zeptej se na anomálie.

The current Calc range becomes a typed context object. Do not use a screenshot when structured cells are available.

# Mastodon / club.v271.cz

Mastodon provides REST APIs plus streaming interfaces. Build a native social adapter.

Objects:

```text
MastodonAccount
MastodonStatus
MastodonThread
MastodonNotification
MastodonMedia
MastodonTag
```

Capabilities:

```text
social.timeline.home
social.timeline.local
social.notifications
social.search
social.status.compose
social.status.reply
social.status.boost
social.status.favorite
social.media.upload
social.thread.get
```

## Streaming integration

Map live social events onto NAI Event Fabric:

```text
new status
notification
conversation update
```

## Fancy Toolbar

Show only meaningful states:

```text
direct mention
reply
priority account
current-project tag
```

Do not add an anxiety-inducing giant counter.

## Social Capsule

Compact shell surface:

```text
Club feed
Mentions
Project tags
Following
Compose
```

## Cross-surface publishing

> Postni tenhle screenshot na club a přidej krátký technický popis.

Flow:

```text
current object
-> media/export
-> draft
-> preview
-> explicit send
```

> Sdílej výsledek tohoto V271 workflow do clubu.

The V271 artifact retains provenance in post metadata/link.

## Moderation and safety

- preserve Mastodon content warnings,
- preserve visibility settings,
- expose mute/block/report,
- do not auto-post publicly without explicit configured authorization policy,
- show target account and visibility before high-impact publish unless pre-authorized.
