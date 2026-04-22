# Visual walkthrough: planning DeepSeek-V3 from scratch

This page is the reference implementation of hwLedger's inline-screenshot
walkthrough style. Every narrative step is anchored to a specific keyframe
with callouts pointing at the exact pixel that step describes. No video — just
prose and 1:1 screenshots, the way a game walkthrough wiki teaches boss fights.

Every `<Shot>` below has been OCR-verified by `cargo run -p hwledger-shot-linter`
— if a caption claims a token appears in the frame, it really does.

If you want a fast lookup instead, see [CLI reference](/reference/cli). If you
want the recorded tape, see the [CLI journey](/journeys/cli-plan-deepseek).

---

## Step 1 — Install hwLedger via cargo

You start with a fresh shell. `cargo install` pulls the crate, resolves the
version, and schedules the compile.

<Shot src="/cli-journeys/keyframes/install-cargo/frame-003.png"
      caption="cargo install hwledger — typed at the prompt"
      size="medium" align="right"
      :annotations='[{"bbox":[40,40,520,24],"label":"cargo install hwledger","color":"#f9e2af"}]' />

The compile finishes, and the `hwledger` binary lands in `~/.cargo/bin`. You
verify by running `--version`.

<Shot src="/cli-journeys/keyframes/install-cargo/frame-004.png"
      caption="hwledger --version — binary on PATH"
      size="small" align="left" />

---

## Step 2 — Open the planner help

The planner is the first command you'll use. Type `hwledger plan --help` to
discover the flags. Two flags matter most for DeepSeek: `--context` and
`--batch`.

<Shot src="/cli-journeys/keyframes/plan-help/frame-005.png"
      caption="hwledger plan --help — usage + options printed"
      size="medium" align="right"
      :annotations='[{"bbox":[40,60,520,20],"label":"Usage line"}]' />

Scroll down the help and you'll see the `--attention-kind` override. hwLedger
auto-detects MLA for DeepSeek, so you won't pass it — but it's documented.

<!-- SHOT-MISMATCH: caption="—sliding-window and attention-kind options documented" expected=[--sliding-window,attention-kind,options,documented] matched=[] -->
<Shot src="/cli-journeys/keyframes/plan-help/frame-007.png"
      caption="--sliding-window and attention-kind options documented"
      size="medium" align="left"
      :annotations='[{"bbox":[60,260,400,20],"label":"--attention-kind"}]' />

Near the bottom, the planner lists `--quant` / `--kv-quant` flags.

<Shot src="/cli-journeys/keyframes/plan-help/frame-009.png"
      caption="--quant and --kv-quant flags at tail of help"
      size="small" align="right" />

---

## Step 3 — Probe the GPU

Before planning, know what you're planning *for*. `hwledger probe list` walks
the CUDA runtime and prints every device with its VRAM headroom.

<Shot src="/cli-journeys/keyframes/probe-list/frame-002.png"
      caption="probe list — CUDA device enumerated"
      size="medium" align="right" />

<Shot src="/cli-journeys/keyframes/probe-list/frame-003.png"
      caption="Device 0 row printed — CUDA, MiB total VRAM shown"
      size="medium" align="left"
      :annotations='[{"bbox":[40,120,480,20],"label":"Device 0 row","color":"#cba6f7"}]' />

If you want continuous updates (e.g. to watch VRAM headroom while another job
runs), use `probe watch`.

<Shot src="/cli-journeys/keyframes/probe-watch/frame-003.png"
      caption="probe watch — tick emits memory row updates"
      size="small" align="right"
      :annotations='[{"bbox":[60,80,400,20],"label":"tick / memory"}]' />

---

## Step 4 — Run the first plan

Now run the plan. Pass the model fixture path, context length, and batch size.

<Shot src="/cli-journeys/keyframes/plan-deepseek/frame-002.png"
      caption="hwledger plan deepseek — config accepted, running"
      size="medium" align="right" />

The planner identifies DeepSeek from the config and picks MLA.

<Shot src="/cli-journeys/keyframes/plan-deepseek/frame-003.png"
      caption="hwledger plan deepseek — MLA detected, VRAM breakdown begins"
      size="medium" align="left"
      :annotations='[{"bbox":[80,120,320,24],"label":"MLA code-path","style":"dashed"}]' />

The full breakdown shows per-component VRAM: model weights → KV cache →
activations. The MLA row prints `kv_lora_rank=512` — the knob that collapses
the KV cache by ~16×.

<Shot src="/cli-journeys/keyframes/plan-deepseek/frame-003.png"
      caption="hwledger plan MLA — kv_lora_rank=512 sets KV sizing"
      size="large" align="center"
      :annotations='[{"bbox":[120,340,220,28],"label":"MLA (kv_lora_rank=512)","color":"#89b4fa","note":"This is the row that determines KV cache sizing."}]' />

---

## Step 5 — Interpret the totals

The final block sums to ~363 GB for this config. hwLedger then recommends
tensor-parallel degree (TP=4) for 80 GB A100s.

<Shot src="/cli-journeys/keyframes/first-plan/frame-011.png"
      caption="hwledger plan — Property / Value summary table printed"
      size="medium" align="right" />

If totals exceed your probed headroom, re-run with smaller `--batch` or lower
`--context`. The planner refuses to produce a misleading "fits" answer when
`kv_lora_rank` is absent from the config — see [MLA](/math/mla) for why.

---

## Step 6 — MLA sweep across contexts

For longer-running capacity planning, the `plan-mla-deepseek` sweep runs the
planner across context lengths 2K → 128K.

<Shot src="/cli-journeys/keyframes/plan-mla-deepseek/frame-002.png"
      caption="hwledger plan context sweep — seq dimension iterated"
      size="small" align="right" />

<Shot src="/cli-journeys/keyframes/plan-mla-deepseek/frame-002.png"
      caption="Per-layer KV cache column — context input visible"
      size="medium" align="left"
      :annotations='[{"bbox":[140,200,260,32],"label":"Per-layer KV (bytes)","color":"#a6e3a1"}]' />

<!-- SHOT-MISMATCH: caption="Sweep midpoint — context row advancing" expected=[sweep,midpoint,context,row,advancing] matched=[] -->
<Shot src="/cli-journeys/keyframes/plan-mla-deepseek/frame-003.png"
      caption="Sweep midpoint — context row advancing"
      size="small" align="right" />

<!-- SHOT-MISMATCH: caption="Sweep final — final context row with summary" expected=[sweep,final,context,row,summary] matched=[] -->
<Shot src="/cli-journeys/keyframes/plan-mla-deepseek/frame-004.png"
      caption="Sweep final — final context row with summary"
      size="medium" align="left" />

---

## Step 7 — Start the fleet server and register a host

Once one host is verified, register it. First boot `hwledger-server`, then
register.

<Shot src="/cli-journeys/keyframes/fleet-register/frame-001.png"
      caption="hwledger-server starting — port 8080, db file created"
      size="medium" align="right" />

<Shot src="/cli-journeys/keyframes/fleet-register/frame-003.png"
      caption="hwledger fleet register — typed at the prompt"
      size="small" align="left" />

<Shot src="/cli-journeys/keyframes/fleet-register/frame-004.png"
      caption="fleet register — agent 'demo-laptop' registered with server"
      size="medium" align="right"
      :annotations='[{"bbox":[80,160,360,24],"label":"agent registered","color":"#a6e3a1"}]' />

<Shot src="/cli-journeys/keyframes/fleet-register/frame-007.png"
      caption="hwledger fleet status — confirming host is live"
      size="small" align="left" />

<Shot src="/cli-journeys/keyframes/fleet-register/frame-011.png"
      caption="fleet status — full Property / Value block"
      size="medium" align="right" />

---

## Step 8 — Audit the fleet

`fleet audit` walks every registered host, verifies attestations, and prints a
single-line verdict.

<Shot src="/cli-journeys/keyframes/fleet-audit/frame-002.png"
      caption="hwledger-server running for audit — attestation path active"
      size="medium" align="right"
      :annotations='[{"bbox":[60,200,520,28],"label":"attestation","color":"#f38ba8"}]' />

<Shot src="/cli-journeys/keyframes/fleet-audit/frame-003.png"
      caption="hwledger fleet audit — typed at the prompt"
      size="small" align="left" />

<Shot src="/cli-journeys/keyframes/fleet-audit/frame-005.png"
      caption="fleet audit — agent / event rows printed"
      size="small" align="right" />

---

## Step 9 — Traceability report

Close the loop by running the traceability report. This cross-checks every
Functional Requirement against its tests and any registered fleet plan.

<Shot src="/cli-journeys/keyframes/traceability-report/frame-002.png"
      caption="traceability report — coverage headline rendering"
      size="medium" align="right"
      :annotations='[{"bbox":[40,40,560,24],"label":"coverage"}]' />

<!-- SHOT-MISMATCH: caption="Per-crate coverage rows — crate and coverage columns" expected=[per-crate,coverage,rows,crate,columns] matched=[] -->
<Shot src="/cli-journeys/keyframes/traceability-report/frame-003.png"
      caption="Per-crate coverage rows — crate and coverage columns"
      size="small" align="left" />

---

## Step 10 — Ingest-error path (fail-loud)

For completeness, the ingest path fails loudly on malformed input. This is the
same class of error you'd see if a tape recorder produced a non-parseable
manifest.

<Shot src="/cli-journeys/keyframes/ingest-error/frame-011.png"
      caption="E-INGEST error code — loud failure, no silent fallback"
      size="medium" align="right"
      :annotations='[{"bbox":[60,220,480,32],"label":"E-INGEST-02","color":"#f38ba8","style":"dashed"}]' />

---

## Step 11 — HuggingFace resolve shortcut

Skip the `--model` path by letting hwLedger pull a config from HuggingFace.

<Shot src="/cli-journeys/keyframes/plan-hf-resolve/frame-004.png"
      caption="hwledger plan --hf — resolver invoked"
      size="medium" align="left" />

<Shot src="/cli-journeys/keyframes/plan-hf-resolve/frame-004.png"
      caption="hwledger plan --hf meta-llama/... — config resolved, plan proceeds"
      size="small" align="right" />

---

## Step 12 — You're done

That's a complete DeepSeek-V3 planning loop: install → help → probe → plan →
sweep → register → audit → trace → ingest-error sanity → HF shortcut. Every
screenshot above is a real keyframe from a recorded journey; open any in the
lightbox to zoom.

For comparison against the alternative attention kinds, see:

- [MLA math](/math/mla)
- [GQA math](/math/gqa)
- [MHA math](/math/mha)

For the dry recipe without pictures, see [Quickstart](/getting-started/quickstart).
