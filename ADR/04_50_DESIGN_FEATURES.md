# 50 New NAI OS Design Features

These are product features, not random visual effects. Every item must be tokenized, configurable and
performance-budgeted.

## 1. Candy Sugar Glyphs

2.5D SVG icon language with crisp silhouette, jewel fill and context-sensitive bloom.

## 2. Semantic Glow

Glow color means state: lavender active, cyan agent-working, amber attention, coral failure.

## 3. OLED Pure Mode

True-black surfaces with emissive accents and reduced large-area luminance.

## 4. Lavender Aurora Glass

North Star lavender/indigo glass tokens with monitor-aware contrast.

## 5. Chromatic Focus Ring

One precise focus halo morphing between keyboard, voice and agent ownership.

## 6. Depth Without Double Borders

Use luminance and inner highlights instead of stacked outlines.

## 7. Micro-Texture Layer

Optional ultra-subtle grain/diffusion to avoid sterile flat glass.

## 8. Adaptive Saturation

Dim inactive surfaces while preserving active content saturation.

## 9. Per-Activity Theme Accent

Development, Media, Ops and Research can carry restrained accent variants.

## 10. HDR-Aware Bloom

Clamp glow intensity based on HDR/SDR output and user luminance preference.

## 11. Semantic Weg Badges

Window groups show project alias, agent state and attention, not meaningless process dots.

## 12. Elastic Dock Lens

Optional hover magnification while logical positions remain stable for muscle memory.

## 13. Grouped Preview Ribbon

Compact ordered draggable cards visually attached to their app group.

## 14. Persistent Window Slots

TaskQoS/XeOm/new-chat keep the same spatial slot across focus and restart.

## 15. Pinned Tray Candy Cluster

Top-toolbar tray icons render as first-class candy glyphs while preserving native click semantics.

## 16. Activity Chip

Toolbar shows current Activity with one-click switch and voice state.

## 17. Agent Presence Pips

Tiny animated markers show which app/window an agent is acting in.

## 18. Attention Heat

Subtle edge heat indicates pending action instead of aggressive flashing.

## 19. Window Alias Plates

Short user aliases shown consistently in Weg, Overview, Task Switcher and voice confirmations.

## 20. Monitor Role Labels

Monitors can be Editor/Research/Media/Comms and layout rules target roles.

## 21. Frozen Task Switcher Geometry

Switcher order never changes merely because focus changed.

## 22. Switcher Search-As-Type

Type while Alt-Tab is held to filter the frozen session.

## 23. Overview Constellation

Overview groups related windows around Projects/Activities rather than a flat process list.

## 24. Spatial History Trail

Optional faint trail shows last three workspace transitions without reordering anything.

## 25. Window Set Focus

Focus a named set such as Coding, Ops or Communication in one action.

## 26. One-Hand Keyboard Navigation

Consistent arrows, Tab, numbers and fuzzy jump across shell surfaces.

## 27. Nearest-Intent Projection

Visual preview may project likely next window closer to pointer without changing logical order.

## 28. Semantic Minimize

Minimized windows remain identity objects with frozen preview and context.

## 29. Live Window Health

Mark hung/restarting windows and recovery state without blocking other widgets.

## 30. Per-Window Privacy Shield

Hide preview thumbnail/content while keeping the window manageable.

## 31. Voice Cursor

Small indicator shows what object Jarvis believes "this/that" refers to.

## 32. Agent Cursor

Distinct visual marker shows an agent's target before a consequential action.

## 33. Action Breadcrumb

Transient compact breadcrumb such as `Jarvis → Velo → thread → draft created`.

## 34. Approval Halo

High-impact actions get a crisp confirmation halo around the exact target object.

## 35. Undo Toast

Every reversible AI action offers one deterministic undo.

## 36. Agent Replay Scrubber

Inspect a task as timestamped semantic actions, inspired by BrowserOS neo replay.

## 37. Parallel Agent Lanes

Overview shows concurrent agents as separate lanes with resource/QoS state.

## 38. Capability Peek

Alt-hover an app to show NAI capabilities: search, compose, focus, export, automate.

## 39. Context Capsule Badge

Windows display which capsule they belong to, preventing accidental cross-project edits.

## 40. Ambient Quiet Mode

Suppress decorative motion/glow during deep work while preserving semantic color.

## 41. PiP Magnet Zones

Drag media toward screen corners to snap into persistent OLED PiP regions.

## 42. Vertical Shorts Rail

Optional edge rail provides wheel/swipe vertical media queue without taking over the desktop.

## 43. Now Playing Lens

Hover media badge for creator, channel, queue reason and visual-match confidence.

## 44. Cross-App Drop Targets

Drag mail attachment to Writer, browser context to chat, document to V271 with semantic intent.

## 45. Smart Clipboard Shelf

Clipboard items appear as typed objects: code, image, URL, table, file, credential-protected.

## 46. Notification Capsules

Notifications group by project/person/agent rather than only application.

## 47. Microphone Presence Ring

Voice activity renders a tiny hardware-linked ring with explicit source/privacy state.

## 48. Candy Progress Orbits

Long operations use thin orbit/progress traces around app icons instead of giant bars.

## 49. Haptic-Like Visual Response

Buttons compress/light quickly enough to feel tactile without excessive animation.

## 50. Accessibility Contrast Governor

Automatically limits glass/bloom when contrast or reduced-transparency preferences require it.

---

# Visual system implementation notes

## Palette defaults

```text
NAI Lavender Core       #8D7CFF
NAI Electric Indigo     #6557FF
NAI Iris                #A778FF
NAI Aurora Violet       #C48CFF
NAI Plasma Cyan         #4DEBFF
NAI Signal Mint         #56F5C2
NAI Hot Coral           #FF5D8F
NAI Warning Amber       #FFC857
NAI OLED Ink            #070710
NAI Evening Surface     #10101A
```

These are design-token defaults, never component literals.

## Candy Sugar icon anatomy

Every NAI icon may have four layers:

```text
silhouette
inner jewel fill
edge highlight
semantic glow
```

States:

```text
idle
hover
pressed
active
attention
agent-working
error
offline
```

At 16/20 px readability beats decoration. Halo belongs outside the glyph. Third-party icons are wrapped in a Candy Tile
rather than destructively recolored.

## Motion budgets

```text
interactive target: 80–140 ms
panel morph: 140–240 ms
workspace transition: 220–360 ms
```

Reduced-motion mode is first class.
