# Agent Runtime, Security, QoS and Compute

## Agent classes

```text
Interactive assistant
Background researcher
Browser agent
Developer agent
Mail agent
Media agent
Home agent
System maintenance agent
```

Each receives a capability lease.

## Capability leases

```yaml
agent: kelvin-clyne
allow:
  - window.*
  - repo.*
  - terminal.run:workspace
  - browser.read
deny:
  - mail.send
  - social.publish
```

Lease scopes:

```text
session
Activity
task
time-limited
persistent
```

## High-impact actions

Policy-gate:

```text
mail.send
social.publish
package.install
package.remove
file.delete
credential.use
external purchase
system setting change
```

Do not blanket-confirm every harmless action.

## Local secret broker

Secrets are referenced by logical id:

```text
secret:github
secret:youtube-api
secret:v271
```

Adapters request scoped use. Agents should not receive plaintext secret values unless absolutely necessary.

## BrowserOS neo isolation

Treat signed-in browser sessions as high-trust/high-impact.

Capabilities may restrict:

```text
allowed domains
read vs write
download destination
purchase
social posting
```

## QoS scheduler

Existing TaskQoS becomes part of `nai-qos`.

Profiles:

```text
hot-interactive
background-agent
media
low-latency-voice
batch
idle
```

Rules:

- VS Code/Insiders can remain hot.
- active BrowserOS can remain hot.
- Toastovač audio path is latency-critical.
- model inference gets an explicit compute class.
- background indexing may not steal interactive latency.

## Model residency

BeeLlama publishes:

```text
model loaded
VRAM/RAM footprint
context used
tokens/s
backend
device
```

Scheduler decides keep/prewarm/evict/move by workload.

No silent provider fallback.

## Intel NPU role

Use NPU for supported always-on low-power work:

```text
intent classification
embeddings
reranking
notification classification
lightweight semantic matching
```

Larger generative work goes to the appropriate GPU/CPU/local/cloud lane.

## Long-running agents

Healthy 20-hour workflows are legitimate.

Use heartbeat, checkpoint, progress, anomaly/stuck detection and self-healing where safe.

Do not kill healthy runs merely because wall-clock duration exceeds a generic timeout.

## Offline failover

For critical workflows preserve a local recovery path where feasible. Do not make recovery depend exclusively on a
phone, cloud backend or live token if an independent offline path can exist.
