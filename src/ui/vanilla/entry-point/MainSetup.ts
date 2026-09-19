import type { Widget } from "@seelen-ui/lib/types";
import { _invoke, webviewInfo } from "src/ui/vanilla/entry-point/_tauri.ts";
import { listen } from "@tauri-apps/api/event";

import { hookLocalStorage } from "src/ui/vanilla/entry-point/_LocalStorage";

console.debug("boot: MainSetup start");

function record(stage: string): void {
  _invoke("record_boot_stage", { stage }).catch(() => {});
}

record("bootstrap.widget_definition.request.start");
const indexJsCode = fetch("./index.js").then((res) => res.text());

// initialize global widget variable, needed by slu-lib
const currentWidgetId = webviewInfo.widgetId;
const widgetList = await _invoke<Widget[]>("state_get_widgets");
record("bootstrap.widget_definition.request.done");
console.debug(`boot: state_get_widgets success (${widgetList.length} widgets)`);
window.__SLU_WIDGET = widgetList.find((widget) => widget.id === currentWidgetId)!;

if (!window.__SLU_WIDGET) {
  throw new Error(`Widget definition not found for ${currentWidgetId}`);
}
record("bootstrap.widget_definition.resolved");
console.debug(`boot: __SLU_WIDGET resolved for ${currentWidgetId}`);

// reload if widget definition changed
listen<Widget[]>("widgets-changed", ({ payload }) => {
  const actual = payload.find((widget) => widget.id === currentWidgetId);
  if (actual && JSON.stringify(actual) !== JSON.stringify(window.__SLU_WIDGET)) {
    window.location.reload();
  }
});

// set document id
document.documentElement.id = currentWidgetId;

// hook local storage, to avoid collition of keys
hookLocalStorage(currentWidgetId);
record("bootstrap.local_storage.hooked");

// add base css
const link = document.createElement("link");
link.rel = "stylesheet";
link.href = "/vanilla/entry-point/index.css";
document.head.appendChild(link);

// load index.js
record("bootstrap.widget_entry.fetch.start");
const script = document.createElement("script");
script.type = "module";
script.textContent = await indexJsCode;
record("bootstrap.widget_entry.fetch.done");
console.debug("boot: vanilla index.js fetched");
document.head.appendChild(script);
record("bootstrap.widget_entry.injected");
