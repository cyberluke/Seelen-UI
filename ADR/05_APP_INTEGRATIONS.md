# First-Class Application Integrations

## Integration rule

Do not automate first-class apps by clicking pixels if the app can expose a typed adapter.

Each fork should publish:

```text
identity provider
capability provider
event provider
context provider
deep links
health
version
permissions
```

# 1. Velo Mail Client

Current Velo is a Tauri v2 + React/Rust local-first mail client with Gmail API, IMAP/SMTP, SQLite/FTS and built-in AI
functions. Fork it into `NAI Mail` or keep Velo branding internally.

## Native NAI bridge

Expose a local named-pipe API from the Rust backend.

Capabilities:

```text
mail.accounts.list
mail.search
mail.thread.get
mail.thread.summarize
mail.thread.commitments
mail.draft.create
mail.draft.update
mail.send
mail.archive
mail.snooze
mail.labels
mail.attachments.get
mail.contact.lookup
mail.calendar.related
```

Events:

```text
mail.received
mail.thread.updated
mail.draft.changed
mail.sent
mail.followup.due
```

## Commitment graph

Extract durable objects with provenance:

```text
person
request
promise
deadline
amount
attachment
project
calendar date
reply status
```

Fancy Toolbar shows only meaningful mail state, not giant unread noise.

Jarvis examples:

> Co chce Cassandra? Odpověz, že zítra pošlu build. Připomeň mi ten thread, až otevřu VIVERRU.

# 2. Toastovač / Jarvis

Toastovač is the voice kernel, not a separate chatbot application.

Inputs:

```text
HA Voice PE microphone
Windows default microphone
USB headset
optional application-audio reference
```

Outputs:

```text
speech
shell actions
agent tasks
media control
smart-home actions
```

## Voice Context Resolver

Resolve:

```text
tohle
tenhle mail
tady
to video
ten projekt
```

against focused object, pointer target, current media, active capsule and recent command graph.

Use local NPU intent/entity pre-routing before escalating to a larger model.

# 3. BrowserOS

BrowserOS is the human-operated AI browser.

Expose objects/capabilities:

```text
browser.windows
browser.tabs
browser.active_tab
browser.bookmarks
browser.history
browser.open
browser.search
browser.ask_page
browser.send_to_agent_browser
```

Bind tabs/windows into Context Capsules.

# 4. BrowserOS neo

BrowserOS neo is the dedicated browser for agents.

Treat its local MCP/API as an agent execution provider.

Objects:

```text
neo.session
neo.task
neo.tab
neo.replay
```

Capabilities:

```text
browser_agent.run
browser_agent.status
browser_agent.pause
browser_agent.cancel
browser_agent.replay.open
```

Toastovač:

> Dej to do nea a zjisti ceny.

The user's human browser remains untouched.

# 5. LibreOffice

LibreOffice 26.8 provides UNO with Python, Java and C++ bindings and external process control via pipe/socket.

Build `nai-libreoffice-bridge` using published UNO APIs.

## Writer

```text
document.open
document.save
document.export
writer.selection.get
writer.selection.replace
writer.heading.list
writer.section.get
writer.comment.add
writer.track_changes.toggle
```

## Calc

```text
calc.sheets.list
calc.range.read
calc.range.write
calc.formula.set
calc.chart.create
calc.filter.apply
```

## Impress

```text
impress.slides.list
impress.slide.create
impress.element.update
impress.export.pdf
```

Document graph objects:

```text
Document
Section
Table
Sheet
Range
Slide
Comment
Reference
```

Jarvis:

> V tomhle dokumentu přepiš druhou kapitolu, tabulku nech být.

# 6. Cross-app workflows

## Mail -> Office

```text
Velo thread
-> extract requirements
-> create Writer document through UNO
-> graph artifact
-> optional Velo draft reply
```

## Browser -> V271

```text
active BrowserOS tab
-> structured/sanitized page context
-> V271 conversation asset
-> citation/provenance link
```

## Office -> Mastodon

```text
Writer document
-> summary
-> social draft
-> preview
-> explicit publish
```

## Neo -> Activity

```text
BrowserOS neo session
-> extracted sources + replay
-> Activity graph
-> browser research context
-> optional V271 thread
```
