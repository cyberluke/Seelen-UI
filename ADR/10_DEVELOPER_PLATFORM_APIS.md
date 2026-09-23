# NAI Developer Platform, APIs and SDK

## Goal

Third-party software should become NAI-native without forking the shell.

## SDK packages

```text
@nai/sdk
@nai/ui
@nai/icons
@nai/mcp
nai-sdk-rs
nai-sdk-dotnet
nai-sdk-python
```

## App manifest

```yaml
schema: nai.app/v1
id: com.velomail.app
name: Velo

identity:
  process:
    exe: velo.exe

capabilities:
  endpoint: pipe://nai/velo

events:
  - mail.received
  - mail.thread.updated

surfaces:
  command:
    - mail.search
    - mail.compose

permissions:
  requested:
    - network.mail
    - contacts.read
```

## Semantic object contracts

```json
{
  "id": "mail:thread:abc",
  "kind": "MailThread",
  "title": "...",
  "subtitle": "...",
  "capabilities": ["mail.reply", "mail.archive"],
  "relations": {
    "project": "XeOm"
  }
}
```

## Command Field provider API

Providers register actions and search sources.

No plugin may block the shell search loop. Use cancellation and time budgets.

## Shell surface API

Third parties may contribute:

```text
toolbar compact item
overview card
command result
context action
settings page
notification renderer
store detail renderer
```

All surfaces consume NAI design tokens.

No arbitrary full-screen DOM injection into the core shell.

## MCP gateway

Expose machine tools through one gateway:

```text
window.*
workspace.*
mail.*
browser.*
document.*
media.*
social.*
package.*
agent.*
```

The gateway authenticates, applies policy, logs provenance, routes to providers and normalizes errors.

## A2A / ACP / harness adapters

Do not hard-wire NAI Core to one agent protocol.

Adapters may bridge:

```text
MCP
A2A
ACP
VS Code agent harness
Kelvin-specific runtime
```

The core itself speaks typed capabilities.

## Event subscription

Trusted local clients use named-pipe subscription.

Web clients use authenticated loopback SSE/WebSocket.

Events are typed and versioned.

## CLI

```text
nai windows
nai focus XeOm
nai activity Development
nai mail search "Cassandra"
nai browser tabs
nai media pip
nai package search "ocr"
nai agent list
nai trace tail
```

Support `--json` everywhere practical.

## Developer mode

One shell toggle enables:

```text
object ids
capability inspector
event stream
latency overlays
widget boot state
WebView labels
agent provenance
```

Never force normal users to see developer noise.
