# User-Journey Verification Harness for SwiftUI macOS Apps + CLI Tools: 2026 Research Brief

## Executive Summary

This brief surveys the 2026 state of the art for building an automated user-journey verification harness that enables agents (coding and VLM-capable review agents) to verify real user-facing behavior without manually running apps. The harness must drive a native SwiftUI macOS app, record CLI output, generate GIF/MP4 artifacts, and leverage VLM (Vision Language Model) agents to audit visual equivalence against intent labels.

**Key finding**: Claude Opus 4.7 (April 2026) accepts static images up to 2576px (3.75MP) in JPEG/PNG/GIF format, but animated GIFs are **not supported**—only the first frame is processed. This mandates a **keyframe extraction + gallery layout** approach rather than sending video directly. The stack is mature, with native macOS APIs (ScreenCaptureKit, XCUITest, Accessibility API) providing reliable automation and recording; the primary implementation work is orchestration and VLM integration.

---

## Part 1: Stack Recommendation

| Requirement | 2026 Recommendation | Rationale |
|---|---|---|
| **UI Automation (SwiftUI macOS)** | **XCUITest** (primary) + **Accessibility API** (fallback) | XCUITest is Apple's native framework—zero external dependencies, stable, integrated into Xcode. Accessibility API via Swift wrappers (e.g., AXorcist) handles complex selectors and out-of-process automation. Appium Mac2 Driver adds overhead without significant value for proprietary integration. |
| **CLI Recording** | **VHS (Charmbracelet)** for declarative .tape files → MP4/GIF | Mature, widely adopted in 2026. Supports custom keybindings, deterministic timing, MP4 output. Alternative: `ScreenCaptureKit` for windowed TUI capture (if VHS is insufficient). |
| **Screen Recording (Windowed)** | **ScreenCaptureKit** (native Swift) + **AVAssetWriter** | Apple's recommended API (macOS 12.3+). High-fidelity, hardware-accelerated H.264/HEVC encoding. Replacement for deprecated AVCaptureScreenInput. |
| **Video → Keyframes** | **ffmpeg** with `palettegen` + `paletteuse` for GIF; keyframe extraction via `-vf select='eq(pict_type,I)'` | Battle-tested, mature, part of POSIX toolchain. Palette generation critical for GIF quality (custom 256-color palette per video prevents color banding). |
| **VLM + Blackbox Verification** | **Claude API** (Opus 4.7) + **keyframe gallery** layout (not video) | Claude 4.x is the agent engine. Since animated GIFs are not processed, send ~10–20 extracted keyframes as a temporal sequence. LLM-judge model (Sonnet 4.6) for equivalence scoring. |
| **Docs + Embedding** | **VitePress 2 (2026)** with Vue 3 components for galleries + native `<video>`/`<img>` tags | VitePress has no built-in MDX; use Vue components (.vue files) for rich embeds. Supports MP4 via standard HTML5 `<video>`. GIFs via `<img>`. |

---

## Part 2: Concrete WP Implementation Plan

### WP 25: XCUITest + Accessibility Framework Integration

**Goal**: Drive SwiftUI app actions, capture screenshots, navigate UI state programmatically.

1. Create `Tests/UITestHarness.swift` test bundle (linked to SwiftUI app target)
2. Define `AppDriver` class wrapping XCUIApplication, exposing:
   - `navigate(_ path: [String]) -> XCUIElement` — traverse hierarchy by label/identifier
   - `tapButton(_ label: String)` — atomically find + tap button by accessibility label
   - `typeText(_ text: String) -> Void` — simulate keyboard input
   - `screenshot() -> CGImage` — capture current state via XCUIDevice
3. Fallback: Use AXorcist (Swift wrapper around Accessibility API) for selectors XCUITest cannot find
4. Add inline tests verifying each driver method; trace to FR-UI-* functional requirements
5. **Acceptance**: `cargo test --test ui_harness -- --nocapture` passes; screenshot array produced

### WP 26: ScreenCaptureKit-Based Recording Pipeline

**Goal**: Record app interaction as MP4 with sub-frame capture; produce artifact suitable for ffmpeg processing.

1. Create `apps/macos/HwLedger/Recorder/ScreenRecorder.swift` (Swift 6, SwiftUI integration)
2. Implement `ScreenRecorder` class:
   - `startRecording(outputPath: URL) async throws` — initialize SCContentFilter + SCStream
   - `captureFrame() async -> CMSampleBuffer` — pull frame from stream
   - `stopRecording() async -> URL` — finalize AVAssetWriter, return file path
3. Use `AVAssetWriter` with H.264 encoder (hardware-accelerated on Apple Silicon)
4. Wire into test harness: before each user-journey test, spawn recorder; after, finalize MP4
5. **Acceptance**: `test_record_and_playback()` produces 30fps MP4 (>2MB, seekable, plays in QuickTime)

### WP 27: VHS Tape Scripts + CLI Recording Harness

**Goal**: Generate .tape files declaratively; execute via vhs CLI; output MP4 + GIF.

1. Create `docs/recordings/tape-templates/` directory
2. Implement `TapeGenerator` (Rust or Python) that:
   - Takes journey TOML/YAML spec (actions: [Type, Sleep, Wait, KeyPress], assertions: [output contains])
   - Emits `.tape` file with VHS syntax
   - Example: `Type "cargo build"` → `Type "cargo build"`, `Sleep 2s` → wait for "Finished"
3. Shell wrapper `./record-journey.sh <journey-id>` that:
   - Calls `vhs run docs/recordings/<journey-id>.tape`
   - Outputs: `out/<journey-id>.mp4`, `out/<journey-id>.gif`
4. VHS configuration: 1400x800 terminal, 16pt font, Catppuccin theme, 30fps
5. **Acceptance**: `./record-journey.sh demo-build` produces `out/demo-build.{mp4,gif}` in <10 seconds

### WP 28: FFmpeg Keyframe Extraction + Palette Optimization

**Goal**: Extract I-frames from MP4; generate optimized GIF; produce PNG keyframe gallery.

1. Create `tools/ffmpeg-pipeline/` Rust crate:
   - `extract_keyframes(mp4_path: &Path) -> Vec<PathBuf>` — run `ffmpeg -vf select='eq(pict_type,I)',showinfo -vsync vfr`
   - `optimize_gif(mp4_path: &Path, output: &Path) -> Result<()>` — two-pass:
     ```bash
     ffmpeg -ss 0 -i input.mp4 \
       -filter_complex "fps=10,scale=360:-1[s]; [s]split[a][b]; [a]palettegen[pal]; [b][pal]paletteuse" \
       output.gif
     ```
   - `generate_gallery(keyframes: Vec<PathBuf>, output_dir: &Path)` — copy + rename PNGs
2. Integrate into CI/recording pipeline: after VHS produces MP4, auto-call this crate
3. Store keyframes in `docs/recordings/<journey-id>/frames/` (numbered PNG files)
4. **Acceptance**: `cargo test test_ffmpeg_pipeline` extracts 8–12 keyframes from 30-second MP4; GIF is <2MB, <5% quality loss

### WP 29: Keyframe Gallery + VLM Blackbox Verification

**Goal**: For each user journey, spawn a fresh Claude agent to describe the keyframe sequence; compare against intent label using LLM-judge.

1. Create `tools/vlm-verifier/` Rust/Python crate:
   - `load_keyframes(journey_id: &str) -> Vec<Image>` — read PNG gallery from WP28 output
   - `generate_intent_label(journey_id: &str) -> IntentLabel` — load from YAML:
     ```yaml
     journey_id: "build-and-test"
     actions:
       - action: "run cargo build"
         precondition: "Terminal shows prompt"
         expected_visible_change: "Compilation output scrolls; progress bar appears"
     ```
   - `invoke_claude_vision(images: Vec<Image>, system_prompt: &str) -> String` — send keyframes + gallery layout to Claude Opus 4.7
   - `invoke_lvm_judge(intent: &str, vlm_response: &str) -> EquivalenceScore` — ask Claude Sonnet 4.6: "Are these descriptions equivalent? (1-5 scale)"
2. System prompt for VLM: *"You are viewing a sequence of keyframes from a terminal/app interaction. Describe in 2–3 sentences what happens at each keyframe: what command was typed, what output appeared, what changed visually."*
3. Intent label schema: `action`, `precondition`, `expected_visible_change`, `actual_visible_change` (populated by VLM)
4. Store results in `docs/recordings/<journey-id>/verification.json`:
   ```json
   {
     "journey_id": "build-and-test",
     "vlm_description": "User types 'cargo build'; compilation output scrolls...",
     "intent_label": { "action": "run cargo build", ... },
     "equivalence_score": 4,
     "status": "PASS"
   }
   ```
5. **Acceptance**: `./verify-journey.sh build-and-test` completes in <15 seconds; verification.json populated with score and status

### WP 30: VitePress Documentation Integration + Auto-Sidebar

**Goal**: Embed recordings, keyframe galleries, and verification status in VitePress docs; auto-generate sidebar from journey catalog.

1. Create `docs/.vitepress/theme/components/JourneyViewer.vue` (Vue 3 SFC):
   - Props: `journeyId: string`
   - Render:
     - `<video src="...mp4" controls />` (MP4 playback)
     - `<img src="...gif" alt="..." />` (GIF fallback)
     - Keyframe gallery: horizontal scroll of PNGs with timestamps
     - Verification badge: "PASS" (green) / "FAIL" (red) from verification.json
     - Intent vs. VLM description in side-by-side Markdown blocks
2. Create `docs/journeys/index.md` template:
   ```markdown
   <JourneyViewer journey-id="build-and-test" />
   ```
3. Implement `sidebar-auto-journeys.ts` generator:
   - Scan `docs/recordings/*/metadata.yaml` (journey catalog)
   - For each journey, emit sidebar entry:
     ```ts
     { text: "Build and Test", link: "/journeys/build-and-test" }
     ```
   - Embed into `.vitepress/config.ts` `themeConfig.sidebar`
4. Add GitHub Pages CI step: post-recording, run VLM verification, commit verification.json, rebuild docs
5. **Acceptance**: `bun run docs:build` produces HTML with embedded MP4s, GIF fallbacks, keyframe galleries, verification badges; sidebar auto-populated; deploy to GitHub Pages

---

## Part 3: VLM + Video Caveat & Keyframe Strategy

### Claude Opus 4.7 Image Limitations (April 2026)

**Critical Constraint**: Animated GIFs are **NOT supported**. Only the first frame is processed.

- **Supported formats**: JPEG, PNG, GIF (static only), WebP
- **Max dimensions**: 2576 x 2576 px (3.75 MP) — up from 1568 px in prior version
- **Max images per request**: 100 (API, 200k-token models); 600 (API, all others)
- **Request size limit**: 32 MB (standard endpoints)

**No native video support** across Claude, Sonnet, or Haiku as of April 2026. Alternative providers (Gemini 2.0+) ship native video, but integration complexity is high.

### Recommended Approach: Keyframe Gallery

Instead of sending one animated GIF, send **10–20 extracted keyframes** as a temporal sequence:

```python
# Pseudocode
keyframes = extract_keyframes("out/journey.mp4", stride=3)  # Every 3 seconds
images = [Image.open(kf) for kf in keyframes]

response = client.messages.create(
  model="claude-opus-4-7-20250416",
  max_tokens=1024,
  messages=[
    {
      "role": "user",
      "content": [
        {
          "type": "text",
          "text": "Describe what happens in this terminal session (images in order):"
        },
        *[{"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": b64(img)}} 
          for img in images]
      ]
    }
  ]
)
```

**Trade-offs**:
- **Pros**: Temporal narrative preserved; agent sees full interaction; ~10–20 images = ~120–180 tokens (vs. one GIF that gets ignored)
- **Cons**: Multiple images increase request size; slower per-request latency; manual timestamping required for reference

### FFmpeg Keyframe Extraction Command

```bash
# Extract all I-frames (keyframes) with timing information
ffmpeg -i input.mp4 -vf "select='eq(pict_type,I)',showinfo" -vsync vfr frame_%04d.png 2>&1 | grep "pts:" | awk '{print $5}' > timestamps.txt

# Or use `-vf fps=0.33` to extract 1 frame every 3 seconds
ffmpeg -i input.mp4 -vf fps=0.33 frame_%04d.png
```

**Quality**: For a 30-second, 1400×800 terminal recording at 30fps, extracting at fps=0.33 (1 frame every 3s) yields ~10 PNG files, ~100–200 KB each. Total upload: ~1–2 MB (well under 32 MB limit).

---

## Part 4: Token Cost Analysis

### Per-Journey Cost Estimate

**Scenario**: 30-second SwiftUI app demo + CLI build.

**Inputs**:
- 10 keyframes (extracted, 1400×800 PNG each)
- 1 intent label (YAML/JSON, ~200 tokens)
- System prompt for VLM ("describe what you see")

**Claude Opus 4.7 pricing** (April 2026): $3 / 1M input tokens, $15 / 1M output tokens

| Step | Model | Input Tokens | Output Tokens | Cost (USD) |
|---|---|---|---|---|
| 1. Keyframe description (VLM) | Opus 4.7 | ~2,500 (10 images + text) | ~500 (2–3 sentence description) | $0.0065 |
| 2. Equivalence judge (LLM) | Sonnet 4.6 | ~800 (intent + description) | ~100 (1–5 score) | ~$0.0003 |
| **Total per journey** | — | ~3,300 | ~600 | **~$0.007** |

**Cost per 100 journeys**: ~$0.70 (negligible)

**Note**: Pricing assumes Claude Opus 4.7 API rates (publicly announced April 2026). Actual costs depend on context-length (longer descriptions inflate token count). Keyframe extraction via ffmpeg is free (local compute).

---

## Part 5: Existing Phenotype Infrastructure Worth Reusing

### Repositories with Relevant Patterns

| Path | What's There | Reuse Opportunity |
|---|---|---|
| `/repos/KlipDot/demos/` | VHS `.tape` files (7 demos) | Tape generation template; reference CLI recording patterns |
| `/repos/heliosApp/docs/.vitepress/` | VitePress config + Vue components (CategorySwitcher, etc.) | Copy JourneyViewer.vue pattern; sidebar config structure |
| `/repos/heliosApp/docs/reports/worklog.md` | "VitePress configured, pages workflow exists" | CI workflow for docs deployment |
| `/repos/HexaKit/docs/.vitepress/theme/components/` | Reusable Vue 3 components (ModuleSwitcher, SidebarFilter) | Pattern for custom theme components |
| `/repos/RIP-Fitness-App/.archive/docs/demos/gifs/` | Pre-recorded GIFs organized by platform (web, mobile, social) | Directory structure for journey recordings |
| `/repos/agentapi-plusplus/docs/.vitepress/` | Production VitePress setup | Reference for CI integration, build config |
| `/repos/phenotype-config/docs/journeys/` | UserJourney.vue + FeatureDetail.vue component imports | MDX-like embedded component patterns in VitePress |

### Missing Components (Need to Build)

1. **XCUITest harness** — No existing Swift UI test framework in Phenotype (hwLedger is greenfield)
2. **ScreenCaptureKit wrapper** — Not found; needs new Swift module
3. **FFmpeg pipeline CLI** — Not found; needs Rust crate or Python script
4. **VLM verifier agent** — Not found; needs new tool (Rust or Python, Claude API integration)
5. **JourneyViewer.vue component** — Not found; derive from existing component patterns
6. **sidebar-auto-journeys.ts** — Not found; pattern exists (sidebar-auto.ts search was empty, but sidebar config structure is in heliosApp, HexaKit, agentapi-plusplus)

---

## Part 6: Known-Unknowns & R&D Gaps

### Genuinely Mature (Just Engineering)

- XCUITest fundamentals (screenshot capture, element navigation, tapping) — 15+ years stable
- ScreenCaptureKit (macOS 12.3+) — native Swift API, well-documented
- VHS tape language — mature, widely used in TUI projects (Bubble, Charmbracelet ecosystem)
- FFmpeg keyframe extraction + palette generation — standard Unix tools, command-line stable
- Claude API vision — stable as of April 2026 (Opus 4.7); no breaking changes expected
- VitePress 2 — production-ready; Vue 3 components work as expected

### Novel R&D (Needs Prototyping)

1. **Large-scale keyframe sequence interpretation**: Sending 20+ PNG frames to Claude — does it maintain temporal coherence across the sequence, or treat them as independent images? Prototype WP29 with 3–5 real journeys; measure coherence quality.

2. **Equivalence scoring variance**: Does Claude Sonnet 4.6 (as judge) consistently rank equivalent journeys the same way across multiple runs? Build regression test: same intent + VLM description → score variance. Target: ±0.5 on 1–5 scale.

3. **False negatives from VLM**: When a journey genuinely differs from intent (e.g., unexpected crash dialog), does the VLM notice without being prompted? Test: inject intentional UI regression, measure detection rate.

4. **Accessibility API fallback reliability**: When XCUITest cannot locate a SwiftUI element, does AXorcist consistently find it via Accessibility hierarchy? Prototype with 5–10 complex SwiftUI views; measure hit rate.

5. **Intent label minimalism**: What's the minimum spec for an intent label to avoid VLM hallucination? Current schema: `action`, `precondition`, `expected_visible_change`. Can we reduce further without losing signal? A/B test with live journeys.

6. **CI latency**: End-to-end (record + extract + verify) for 1 journey: target <20 seconds. Measure: CPU time (VHS, ffmpeg), I/O (upload to Claude), LLM latency (2 requests). Identify bottleneck.

---

## Part 7: Recommended Rollout Sequence

**Phase 1 (WP 25–26, 1–2 weeks)**:
- XCUITest harness for hwLedger app
- ScreenCaptureKit recording pipeline
- Manual smoke test: run test, capture screenshots, verify MP4 produced

**Phase 2 (WP 27–28, 1 week)**:
- VHS tape generation + ffmpeg pipeline
- CLI journey recording (build, test, deploy scenarios)
- Golden MP4 + GIF + keyframe gallery artifacts

**Phase 3 (WP 29–30, 2 weeks)**:
- Claude API integration (Opus 4.7 keyframe VLM + Sonnet 4.6 judge)
- VitePress JourneyViewer component + sidebar auto-generation
- E2E: record journey → extract frames → verify → embed in docs

**Phase 4 (Iterate, ongoing)**:
- Prototype R&D unknowns (temporal coherence, equivalence variance)
- Tune intent label schema
- Optimize CI latency (parallel ffmpeg, batch API calls)

---

## Part 8: Stack Command Reference

### Record a CLI Journey

```bash
vhs run docs/recordings/demo-build.tape --output out/demo-build.mp4
```

### Extract Keyframes + Optimize GIF

```bash
# Keyframes
ffmpeg -i out/demo-build.mp4 -vf fps=0.33 frames/frame_%04d.png

# GIF with custom palette
ffmpeg -i out/demo-build.mp4 \
  -filter_complex "fps=10,scale=360:-1[s]; [s]split[a][b]; [a]palettegen[pal]; [b][pal]paletteuse" \
  out/demo-build.gif
```

### Run VLM Verification

```bash
cargo run --bin vlm-verifier -- --journey-id demo-build --model opus-4-7
```

### Build VitePress Docs with Journeys

```bash
bun run docs:build  # Runs sidebar-auto-journeys.ts, embeds verification.json
```

---

## References

- [Anthropic Claude API Vision Docs](https://platform.claude.com/docs/en/build-with-claude/vision)
- [Apple XCUITest Documentation](https://developer.apple.com/documentation/xcuiautomation)
- [Apple ScreenCaptureKit Documentation](https://developer.apple.com/documentation/screencapturekit/)
- [Charmbracelet VHS GitHub](https://github.com/charmbracelet/vhs)
- [FFmpeg GIF/Keyframe Guide](https://cloudinary.com/guides/image-formats/ffmpeg-mp4-to-gif)
- [Visual Regression Testing in 2026](https://saucelabs.com/resources/blog/comparing-the-20-best-visual-testing-tools-of-2026)
- [macOS Accessibility API — AXorcist](https://github.com/steipete/AXorcist)
- [Appium Mac2 Driver](https://appium.github.io/appium.io/docs/en/drivers/mac2/)
- [VitePress 2 Documentation](https://vitepress.dev/)

---

**Research completed**: April 18, 2026  
**Status**: Ready for WP implementation planning  
**Next step**: Prioritize WP25–26 (XCUITest + ScreenCaptureKit) for hwLedger proof-of-concept
