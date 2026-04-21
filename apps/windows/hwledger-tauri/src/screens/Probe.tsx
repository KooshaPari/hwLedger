import { createSignal, For, Show, type Component } from "solid-js";
import { api, formatBytes, type DeviceInfo, type TelemetrySample } from "../lib/api";

/** Probe screen — real `probe_detect` + `probe_sample` Tauri commands wired up. */
export const ProbeScreen: Component = () => {
  const [devices, setDevices] = createSignal<DeviceInfo[]>([]);
  const [samples, setSamples] = createSignal<Record<number, TelemetrySample>>({});
  const [busy, setBusy] = createSignal(false);
  const [err, setErr] = createSignal<string | null>(null);

  const detect = async () => {
    setErr(null);
    setBusy(true);
    try {
      setDevices(await api.probeDetect());
    } catch (e) {
      setErr(String((e as Error)?.message ?? e));
    } finally {
      setBusy(false);
    }
  };

  const sampleOne = async (d: DeviceInfo) => {
    try {
      const s = await api.probeSample(d.id, d.backend);
      setSamples({ ...samples(), [d.id]: s });
    } catch (e) {
      setErr(String((e as Error)?.message ?? e));
    }
  };

  return (
    <>
      <header class="screen-header">
        <div>
          <h2>Probe</h2>
          <p class="screen-hint">
            Detect local GPUs across NVIDIA/AMD/Metal/Intel and sample telemetry
          </p>
        </div>
        <button class="primary" type="button" onClick={detect} disabled={busy()}>
          {busy() ? "Detecting…" : "Detect GPUs"}
        </button>
      </header>

      <section class="card" aria-label="Detected devices">
        <Show when={devices().length > 0} fallback={<p class="muted">No GPUs detected yet. Click "Detect GPUs".</p>}>
          <table>
            <thead>
              <tr>
                <th>Backend</th>
                <th>Name</th>
                <th>Total VRAM</th>
                <th>Telemetry</th>
                <th />
              </tr>
            </thead>
            <tbody>
              <For each={devices()}>
                {(d) => {
                  const s = () => samples()[d.id];
                  return (
                    <tr>
                      <td><code>{d.backend}</code></td>
                      <td>{d.name}</td>
                      <td>{formatBytes(d.total_vram_bytes)}</td>
                      <td>
                        <Show when={s()} fallback={<span class="muted">—</span>}>
                          util {s()!.util_percent.toFixed(0)}% · {s()!.temperature_c.toFixed(0)}°C ·
                          {" "}
                          {s()!.power_watts.toFixed(0)}W · free {formatBytes(s()!.free_vram_bytes)}
                        </Show>
                      </td>
                      <td>
                        <button type="button" onClick={() => sampleOne(d)}>Sample</button>
                      </td>
                    </tr>
                  );
                }}
              </For>
            </tbody>
          </table>
        </Show>
        <Show when={err()}>
          <p class="err" role="alert">{err()}</p>
        </Show>
      </section>
    </>
  );
};
