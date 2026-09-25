//! NAI hardware + AI-residency telemetry (ADR/14 §6).
//!
//! One deterministic sample per call, with explicit source attribution and
//! distinct `measured` / `estimated` / `unavailable` states. Nothing is
//! inferred from a generic counter when a vendor API answered.

use serde::Deserialize;
use serde_json::{Value, json};

use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory1, DXGI_ADAPTER_DESC1, DXGI_MEMORY_SEGMENT_GROUP,
    DXGI_MEMORY_SEGMENT_GROUP_LOCAL, DXGI_MEMORY_SEGMENT_GROUP_NON_LOCAL,
    DXGI_QUERY_VIDEO_MEMORY_INFO, IDXGIAdapter1, IDXGIAdapter3, IDXGIFactory1,
};
use windows_core::Interface;

fn cpu_sample(sys: &mut sysinfo::System) -> Value {
    sys.refresh_cpu_all();
    let cores = sys.cpus();
    let count = cores.len().max(1);
    let usage: f32 = cores.iter().map(|c| c.cpu_usage()).sum::<f32>() / count as f32;
    let frequency: u64 = cores.iter().map(|c| c.frequency()).sum::<u64>() / count as u64;
    json!({
        "usagePercent": usage as f64,
        "frequencyMHz": frequency,
        "cores": count,
        "definition": "mean per-core busy percentage",
        "source": "sysinfo",
    })
}

fn memory_sample(sys: &mut sysinfo::System) -> Value {
    sys.refresh_memory();
    json!({
        "usedBytes": sys.used_memory(),
        "totalBytes": sys.total_memory(),
        "freeBytes": sys.free_memory(),
        "definition": "physical working set (used = total - free)",
        "source": "sysinfo",
    })
}

fn trim_wide(raw: &[u16]) -> String {
    let end = raw.iter().position(|c| *c == 0).unwrap_or(raw.len());
    String::from_utf16_lossy(&raw[..end])
}

fn memory_info(adapter: &IDXGIAdapter1, group: DXGI_MEMORY_SEGMENT_GROUP) -> Value {
    let Ok(adapter3) = adapter.cast::<IDXGIAdapter3>() else {
        return json!({ "state": "unavailable" });
    };
    let mut info = DXGI_QUERY_VIDEO_MEMORY_INFO::default();
    if unsafe { adapter3.QueryVideoMemoryInfo(0, group, &mut info) }.is_err() {
        return json!({ "state": "unavailable" });
    }
    // The WDDM budget is a scheduling hint, not a hard occupancy total.
    json!({
        "usedBytes": info.CurrentUsage,
        "budgetBytes": info.Budget,
        "reservationBytes": info.CurrentReservation,
        "state": "measured",
        "note": "WDDM budget (estimated occupancy, not a hard total)",
    })
}

/// DXGI adapter table: LUID + PCI identity + dedicated/shared budget & usage.
fn adapter_samples() -> Vec<Value> {
    let mut adapters = Vec::new();
    let Ok(factory) = (unsafe { CreateDXGIFactory1::<IDXGIFactory1>() }) else {
        return adapters;
    };

    let mut index = 0u32;
    loop {
        let Ok(adapter) = (unsafe { factory.EnumAdapters1(index) }) else {
            break;
        };
        index += 1;

        let Ok(desc) = (unsafe { adapter.GetDesc1() }) else {
            continue;
        };
        let desc: DXGI_ADAPTER_DESC1 = desc;

        adapters.push(json!({
            "index": index - 1,
            "name": trim_wide(&desc.Description),
            "luid": format!("{}:{}", desc.AdapterLuid.HighPart, desc.AdapterLuid.LowPart),
            "pci": {
                "vendorId": format!("0x{:04X}", desc.VendorId),
                "deviceId": format!("0x{:04X}", desc.DeviceId),
                "subSystemId": format!("0x{:08X}", desc.SubSysId),
            },
            "dedicatedTotalBytes": desc.DedicatedVideoMemory as u64,
            "dedicated": memory_info(&adapter, DXGI_MEMORY_SEGMENT_GROUP_LOCAL),
            "shared": memory_info(&adapter, DXGI_MEMORY_SEGMENT_GROUP_NON_LOCAL),
            "source": "dxgi",
        }));
    }
    adapters
}

#[derive(Deserialize)]
struct NpuEntity {
    #[serde(rename = "Name")]
    name: String,
}

/// NPU presence via the Plug & Play entity table. Explicitly `unavailable`
/// when no NPU device node exists (no generic CPU/NPU equivalence claim).
fn npu_sample() -> Value {
    let connection = match wmi::WMIConnection::new() {
        Ok(connection) => connection,
        Err(error) => return json!({ "state": "unavailable", "note": error.to_string() }),
    };

    let names: Vec<String> = match connection.query::<NpuEntity>() {
        Ok(rows) => rows.into_iter().map(|row| row.name).collect(),
        Err(error) => return json!({ "state": "unavailable", "note": error.to_string() }),
    };

    let npu = names.iter().find(|name| {
        let lower = name.to_lowercase();
        lower.contains("npu") || lower.contains("neural processing")
    });

    match npu {
        Some(name) => json!({
            "state": "present",
            "name": name,
            "activity": "estimated",
            "source": "wmi:Win32_PnPEntity",
        }),
        None => json!({
            "state": "unavailable",
            "name": "NPU: unavailable/unsupported",
            "source": "wmi:Win32_PnPEntity",
        }),
    }
}

/// Model / provider residency: gateway contract plus live process identity.
fn model_samples() -> Value {
    use sysinfo::ProcessesToUpdate;
    let mut sys = sysinfo::System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);

    let mut running: Vec<Value> = Vec::new();
    for process in sys.processes().values() {
        let name = process.name().to_string_lossy().to_lowercase();
        if name.contains("llama-server") || name.contains("jarvis") || name.contains("openvino") {
            running.push(json!({
                "pid": process.pid().as_u32(),
                "name": name,
                "memoryBytes": process.memory(),
            }));
        }
    }

    json!({
        "contract": crate::modules::nai::gateway_models(),
        "running": running,
        "attribution": "process-name correlation; KV cache is not published by every backend",
        "source": "gateway-contract + sysinfo",
    })
}

/// One complete telemetry sample.
pub fn sample() -> Value {
    let mut sys = sysinfo::System::new_all();
    // prime the delta counters, then read the real window
    sys.refresh_cpu_all();
    std::thread::sleep(std::time::Duration::from_millis(10));

    json!({
        "schema": "nai.telemetry/v1",
        "sampledAtMs": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
        "cpu": cpu_sample(&mut sys),
        "memory": memory_sample(&mut sys),
        "adapters": adapter_samples(),
        "npu": npu_sample(),
        "models": model_samples(),
    })
}
