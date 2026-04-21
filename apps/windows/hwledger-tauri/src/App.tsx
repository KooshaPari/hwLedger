import { createSignal, Show, For, type Component } from "solid-js";
import { PlannerScreen } from "./screens/Planner";
import { ProbeScreen } from "./screens/Probe";
import { FleetScreen } from "./screens/Fleet";
import { HfSearchScreen } from "./screens/HfSearch";

type ScreenId = "planner" | "probe" | "fleet" | "hf";

interface NavItem {
  id: ScreenId;
  label: string;
  hint: string;
}

const SCREENS: NavItem[] = [
  { id: "planner", label: "Planner", hint: "Size GPU memory for a model" },
  { id: "probe", label: "Probe", hint: "Detect & sample local GPUs" },
  { id: "fleet", label: "Fleet", hint: "Manage remote hosts" },
  { id: "hf", label: "HF Search", hint: "Find models on Hugging Face" },
];

export const App: Component = () => {
  const [current, setCurrent] = createSignal<ScreenId>("planner");

  return (
    <div class="app-shell">
      <aside class="nav" aria-label="Primary navigation">
        <h1 class="nav-title" aria-label="hwLedger">hwLedger</h1>
        <ul role="tablist" class="nav-list">
          <For each={SCREENS}>
            {(item) => (
              <li role="presentation">
                <button
                  type="button"
                  role="tab"
                  aria-selected={current() === item.id}
                  aria-controls={`panel-${item.id}`}
                  id={`tab-${item.id}`}
                  class="nav-btn"
                  classList={{ active: current() === item.id }}
                  title={item.hint}
                  onClick={() => setCurrent(item.id)}
                >
                  {item.label}
                </button>
              </li>
            )}
          </For>
        </ul>
        <p class="nav-foot">Tauri 2 · SolidJS · hwledger-core</p>
      </aside>
      <main
        class="screen"
        id={`panel-${current()}`}
        role="tabpanel"
        aria-labelledby={`tab-${current()}`}
        tabindex="0"
      >
        <Show when={current() === "planner"}><PlannerScreen /></Show>
        <Show when={current() === "probe"}><ProbeScreen /></Show>
        <Show when={current() === "fleet"}><FleetScreen /></Show>
        <Show when={current() === "hf"}><HfSearchScreen /></Show>
      </main>
    </div>
  );
};
