# Visual walkthrough: planning DeepSeek-V3 from scratch

This page is the reference implementation of hwLedger's inline-screenshot
walkthrough style. Every narrative step is anchored to a specific keyframe
with callouts pointing at the exact pixel that step describes. No video — just
prose and 1:1 screenshots, the way a game walkthrough wiki teaches boss fights.

If you want a fast lookup instead, see [CLI reference](/reference/cli). If you
want the recorded tape, see the [CLI journey](/journeys/cli-plan-deepseek).

---

## Step 1 — Install and sanity-check

You start with a fresh shell and install hwLedger via cargo. The install prints
the resolved version plus the path to the binary, which you verify with
`--version`.

<Shot src="/cli-journeys/keyframes/install-cargo/frame-001.png"
      caption="cargo install hwLedger — download starts, version resolved"
      size="medium" align="right"
      :annotations='[{"bbox":[40,40,520,24],"label":"cargo install line","color":"#f9e2af"}]' />

Once the linker finishes, you get a `Compiling hwledger-cli` line and the
binary lands in `~/.cargo/bin/hwledger`.

<Shot src="/cli-journeys/keyframes/install-cargo/frame-004.png"
      caption="Install complete — binary path printed"
      size="small" align="left" />

---

## Step 2 — Open the planner help

The planner is the first command you'll use. Type `hwledger plan --help` to
discover the flags. Two flags matter most for DeepSeek: `--context` and
`--batch`.

<Shot src="/cli-journeys/keyframes/plan-help/frame-001.png"
      caption="plan --help usage line"
      size="medium" align="right"
      :annotations='[{"bbox":[40,60,520,20],"label":"Usage line"}]' />

Scroll past the usage line and you'll see the attention-kind override. hwLedger
auto-detects MLA for DeepSeek models, so for this walkthrough you won't pass
it — but it's good to know it exists.

<Shot src="/cli-journeys/keyframes/plan-help/frame-005.png"
      caption="--attention-kind flag documented mid-help"
      size="medium" align="left"
      :annotations='[{"bbox":[60,260,400,20],"label":"--attention-kind"}]' />

Near the bottom, the planner lists JSON output and `--hf` for HuggingFace IDs.

<Shot src="/cli-journeys/keyframes/plan-help/frame-009.png"
      caption="--json and --hf flags near the tail"
      size="small" align="right" />

---

## Step 3 — Probe the GPU

Before planning, know what you're planning *for*. `hwledger probe list` walks
the CUDA runtime and prints every device with its VRAM headroom.

<Shot src="/cli-journeys/keyframes/probe-list/frame-001.png"
      caption="probe list — detects CUDA driver"
      size="medium" align="right" />

<Shot src="/cli-journeys/keyframes/probe-list/frame-002.png"
      caption="CUDA device 0 printed with total and free VRAM"
      size="medium" align="left"
      :annotations='[{"bbox":[40,120,480,20],"label":"Device 0 row","color":"#cba6f7"}]' />

If you want continuous updates (e.g. to watch VRAM headroom while another job
runs), use `probe watch`.

<Shot src="/cli-journeys/keyframes/probe-watch/frame-001.png"
      caption="probe watch — live-refresh header"
      size="small" align="right"
      :annotations='[{"bbox":[60,80,400,20],"label":"Refresh header"}]' />

---

## Step 4 — Run the first plan

Now run the plan. Pass the model fixture path, context length, and batch size.

<Shot src="/cli-journeys/keyframes/plan-deepseek/frame-001.png"
      caption="Command line entered: plan deepseek-v3.json --context 2048 --batch 2"
      size="medium" align="right" />

The planner immediately identifies DeepSeek-V2/V3 from the config and picks
the MLA code path.

<Shot src="/cli-journeys/keyframes/plan-deepseek/frame-002.png"
      caption="Architecture detected: MLA"
      size="medium" align="left"
      :annotations='[{"bbox":[80,120,320,24],"label":"Model: DeepSeek-V2","style":"dashed"}]' />

The breakdown shows per-component VRAM: model weights → KV cache → activations.
Note the MLA row explicitly prints `kv_lora_rank=512` — this is the knob that
collapses the KV cache by ~16×.

<Shot src="/cli-journeys/keyframes/plan-deepseek/frame-003.png"
      caption="MLA with kv_lora_rank=512; VRAM breakdown per component"
      size="large" align="center"
      :annotations='[{"bbox":[120,340,220,28],"label":"MLA (kv_lora_rank=512)","color":"#89b4fa","note":"This is the row that determines KV cache sizing."}]' />

---

## Step 5 — Interpret the totals

The final block sums to ~363 GB for this config. hwLedger then recommends
tensor-parallel degree (TP=4) for 80 GB A100s.

<!-- SHOT-TODO: capture summary frame with TP recommendation highlighted -->

If totals exceed your probed headroom, re-run with smaller `--batch` or lower
`--context`. The planner refuses to produce a misleading "fits" answer when
`kv_lora_rank` is absent from the config — see [MLA](/math/mla) for why.

---

## Step 6 — MLA sweep (optional)

For longer-running capacity planning, the `plan-mla-deepseek` sweep runs the
planner across context lengths 2K → 128K and emits a CSV plus per-step frames.

<Shot src="/cli-journeys/keyframes/plan-mla-deepseek/frame-001.png"
      caption="Sweep start: context=2048"
      size="small" align="right" />

<Shot src="/cli-journeys/keyframes/plan-mla-deepseek/frame-002.png"
      caption="Per-layer KV cache column"
      size="medium" align="left"
      :annotations='[{"bbox":[140,200,260,32],"label":"Per-layer KV (bytes)","color":"#a6e3a1"}]' />

<Shot src="/cli-journeys/keyframes/plan-mla-deepseek/frame-003.png"
      caption="Sweep midpoint at 32K context"
      size="small" align="right" />

<Shot src="/cli-journeys/keyframes/plan-mla-deepseek/frame-004.png"
      caption="Final sweep summary with cross-over point"
      size="medium" align="left" />

---

## Step 7 — Register to the fleet

Once one host is verified, register it. `hwledger fleet register` signs the
capability report and commits it to the ledger.

<Shot src="/cli-journeys/keyframes/fleet-register/frame-001.png"
      caption="fleet register invocation"
      size="medium" align="right" />

<Shot src="/cli-journeys/keyframes/fleet-register/frame-003.png"
      caption="Host entry added"
      size="small" align="left"
      :annotations='[{"bbox":[80,160,360,24],"label":"host added","color":"#a6e3a1"}]' />

<Shot src="/cli-journeys/keyframes/fleet-register/frame-007.png"
      caption="Signature attached"
      size="small" align="right" />

---

## Step 8 — Audit the fleet

`fleet audit` walks every registered host, verifies attestations, and prints a
single-line verdict.

<Shot src="/cli-journeys/keyframes/fleet-audit/frame-002.png"
      caption="Attestation hash matched"
      size="medium" align="right"
      :annotations='[{"bbox":[60,200,520,28],"label":"attestation hash","color":"#f38ba8"}]' />

<Shot src="/cli-journeys/keyframes/fleet-audit/frame-005.png"
      caption="Audit complete — summary line"
      size="small" align="left" />

---

## Step 9 — Traceability report

Close the loop by running the traceability report. This cross-checks every
Functional Requirement against its tests and any registered fleet plan.

<Shot src="/cli-journeys/keyframes/traceability-report/frame-001.png"
      caption="Total FR coverage headline"
      size="medium" align="right"
      :annotations='[{"bbox":[40,40,560,24],"label":"FR coverage"}]' />

<Shot src="/cli-journeys/keyframes/traceability-report/frame-003.png"
      caption="Per-crate coverage table"
      size="small" align="left" />

---

## Step 10 — You're done

That's a complete DeepSeek-V3 planning loop: install → probe → plan → sweep →
register → audit → trace. Every step above was a real keyframe from a recorded
journey; open any screenshot in the lightbox to zoom.

For comparison against the alternative attention kinds, see:

- [MLA math](/math/mla)
- [GQA math](/math/gqa)
- [MHA math](/math/mha)

For the dry recipe without pictures, see [Quickstart](/getting-started/quickstart).
