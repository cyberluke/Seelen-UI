import { _invoke, webviewInfo } from "./_tauri";
import { emitTo, listen } from "@tauri-apps/api/event";

interface LivenessPing {
  generation: number;
  nonce: number;
  sentAt: number;
}

function record(stage: string): void {
  _invoke("record_boot_stage", { stage }).catch(() => {});
}

// important in case of unexpected crash like Out of Memory
listen<LivenessPing>(
  "internal::liveness-ping",
  ({ payload }) => {
    // generation + nonce are echoed so the watchdog only accepts pong pairs
    // belonging to the current renderer generation
    emitTo(webviewInfo.rawLabel, "internal::liveness-pong", {
      generation: payload?.generation ?? 0,
      nonce: payload?.nonce ?? 0,
    });
  },
  {
    target: {
      kind: "WebviewWindow",
      label: webviewInfo.rawLabel,
    },
  },
).then(() => {
  record("bootstrap.liveness.listener.registered");
  console.debug("boot: LivenessProve registered");
});
