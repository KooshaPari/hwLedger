# Visual walkthrough: planning DeepSeek-V3 from scratch

This page is the reference implementation of hwLedger's inline-screenshot
walkthrough style. Every narrative step is anchored to a specific keyframe
with callouts pointing at the exact pixel that step describes. No video — just
prose and 1:1 screenshots, the way a game walkthrough wiki teaches boss fights.

Every `<ShotGallery>` below has been OCR-verified by `cargo run -p hwledger-shot-linter`
— if a caption claims a token appears in the frame, it really does. Steps with
known OCR misses are marked inline.

If you want a fast lookup instead, see [CLI reference](/reference/cli). If you
want the recorded tape, see the [CLI journey](/journeys/cli-plan-deepseek).

---

## Step 1 — Install hwLedger via cargo

You start with a fresh shell. `cargo install` pulls the crate, resolves the
version, and schedules the compile. The binary lands in `~/.cargo/bin`; verify
it with `--version`.

<ShotGallery
  title="Step 1 — cargo install + version verify"
  :shots='[
    {"src":"/cli-journeys/keyframes/install-cargo/frame-003.png","caption":"cargo install hwledger — typed at the prompt"},
    {"src":"/cli-journeys/keyframes/install-cargo/frame-004.png","caption":"hwledger --version — binary on PATH"}
  ]' />

---

## Step 2 — Open the planner help

The planner is the first command you'll use. Type `hwledger plan --help` to
discover the flags. Two flags matter most for DeepSeek: `--context` and
`--batch`. Scroll down and you'll see the `--attention-kind` override (hwLedger
auto-detects MLA for DeepSeek, so you won't pass it — but it's documented).
Near the bottom, the planner lists `--quant` / `--kv-quant` flags.

> **OCR note:** one frame in this step (`plan-help/frame-007.png`) failed
> automated OCR verification; the caption describes `--sliding-window` +
> `--attention-kind`, which appear visibly in the frame but were not matched by
> the linter.

<ShotGallery
  title="Step 2 — planner help output"
  :shots='[
    {"src":"/cli-journeys/keyframes/plan-help/frame-005.png","caption":"hwledger plan --help — usage + options printed"},
    {"src":"/cli-journeys/keyframes/plan-help/frame-007.png","caption":"--sliding-window and attention-kind options documented"},
    {"src":"/cli-journeys/keyframes/plan-help/frame-009.png","caption":"--quant and --kv-quant flags at tail of help"}
  ]' />

---

## Step 3 — Probe the GPU

Before planning, know what you're planning *for*. `hwledger probe list` walks
the CUDA runtime and prints every device with its VRAM headroom. If you want
continuous updates (e.g. to watch VRAM headroom while another job runs), use
`probe watch`.

<ShotGallery
  title="Step 3 — probe list + probe watch"
  :shots='[
    {"src":"/cli-journeys/keyframes/probe-list/frame-002.png","caption":"probe list — CUDA device enumerated"},
    {"src":"/cli-journeys/keyframes/probe-list/frame-003.png","caption":"Device 0 row printed — CUDA, MiB total VRAM shown"},
    {"src":"/cli-journeys/keyframes/probe-watch/frame-003.png","caption":"probe watch — tick emits memory row updates"}
  ]' />

---

## Step 4 — Run the first plan

Now run the plan. Pass the model fixture path, context length, and batch size.
The planner identifies DeepSeek from the config and picks MLA. The full
breakdown shows per-component VRAM (model weights → KV cache → activations).
The MLA row prints `kv_lora_rank=512` — the knob that collapses the KV cache
by ~16×.

<ShotGallery
  title="Step 4 — plan deepseek (MLA detected, VRAM breakdown)"
  :shots='[
    {"src":"/cli-journeys/keyframes/plan-deepseek/frame-002.png","caption":"hwledger plan deepseek — config accepted, running"},
    {"src":"/cli-journeys/keyframes/plan-deepseek/frame-003.png","caption":"MLA detected, VRAM breakdown begins"},
    {"src":"/cli-journeys/keyframes/plan-deepseek/frame-003.png","caption":"MLA row — kv_lora_rank=512 sets KV sizing"}
  ]' />

---

## Step 5 — Interpret the totals

The final block sums to ~363 GB for this config. hwLedger then recommends
tensor-parallel degree (TP=4) for 80 GB A100s. If totals exceed your probed
headroom, re-run with smaller `--batch` or lower `--context`. The planner
refuses to produce a misleading "fits" answer when `kv_lora_rank` is absent
from the config — see [MLA](/math/mla) for why.

<ShotGallery
  title="Step 5 — summary totals"
  :shots='[
    {"src":"/cli-journeys/keyframes/first-plan/frame-011.png","caption":"hwledger plan — Property / Value summary table printed"}
  ]' />

---

## Step 6 — MLA sweep across contexts

For longer-running capacity planning, the `plan-mla-deepseek` sweep runs the
planner across context lengths 2K → 128K.

> **OCR note:** two sweep frames (`plan-mla-deepseek/frame-003.png` and
> `frame-004.png`) failed automated OCR; captions describe the midpoint and
> final context rows — both are visible in the frames.

<ShotGallery
  title="Step 6 — MLA context sweep"
  :shots='[
    {"src":"/cli-journeys/keyframes/plan-mla-deepseek/frame-002.png","caption":"hwledger plan context sweep — seq dimension iterated"},
    {"src":"/cli-journeys/keyframes/plan-mla-deepseek/frame-002.png","caption":"Per-layer KV cache column — context input visible"},
    {"src":"/cli-journeys/keyframes/plan-mla-deepseek/frame-003.png","caption":"Sweep midpoint — context row advancing"},
    {"src":"/cli-journeys/keyframes/plan-mla-deepseek/frame-004.png","caption":"Sweep final — final context row with summary"}
  ]' />

---

## Step 7 — Start the fleet server and register a host

Once one host is verified, register it. First boot `hwledger-server`, then
register.

<ShotGallery
  title="Step 7 — fleet server boot + host register"
  :shots='[
    {"src":"/cli-journeys/keyframes/fleet-register/frame-001.png","caption":"hwledger-server starting — port 8080, db file created"},
    {"src":"/cli-journeys/keyframes/fleet-register/frame-003.png","caption":"hwledger fleet register — typed at the prompt"},
    {"src":"/cli-journeys/keyframes/fleet-register/frame-004.png","caption":"fleet register — agent demo-laptop registered with server"},
    {"src":"/cli-journeys/keyframes/fleet-register/frame-007.png","caption":"hwledger fleet status — confirming host is live"},
    {"src":"/cli-journeys/keyframes/fleet-register/frame-011.png","caption":"fleet status — full Property / Value block"}
  ]' />

---

## Step 8 — Audit the fleet

`fleet audit` walks every registered host, verifies attestations, and prints a
single-line verdict.

<ShotGallery
  title="Step 8 — fleet audit"
  :shots='[
    {"src":"/cli-journeys/keyframes/fleet-audit/frame-002.png","caption":"hwledger-server running for audit — attestation path active"},
    {"src":"/cli-journeys/keyframes/fleet-audit/frame-003.png","caption":"hwledger fleet audit — typed at the prompt"},
    {"src":"/cli-journeys/keyframes/fleet-audit/frame-005.png","caption":"fleet audit — agent / event rows printed"}
  ]' />

---

## Step 9 — Traceability report

Close the loop by running the traceability report. This cross-checks every
Functional Requirement against its tests and any registered fleet plan.

> **OCR note:** `traceability-report/frame-003.png` failed automated OCR; the
> per-crate coverage rows are visible in the frame.

<ShotGallery
  title="Step 9 — traceability report"
  :shots='[
    {"src":"/cli-journeys/keyframes/traceability-report/frame-002.png","caption":"traceability report — coverage headline rendering"},
    {"src":"/cli-journeys/keyframes/traceability-report/frame-003.png","caption":"Per-crate coverage rows — crate and coverage columns"}
  ]' />

---

## Step 10 — Ingest-error path (fail-loud)

For completeness, the ingest path fails loudly on malformed input. This is the
same class of error you'd see if a tape recorder produced a non-parseable
manifest.

<ShotGallery
  title="Step 10 — ingest error (loud)"
  :shots='[
    {"src":"/cli-journeys/keyframes/ingest-error/frame-011.png","caption":"E-INGEST error code — loud failure, no silent fallback"}
  ]' />

---

## Step 11 — HuggingFace resolve shortcut

Skip the `--model` path by letting hwLedger pull a config from HuggingFace.

<ShotGallery
  title="Step 11 — hf resolve"
  :shots='[
    {"src":"/cli-journeys/keyframes/plan-hf-resolve/frame-004.png","caption":"hwledger plan --hf — resolver invoked"},
    {"src":"/cli-journeys/keyframes/plan-hf-resolve/frame-004.png","caption":"hwledger plan --hf meta-llama/... — config resolved, plan proceeds"}
  ]' />

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
