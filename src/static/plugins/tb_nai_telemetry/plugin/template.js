const t = naiTelemetry || {};
const cpu = t.cpu || {};
const mem = t.memory || {};
const npu = t.npu || {};
const adapters = t.adapters || [];

const gib = (bytes) => (bytes ? (bytes / 1073741824).toFixed(1) : "0");

// Dedicated GPU (largest dedicated memory) is the dGPU; the rest is iGPU/shared.
const dgpu = adapters.reduce(
  (best, a) => ((a.dedicatedTotalBytes || 0) > (best ? best.dedicatedTotalBytes || 0 : -1) ? a : best),
  null,
);
const igpu = adapters.find((a) => a !== dgpu) || null;

const parts = [
  icon("LuCpu"),
  " ",
  (cpu.usagePercent ?? 0).toFixed(0) + "%",
  " · ",
  icon("FaMemory"),
  " ",
  gib(mem.usedBytes) + "/" + gib(mem.totalBytes) + "GB",
];

if (npu.state === "present") {
  parts.push(" · NPU " + (npu.activity || "measured"));
} else {
  parts.push(" · NPU: unavailable/unsupported");
}

if (igpu) {
  parts.push(" · iGPU " + gib(igpu.shared && igpu.shared.usedBytes) + "GB");
}
if (dgpu) {
  parts.push(" · dGPU " + gib(dgpu.dedicated && dgpu.dedicated.usedBytes) + "/" + gib(dgpu.dedicatedTotalBytes) + "GB");
}

const running = (t.models && t.models.running) || [];
parts.push(" · " + (running.length ? running.length + " model proc" : "no model proc"));

return parts;
