import { createSignal, For, Show, type Component } from "solid-js";
import { api, type HfModelCard } from "../lib/api";

/** HF Search screen — real `hf_search` Tauri command, anonymous (no token). */
export const HfSearchScreen: Component = () => {
  const [q, setQ] = createSignal("llama");
  const [limit, setLimit] = createSignal(20);
  const [results, setResults] = createSignal<HfModelCard[]>([]);
  const [busy, setBusy] = createSignal(false);
  const [err, setErr] = createSignal<string | null>(null);

  const search = async (e: Event) => {
    e.preventDefault();
    setErr(null);
    setBusy(true);
    try {
      const r = await api.hfSearch({ text: q(), limit: limit(), sort: "downloads" });
      setResults(r);
    } catch (e2) {
      setErr(String((e2 as Error)?.message ?? e2));
      setResults([]);
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <header class="screen-header">
        <div>
          <h2>HF Search</h2>
          <p class="screen-hint">Search Hugging Face models (anonymous)</p>
        </div>
      </header>

      <form class="card" onSubmit={search} aria-label="Search query">
        <div class="grid two" style="align-items:end">
          <label>
            Query
            <input
              type="search"
              value={q()}
              onInput={(e) => setQ(e.currentTarget.value)}
              aria-label="Text query"
              placeholder="e.g. qwen coder"
            />
          </label>
          <label>
            Limit
            <input
              type="number"
              min="1"
              max="100"
              value={limit()}
              onInput={(e) => setLimit(Math.max(1, Math.min(100, parseInt(e.currentTarget.value, 10) || 20)))}
            />
          </label>
        </div>
        <button class="primary" type="submit" style="margin-top:12px" disabled={busy()}>
          {busy() ? "Searching…" : "Search"}
        </button>
      </form>

      <section class="card" style="margin-top:16px" aria-label="Results">
        <Show
          when={results().length > 0}
          fallback={<p class="muted">No results yet.</p>}
        >
          <table>
            <thead>
              <tr>
                <th>Model</th>
                <th>Author</th>
                <th>↓ Downloads</th>
                <th>♥ Likes</th>
                <th>Pipeline</th>
              </tr>
            </thead>
            <tbody>
              <For each={results()}>
                {(m) => (
                  <tr>
                    <td><code>{m.id}</code></td>
                    <td>{m.author ?? "—"}</td>
                    <td>{m.downloads?.toLocaleString() ?? "—"}</td>
                    <td>{m.likes?.toLocaleString() ?? "—"}</td>
                    <td>{m.pipeline_tag ?? "—"}</td>
                  </tr>
                )}
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
