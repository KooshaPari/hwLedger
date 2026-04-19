---
title: GPU Telemetry Backends — Rust Probe Matrix
description: NVIDIA via nvml-wrapper, AMD via rocm-smi shell, Apple Silicon via macmon shell, Intel Arc deferred. Single GpuProbe trait, 4 platform-specific implementations.
brief_id: 6
status: archived
date: 2026-04-18
sources:
  - url: https://docs.nvidia.com/deploy/nvml-api/
    title: NVIDIA Management Library (NVML) API
  - url: https://github.com/Codeplay/computecpp-sdk/blob/master/doc/rocm-smi-manual.md
    title: rocm-smi Documentation
  - url: https://github.com/apple/swift-system
    title: Apple Swift System (Metal framework reference)
  - url: https://github.com/turing-machines/macmon
    title: macmon — Apple Silicon GPU Monitor
---

# GPU Telemetry Backends — Rust Probe Matrix

## Overview

Real-time GPU memory and utilization telemetry is critical for hwLedger's runtime planner. Each platform has different API maturity:

| Platform | API | Maturity | Rust Crate |
|----------|-----|----------|-----------|
| NVIDIA | NVML | Mature | `nvml-wrapper` |
| AMD Radeon | rocm-smi CLI | Fragmented | Shell out (no prod crate) |
| Apple Silicon | Metal (private) | Closed | Shell out to `macmon` |
| Intel Arc | N/A | Vacuum | Deferred to v2 |

**Architecture**: Single `GpuProbe` trait; 4 implementations; runtime selection by platform.

## 1. NVIDIA: nvml-wrapper

### Dependencies

```toml
[dependencies]
nvml-wrapper = "0.10"
thiserror = "1.0"
serde = { version = "1.0", features = ["derive"] }
```

### Trait Definition

```rust
pub trait GpuProbe {
    fn list_devices(&self) -> Result<Vec<GpuDevice>>;
    fn get_memory_info(&self, device_id: u32) -> Result<MemoryInfo>;
    fn get_utilization(&self, device_id: u32) -> Result<Utilization>;
    fn get_temperature(&self, device_id: u32) -> Result<f32>;
    fn get_power_draw(&self, device_id: u32) -> Result<f32>; // Watts
}

pub struct GpuDevice {
    pub id: u32,
    pub name: String,
    pub arch: String,
    pub compute_capability: String,
}

pub struct MemoryInfo {
    pub total_mb: u64,
    pub free_mb: u64,
    pub used_mb: u64,
}

pub struct Utilization {
    pub gpu_percent: u32,
    pub memory_percent: u32,
}
```

### NVIDIA Implementation

```rust
use nvml_wrapper::Nvml;
use nvml_wrapper::enum_wrappers::device::MemoryErrorCounter;

pub struct NvidiaProbe {
    nvml: Nvml,
}

impl NvidiaProbe {
    pub fn new() -> Result<Self, IngestError> {
        let nvml = Nvml::init()
            .map_err(|e| IngestError::ProbeError(format!("NVML init failed: {}", e)))?;
        Ok(Self { nvml })
    }
}

impl GpuProbe for NvidiaProbe {
    fn list_devices(&self) -> Result<Vec<GpuDevice>> {
        let count = self.nvml.device_count()
            .map_err(|e| IngestError::ProbeError(format!("Device count: {}", e)))?;

        let devices = (0..count)
            .filter_map(|i| {
                self.nvml.device_by_index(i).ok().map(|dev| {
                    let name = dev.name().unwrap_or_else(|_| "Unknown".to_string());
                    let compute_cap = dev.compute_capability()
                        .ok()
                        .map(|(major, minor)| format!("{}.{}", major, minor))
                        .unwrap_or_default();
                    
                    GpuDevice {
                        id: i,
                        name,
                        arch: "NVIDIA".to_string(),
                        compute_capability: compute_cap,
                    }
                })
            })
            .collect();

        Ok(devices)
    }

    fn get_memory_info(&self, device_id: u32) -> Result<MemoryInfo> {
        let device = self.nvml.device_by_index(device_id)
            .map_err(|e| IngestError::ProbeError(format!("Device {}: {}", device_id, e)))?;

        let mem_info = device.memory_info()
            .map_err(|e| IngestError::ProbeError(format!("Memory info: {}", e)))?;

        Ok(MemoryInfo {
            total_mb: mem_info.total / (1024 * 1024),
            free_mb: mem_info.free / (1024 * 1024),
            used_mb: (mem_info.total - mem_info.free) / (1024 * 1024),
        })
    }

    fn get_utilization(&self, device_id: u32) -> Result<Utilization> {
        let device = self.nvml.device_by_index(device_id)
            .map_err(|e| IngestError::ProbeError(format!("Device {}: {}", device_id, e)))?;

        let util = device.utilization_rates()
            .map_err(|e| IngestError::ProbeError(format!("Utilization: {}", e)))?;

        Ok(Utilization {
            gpu_percent: util.gpu,
            memory_percent: util.memory,
        })
    }

    fn get_temperature(&self, device_id: u32) -> Result<f32> {
        let device = self.nvml.device_by_index(device_id)
            .map_err(|e| IngestError::ProbeError(format!("Device {}: {}", device_id, e)))?;

        device.temperature()
            .map(|t| t as f32)
            .map_err(|e| IngestError::ProbeError(format!("Temperature: {}", e)))
    }

    fn get_power_draw(&self, device_id: u32) -> Result<f32> {
        let device = self.nvml.device_by_index(device_id)
            .map_err(|e| IngestError::ProbeError(format!("Device {}: {}", device_id, e)))?;

        // Power draw in milliwatts
        device.power_draw()
            .map(|pw| pw as f32 / 1000.0)
            .map_err(|e| IngestError::ProbeError(format!("Power: {}", e)))
    }
}
```

## 2. AMD Radeon: rocm-smi Shell-Out

No production-grade Rust crate exists; shell out to `rocm-smi --json`.

```rust
use serde::Deserialize;
use std::process::Command;

#[derive(Debug, Deserialize)]
struct RocmSmiOutput {
    #[serde(rename = "amd_smi_version")]
    version: String,
    #[serde(rename = "gpu_metrics")]
    devices: Vec<RocmDevice>,
}

#[derive(Debug, Deserialize)]
struct RocmDevice {
    #[serde(rename = "gpu_index")]
    index: u32,
    #[serde(rename = "product_name")]
    name: String,
    #[serde(rename = "gpu_memory_max")]
    total_memory_mb: u64,
    #[serde(rename = "gpu_memory_used")]
    used_memory_mb: u64,
    #[serde(rename = "gpu_utilization")]
    gpu_util: u32,
    #[serde(rename = "gpu_memory_util")]
    mem_util: u32,
    #[serde(rename = "temperature_edge")]
    temperature: f32,
    #[serde(rename = "power")]
    power_watts: f32,
}

pub struct AmdProbe;

impl GpuProbe for AmdProbe {
    fn list_devices(&self) -> Result<Vec<GpuDevice>> {
        let output = Command::new("rocm-smi")
            .arg("--json")
            .output()
            .map_err(|e| IngestError::ProbeError(format!("rocm-smi not found: {}", e)))?;

        if !output.status.success() {
            return Err(IngestError::ProbeError("rocm-smi failed".to_string()));
        }

        let amd_output: RocmSmiOutput = serde_json::from_slice(&output.stdout)
            .map_err(|e| IngestError::ProbeError(format!("rocm-smi JSON: {}", e)))?;

        Ok(amd_output.devices.iter().map(|d| {
            GpuDevice {
                id: d.index,
                name: d.name.clone(),
                arch: "AMD Radeon".to_string(),
                compute_capability: "Unknown".to_string(),
            }
        }).collect())
    }

    fn get_memory_info(&self, device_id: u32) -> Result<MemoryInfo> {
        let output = Command::new("rocm-smi")
            .arg("--json")
            .arg("--gpu")
            .arg(device_id.to_string())
            .output()
            .map_err(|e| IngestError::ProbeError(format!("rocm-smi query: {}", e)))?;

        let amd_output: RocmSmiOutput = serde_json::from_slice(&output.stdout)?;
        let dev = amd_output.devices.first()
            .ok_or_else(|| IngestError::ProbeError("No device found".to_string()))?;

        Ok(MemoryInfo {
            total_mb: dev.total_memory_mb,
            used_mb: dev.used_memory_mb,
            free_mb: dev.total_memory_mb - dev.used_memory_mb,
        })
    }

    fn get_utilization(&self, device_id: u32) -> Result<Utilization> {
        let output = Command::new("rocm-smi")
            .arg("--json")
            .arg("--gpu")
            .arg(device_id.to_string())
            .output()?;

        let amd_output: RocmSmiOutput = serde_json::from_slice(&output.stdout)?;
        let dev = amd_output.devices.first()
            .ok_or_else(|| IngestError::ProbeError("No device found".to_string()))?;

        Ok(Utilization {
            gpu_percent: dev.gpu_util,
            memory_percent: dev.mem_util,
        })
    }

    fn get_temperature(&self, device_id: u32) -> Result<f32> {
        let output = Command::new("rocm-smi")
            .arg("--json")
            .arg("--gpu")
            .arg(device_id.to_string())
            .output()?;

        let amd_output: RocmSmiOutput = serde_json::from_slice(&output.stdout)?;
        let dev = amd_output.devices.first()
            .ok_or_else(|| IngestError::ProbeError("No device found".to_string()))?;

        Ok(dev.temperature)
    }

    fn get_power_draw(&self, device_id: u32) -> Result<f32> {
        let output = Command::new("rocm-smi")
            .arg("--json")
            .arg("--gpu")
            .arg(device_id.to_string())
            .output()?;

        let amd_output: RocmSmiOutput = serde_json::from_slice(&output.stdout)?;
        let dev = amd_output.devices.first()
            .ok_or_else(|| IngestError::ProbeError("No device found".to_string()))?;

        Ok(dev.power_watts)
    }
}
```

## 3. Apple Silicon: macmon Shell-Out

No public Metal memory API. Shell out to **macmon** (open-source, MIT licensed):

```bash
brew install macmon
```

### Output Format

```json
{
  "gpu": {
    "total": 16384,  // MB
    "used": 8192,
    "free": 8192
  },
  "neural_engine": { ... },
  "timestamp": 1713547200
}
```

### Rust Implementation

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct MacmonOutput {
    gpu: MacGpu,
}

#[derive(Debug, Deserialize)]
struct MacGpu {
    total: u64,
    used: u64,
    free: u64,
}

pub struct AppleSiliconProbe;

impl GpuProbe for AppleSiliconProbe {
    fn list_devices(&self) -> Result<Vec<GpuDevice>> {
        Ok(vec![GpuDevice {
            id: 0,
            name: "Apple Silicon GPU".to_string(),
            arch: "Apple".to_string(),
            compute_capability: "Unknown".to_string(),
        }])
    }

    fn get_memory_info(&self, _device_id: u32) -> Result<MemoryInfo> {
        let output = Command::new("macmon")
            .arg("--json")
            .output()
            .map_err(|e| IngestError::ProbeError(format!("macmon not found: {}", e)))?;

        let macmon_out: MacmonOutput = serde_json::from_slice(&output.stdout)?;

        Ok(MemoryInfo {
            total_mb: macmon_out.gpu.total,
            used_mb: macmon_out.gpu.used,
            free_mb: macmon_out.gpu.free,
        })
    }

    fn get_utilization(&self, _device_id: u32) -> Result<Utilization> {
        let info = self.get_memory_info(0)?;
        let mem_percent = (info.used_mb * 100 / info.total_mb) as u32;

        // Apple Metal doesn't expose per-core utilization publicly
        Ok(Utilization {
            gpu_percent: 0, // Placeholder
            memory_percent: mem_percent,
        })
    }

    fn get_temperature(&self, _device_id: u32) -> Result<f32> {
        // Deferred: no public Metal thermal API
        Err(IngestError::ProbeError("Temperature unavailable on Apple Silicon".to_string()))
    }

    fn get_power_draw(&self, _device_id: u32) -> Result<f32> {
        // Deferred: no public Metal power API
        Err(IngestError::ProbeError("Power draw unavailable on Apple Silicon".to_string()))
    }
}
```

## 4. Runtime Selection

Factory function selects probe by platform:

```rust
pub fn create_probe() -> Result<Box<dyn GpuProbe>> {
    #[cfg(target_os = "macos")]
    {
        Ok(Box::new(AppleSiliconProbe))
    }

    #[cfg(target_os = "linux")]
    {
        match NvidiaProbe::new() {
            Ok(probe) => Ok(Box::new(probe)),
            Err(_) => {
                // Fallback to AMD
                Ok(Box::new(AmdProbe))
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        // DirectML or NVIDIA via CUDA
        NvidiaProbe::new().map(|p| Box::new(p) as Box<dyn GpuProbe>)
    }
}
```

## Polling Strategy

Collect metrics on a background thread:

```rust
pub struct GpuTelemetryCollector {
    probe: Box<dyn GpuProbe>,
    interval: Duration,
    tx: mpsc::Sender<GpuSnapshot>,
}

pub struct GpuSnapshot {
    pub timestamp: u64,
    pub devices: Vec<DeviceSnapshot>,
}

impl GpuTelemetryCollector {
    pub fn start(self) {
        std::thread::spawn(move || {
            loop {
                if let Ok(devices) = self.probe.list_devices() {
                    let mut snapshots = vec![];
                    for dev in devices {
                        let mem = self.probe.get_memory_info(dev.id).ok();
                        let util = self.probe.get_utilization(dev.id).ok();
                        let temp = self.probe.get_temperature(dev.id).ok();
                        let power = self.probe.get_power_draw(dev.id).ok();

                        snapshots.push(DeviceSnapshot {
                            id: dev.id,
                            memory: mem,
                            utilization: util,
                            temperature: temp,
                            power: power,
                        });
                    }

                    let _ = self.tx.send(GpuSnapshot {
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                        devices: snapshots,
                    });
                }

                std::thread::sleep(self.interval);
            }
        });
    }
}
```

## See also

- Brief 03: Inference Engine Matrix
- ADR-0004: Math Core Dispatch
- `crates/hwledger-probe/src/`

## Sources

- [NVIDIA Management Library (NVML) API Documentation](https://docs.nvidia.com/deploy/nvml-api/)
- [rocm-smi Documentation](https://github.com/ROCmSoftwarePlatform/rocm-smi/wiki)
- [macmon — Apple Silicon GPU Monitor](https://github.com/turing-machines/macmon)
- [nvml-wrapper Rust Crate](https://docs.rs/nvml-wrapper/latest/nvml_wrapper/)
