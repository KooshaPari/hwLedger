import { createSignal, Show, type Component } from "solid-js";
import {
  api,
  formatBytes,
  type KvQuant,
  type PlannerResult,
  type WeightQuant,
} from "../lib/api";

// Minimal llama-3.1-8b-ish config — matches the Planner spec's default preset
// used across the SwiftUI app's Planner screen.
const DEFAULT_CONFIG = JSON.stringify(
  {
    num_hidden_layers: 32,
    hidden_size: 4096,
    num_attention_heads: 32,
    num_key_value_heads: 8,
    vocab_size: 128256,
  },
  null,
  2,
);

const KV_OPTS: KvQuant[] = ["fp16", "fp8", "int8", "int4", "threebit"];
const W_OPTS: WeightQuant[] = ["fp16", "bf16", "int8", "int4", "threebit"];

export const PlannerScreen: Component = () => {
  const [modelLabel, setModelLabel] = createSignal("llama-3.1-8b (preset)");
  const [configJson, setConfigJson] = createSignal(DEFAULT_CONFIG);
  const [seqLen, setSeqLen] = createSignal(4096);
  const [users, setUsers] = createSignal(1);
  const [batch, setBatch] = createSignal(1);
  const [kv, setKv] = createSignal<KvQuant>("fp16");
  const [weight, setWeight] = createSignal<WeightQuant>("fp16");
  const [result, setResult] = createSignal<PlannerResult | null>(null);
  const [err, setErr] = createSignal<string | null>(null);
  const [busy, setBusy] = createSignal(false);

  const runPlan = async () => {
    setErr(null);
    setBusy(true);
    try {
      const r = await api.plan({
        config_json: configJson(),
        seq_len: seqLen(),
        concurrent_users: users(),
        batch_size: batch(),
        kv_quant: kv(),
        weight_quant: weight(),
      });
      setResult(r);
    } catch (e: unknown) {
      setErr(errorToMessage(e));
      setResult(null);
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <header class="screen-header">
        <div>
          <h2>Planner</h2>
          <p class="screen-hint">
            Estimate GPU memory for a model · calls <code>hwledger_plan</code> via Tauri command
          </p>
        </div>
        <span class="muted" aria-live="polite">
          {result() ? `≈ ${formatBytes(result()!.total_bytes)} total` : ""}
        </span>
      </header>

      <div class="grid two">
        <section class="card" aria-label="Planner inputs">
          <div class="grid" style="gap:12px">
            <label>
              Model label
              <input
                type="text"
                value={modelLabel()}
                onInput={(e) => setModelLabel(e.currentTarget.value)}
                aria-describedby="model-hint"
              />
              <span id="model-hint" class="muted" style="font-size:11px">
                Free text — used for display only. Paste config.json below.
              </span>
            </label>

            <label>
              Sequence length: <span class="stat-value">{seqLen()}</span>
              <input
                type="range"
                min="128"
                max="131072"
                step="128"
                value={seqLen()}
                onInput={(e) => setSeqLen(parseInt(e.currentTarget.value, 10))}
                aria-valuemin="128"
                aria-valuemax="131072"
                aria-valuenow={seqLen()}
              />
            </label>

            <div class="grid two">
              <label>
                Concurrent users
                <input
                  type="number"
                  min="1"
                  max="1024"
                  value={users()}
                  onInput={(e) => setUsers(clampInt(e.currentTarget.value, 1, 1024, 1))}
                />
              </label>
              <label>
                Batch size
                <input
                  type="number"
                  min="1"
                  max="256"
                  value={batch()}
                  onInput={(e) => setBatch(clampInt(e.currentTarget.value, 1, 256, 1))}
                />
              </label>
            </div>

            <div class="grid two">
              <label>
                KV quantization
                <select value={kv()} onChange={(e) => setKv(e.currentTarget.value as KvQuant)}>
                  {KV_OPTS.map((o) => (
                    <option value={o}>{o.toUpperCase()}</option>
                  ))}
                </select>
              </label>
              <label>
                Weight quantization
                <select
                  value={weight()}
                  onChange={(e) => setWeight(e.currentTarget.value as WeightQuant)}
                >
                  {W_OPTS.map((o) => (
                    <option value={o}>{o.toUpperCase()}</option>
                  ))}
                </select>
              </label>
            </div>

            <label>
              config.json
              <textarea
                spellcheck={false}
                value={configJson()}
                onInput={(e) => setConfigJson(e.currentTarget.value)}
                aria-label="HuggingFace config.json contents"
              />
            </label>

            <button
              type="button"
              class="primary"
              onClick={runPlan}
              disabled={busy()}
              aria-busy={busy()}
            >
              {busy() ? "Planning…" : "Plan memory"}
            </button>
          </div>
        </section>

        <section class="card" aria-label="Planner results" aria-live="polite">
          <Show
            when={result()}
            fallback={<p class="muted">Run the planner to see a breakdown.</p>}
          >
            <Breakdown r={result()!} />
          </Show>
          <Show when={err()}>
            <p class="err" role="alert">
              {err()}
            </p>
          </Show>
        </section>
      </div>
    </>
  );
};

const Breakdown: Component<{ r: PlannerResult }> = (props) => {
  const pct = (n: number) =>
    `${((n / Math.max(1, props.r.total_bytes)) * 100).toFixed(1)}%`;

  return (
    <>
      <div class="stat-row" style="margin-bottom:16px">
        <div class="stat">
          <span class="stat-value">{formatBytes(props.r.total_bytes)}</span>
          <span class="stat-label">Total</span>
        </div>
        <div class="stat">
          <span class="stat-value">{props.r.attention_kind_label}</span>
          <span class="stat-label">Attention</span>
        </div>
        <div class="stat">
          <span class="stat-value">{props.r.effective_batch}</span>
          <span class="stat-label">Effective batch</span>
        </div>
      </div>

      <div
        class="stacked-bar"
        role="img"
        aria-label={`Memory breakdown: weights ${pct(props.r.weights_bytes)}, KV ${pct(
          props.r.kv_bytes,
        )}, activation ${pct(props.r.prefill_activation_bytes)}, overhead ${pct(
          props.r.runtime_overhead_bytes,
        )}`}
      >
        <div class="seg weights" style={{ "flex-basis": pct(props.r.weights_bytes) }} />
        <div class="seg kv" style={{ "flex-basis": pct(props.r.kv_bytes) }} />
        <div
          class="seg act"
          style={{ "flex-basis": pct(props.r.prefill_activation_bytes) }}
        />
        <div
          class="seg overhead"
          style={{ "flex-basis": pct(props.r.runtime_overhead_bytes) }}
        />
      </div>
      <div class="legend">
        <span>
          <span class="swatch" style="background:var(--accent)" />
          Weights · {formatBytes(props.r.weights_bytes)}
        </span>
        <span>
          <span class="swatch" style="background:var(--accent-2)" />
          KV · {formatBytes(props.r.kv_bytes)}
        </span>
        <span>
          <span class="swatch" style="background:var(--ok)" />
          Activation · {formatBytes(props.r.prefill_activation_bytes)}
        </span>
        <span>
          <span class="swatch" style="background:var(--muted)" />
          Overhead · {formatBytes(props.r.runtime_overhead_bytes)}
        </span>
      </div>

      <details style="margin-top:14px">
        <summary class="muted">Raw JSON</summary>
        <pre style="font-family:var(--mono); font-size:12px; background:var(--surface-2); padding:10px; border-radius:6px; overflow:auto">
{JSON.stringify(props.r, null, 2)}
        </pre>
      </details>
    </>
  );
};

function clampInt(raw: string, min: number, max: number, fallback: number): number {
  const n = parseInt(raw, 10);
  if (Number.isNaN(n)) return fallback;
  return Math.min(max, Math.max(min, n));
}

function errorToMessage(e: unknown): string {
  if (e && typeof e === "object" && "message" in e) {
    return String((e as { message: unknown }).message);
  }
  return String(e);
}
