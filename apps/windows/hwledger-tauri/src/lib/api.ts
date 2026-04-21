// Typed Tauri invoke wrappers. Mirrors `src-tauri/src/commands.rs`.

import { invoke } from "@tauri-apps/api/core";

export type KvQuant = "fp16" | "fp8" | "int8" | "int4" | "threebit";
export type WeightQuant = "fp16" | "bf16" | "int8" | "int4" | "threebit";

export interface PlannerInput {
  config_json: string;
  seq_len: number;
  concurrent_users: number;
  batch_size: number;
  kv_quant: KvQuant;
  weight_quant: WeightQuant;
}

export interface PlannerResult {
  weights_bytes: number;
  kv_bytes: number;
  prefill_activation_bytes: number;
  runtime_overhead_bytes: number;
  total_bytes: number;
  attention_kind_label: string;
  effective_batch: number;
}

export interface DeviceInfo {
  id: number;
  backend: string;
  name: string;
  uuid: string | null;
  total_vram_bytes: number;
}

export interface TelemetrySample {
  device_id: number;
  free_vram_bytes: number;
  util_percent: number;
  temperature_c: number;
  power_watts: number;
  captured_at_ms: number;
}

export interface HfSearchInput {
  text?: string;
  tags?: string[];
  library?: string;
  sort?: "downloads" | "likes" | "recent";
  limit?: number;
  min_downloads?: number;
  author?: string;
  pipeline_tag?: string;
  token?: string;
}

export interface HfModelCard {
  id: string;
  author?: string;
  downloads?: number;
  likes?: number;
  pipeline_tag?: string;
  library_name?: string;
  tags?: string[];
}

export const api = {
  plan: (input: PlannerInput) => invoke<PlannerResult>("plan", { input }),
  planLayerContributions: (input: PlannerInput) =>
    invoke<number[]>("plan_layer_contributions", { input }),
  probeDetect: () => invoke<DeviceInfo[]>("probe_detect"),
  probeSample: (deviceId: number, backend: string) =>
    invoke<TelemetrySample>("probe_sample", { deviceId, backend }),
  hfSearch: (query: HfSearchInput) =>
    invoke<HfModelCard[]>("hf_search", { query }),
  coreVersion: () => invoke<string>("core_version"),
};

export function formatBytes(n: number): string {
  if (n === 0) return "0 B";
  const k = 1024;
  const units = ["B", "KiB", "MiB", "GiB", "TiB"];
  const i = Math.min(units.length - 1, Math.floor(Math.log(n) / Math.log(k)));
  return `${(n / Math.pow(k, i)).toFixed(2)} ${units[i]}`;
}
