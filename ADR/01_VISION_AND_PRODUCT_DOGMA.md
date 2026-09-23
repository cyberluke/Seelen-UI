# NAI OS Vision and Product Dogma

## The thesis

Do not make "Windows with AI buttons".

Do not make a launcher with ChatGPT in the corner.

Do not make another Linux distribution just to own the boot logo.

Build a semantic desktop runtime over Windows 11 where the compatibility layer is boring and the interaction model is
new.

In NAI OS:

```text
apps are capability providers
windows are semantic objects
workspaces are persistent cognitive contexts
voice is a first-class input bus
agents are programmable operators
the taskbar is an object navigator
the browser is an agent substrate
the mail client is a commitment graph
the office suite is a document object runtime
media is searchable by meaning, not filenames
the app store installs capabilities, not just executables
```

## Inspiration, not imitation

### KDE Plasma / Wayland class experience

Take inspiration from the quality bar:

- configurable panels,
- overview,
- KRunner-like universal invocation,
- Activities / workspace semantics,
- predictable window rules,
- per-monitor behavior,
- strong keyboard navigation,
- effects that feel composited rather than HTML glued over Windows,
- Discover-style application management.

Do not copy KDE UI literally.

### SAP North Star AI Vision

Use a coherent enterprise-grade design system:

- lavender / indigo primary energy,
- clear hierarchy,
- high contrast,
- quiet information density,
- polished data surfaces,
- precise spacing,
- trustworthy state transitions.

Then deliberately push it into an OLED-first, high-dopamine consumer/workstation direction.

### Japanese luxury + German precision

The interface may be luminous, playful and saturated, but geometry and behavior must be deterministic.

Visual richness is allowed.

Behavioral ambiguity is not.

## Seven dogmas

1. **Semantic before visual automation.** If an object has an API, use the API. Vision + mouse is fallback.
2. **Persistent identity before process identity.** PID/HWND are runtime addresses, not user concepts.
3. **Spatial memory is sacred.** Focus must never randomly reorder the user's world.
4. **Local first, cloud optional.** Local intent routing, embeddings, reranking and shell control should keep working
   offline.
5. **Agent actions are observable.** Every agent action has source, target, reason, result and undo story.
6. **Interfaces are multimodal.** Voice, keyboard, touch, pen, mouse and agent API all drive the same capability model.
7. **The shell is a runtime, not a theme.** Fancy Toolbar, Weg, Task Switcher and widgets are views over one native
   state model.

## Product surfaces

```text
NAI Shell
NAI Overview
NAI Command Field
NAI Activities
NAI Store
NAI Media
NAI Social
NAI Mail
NAI Office
NAI Browser
NAI Agent Cockpit
NAI Workstation
NAI Developer
NAI Voice
NAI Memory
NAI Settings
```

Each surface consumes the same semantic graph.

## Desktop object model

Example:

```json
{
  "id": "window:vscode-insiders:xeom",
  "kind": "window",
  "application": "vscode-insiders",
  "workspace": "XeOm",
  "runtime": {
    "hwnd": "0x123456",
    "pid": 9999
  },
  "relationships": {
    "repo": "repo:XeOm",
    "terminals": ["terminal:xeom:1"],
    "browserContexts": ["browser:edge:xeom"],
    "memoryNamespace": "qdrant:XeOm"
  },
  "capabilities": [
    "window.focus",
    "window.maximize",
    "repo.open",
    "agent.attach"
  ]
}
```

This is the conceptual DOM of the desktop.

## The ultimate interaction

The target experience is not:

> Open VS Code, find the tab, open Edge, search mail, open terminal.

It is:

> Jarvisi, vrať mě do XeOm a dej vedle toho poslední Valhalla diagnostiku.

The runtime resolves workspace capsule, editor, terminals, browser research tabs, mail context, layout, monitor, model
state and QoS state.

The desktop reorganizes itself around intent.
