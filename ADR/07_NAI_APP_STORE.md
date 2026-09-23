# Full catalog and install contract

The original concepts below are mandatory; the JSON schema, 31 catalog candidates, install/update/uninstall broker and
first-party NAI suite are specified in \`14_FULL_SCOPE_BINDING_ADDENDUM.md\`. The **AI Apps** item in the launcher left
navigation opens this store.

# NAI Store — Unified Apps, Agents, Skills, Models and Capabilities

## Thesis

A 2027 AI desktop should not have separate discovery UX for:

```text
Windows apps
CLI tools
VS Code extensions
MCP servers
agent skills
local models
themes
NAI adapters
```

NAI Store is the meta-catalog. It does not replace every package manager. It normalizes them.

## Upstream executors

Support adapters for:

```text
WinGet
UniGetUI
Chocolatey
Scoop
PowerShell Gallery
npm
pip/pipx
Cargo
.NET tools
VSIX
browser extensions
local model registries
NAI native packages
```

Use direct APIs where mature. Treat UniGetUI as architectural inspiration/optional backend for multi-manager discovery
and operation tracking, not as an opaque shell-out dependency.

## NAI package manifest

```yaml
schema: nai.app/v1

id: com.example.superocr
name: Super OCR
version: 1.4.2
kind: ai-capability

sources:
  - manager: winget
    id: Example.SuperOCR

capabilities:
  - document.ocr
  - image.text.extract

permissions:
  - filesystem.read:user-selected
  - gpu.compute

integrations:
  mcp:
    server: optional
  nai:
    adapter: adapters/superocr

health:
  command: superocr --health

rollback:
  supported: true

ui:
  surfaces:
    - store
    - command-field

models:
  - id: example/ocr-small
    optional: true
```

## Package classes

```text
Application
CLI
Driver
ShellExtension
NAIAdapter
MCPServer
AgentSkill
Model
Theme
IconPack
Workflow
BrowserExtension
VSIX
```

## Capability-first discovery

User searches:

> PDF OCR

Results group by outcome:

```text
Install app
Install local capability
Install MCP server
Install model
Use cloud capability
```

not package-manager brand.

## Trust model

Every result shows:

```text
publisher
source
signature
license
permissions
network access
filesystem access
agent-callable capabilities
update channel
last update
NAI compatibility
```

## Install transaction

```text
resolve
-> verify source
-> show permissions
-> create restore point
-> install dependency chain
-> install adapter
-> register capabilities
-> health check
-> publish to shell
```

Failure rolls back where the underlying package manager allows it.

## Update policy

Per package:

```text
auto
notify
manual
pinned
security-only
```

Activities may suppress updates during presentation/deep focus.

## Store sections

```text
For Your Work
Local AI
Agents
Developer Tools
Media
Office
Communication
Shell
Models
Experimental
```

## NAI compatibility badge

```text
L0 Installed
L1 Deep link
L2 Events
L3 Capabilities
L4 Semantic objects
L5 Full NAI-native integration
```

## Agent-safe install

By default an AI may discover, compare and prepare an install plan. Capability/permission-changing installation requires
approval unless organization/user policy explicitly pre-authorizes the source and capability class.
