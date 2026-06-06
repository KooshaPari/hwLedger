# Journey Manifests

Every user-facing flow documented in a repo MUST have a journey manifest
co-located in this directory (or an equivalent, e.g. `docs/journeys/manifests/`).
Manifests are the source of truth that ties a `ShotGallery` + `RecordingEmbed`
bundle back to a feature spec, automated test, and CI gate.

## Canonical Layout

```
docs/
  journeys/
    manifests/
      README.md                      # this file
      <journey-id>.journey.yaml      # one per documented flow
    <journey-id>.md                  # human-readable journey page
    cli-journeys/
      keyframes/<journey-id>/
        frame-###.png                # keyframe screenshots
      recordings/<journey-id>.gif    # full replay
```

## Manifest Template

Copy the block below into `<journey-id>.journey.yaml`. The shape mirrors
`phenotype_journey_core::Manifest` (see
`phenotype-journeys/crates/phenotype-journey-core/src/lib.rs`).

```yaml
id: <journey-id>                 # stable snake_case id; matches tape + keyframe dir
intent: <one-line user story>    # e.g. "Generate traceability markdown report"
keyframe_count: <int>            # number of keyframes bundled in the gallery
passed: false                    # set true after the verifier accepts the recording
recording: cli-journeys/recordings/<journey-id>.gif
recording_gif: cli-journeys/recordings/<journey-id>.gif
steps:
  - index: 1
    slug: <short-state-name>     # e.g. "help-screen" or "page-loaded"
    assertions:
      must_contain:
        - "<string OCR'd frame must include>"
      must_contain_regex:
        - "<regex tolerated by OCR mangling>"
      must_not_contain:
        - "error:"               # negative gate; CRT-style hard fail
      expected_exit: 0
      ocr_required: true
```

## Required Artifacts (per journey)

1. A journey page under `docs/journeys/<journey-id>.md` with:
   - a short narrative section
   - a `<ShotGallery>` block (one or more keyframes)
   - a `<RecordingEmbed tape="<journey-id>" kind="cli" />` block
2. A manifest under `docs/journeys/manifests/<journey-id>.journey.yaml`.
3. A link from the feature spec (FR/NFR) to the journey page.
4. A link from the journey page back to the feature spec, ADR, or worklog.
5. Stable asset names so other repos can re-embed the same evidence.

## CI Gate

`phenotype-journey verify` MUST pass on PRs that touch a manifest, a journey
page, or the code path the journey documents. The verify step:

1. Resolves every `ShotGallery` / `RecordingEmbed` reference in
   `docs/journeys/**/*.md`.
2. Confirms each referenced keyframe + recording exists in
   `docs/journeys/cli-journeys/...`.
3. Loads every `*.journey.yaml` and runs the assertion engine against
   the recorded OCR text.
4. Fails the build on any `must_contain` / `must_not_contain` /
   `expected_exit` violation.

## Adoption Checklist

When adding a new journey:

- [ ] Capture a recording (VHS tape, Playwright trace, or equivalent).
- [ ] Extract 3–5 keyframes that show the important state transitions.
- [ ] Drop keyframes in `docs/journeys/cli-journeys/keyframes/<id>/`.
- [ ] Save the recording in `docs/journeys/cli-journeys/recordings/`.
- [ ] Author `<id>.journey.yaml` with at least one `must_not_contain: ["error:"]`
      step and a final `expected_exit: 0` step.
- [ ] Author `<id>.md` with `<ShotGallery>` + `<RecordingEmbed>` blocks.
- [ ] Link the journey page from the feature FR/NFR doc.
- [ ] Confirm CI is green on a PR that touches the code path.

## Stub Pages

Until real recordings exist, the canonical stub is:

```md
<!--
STUB: rich journey embed pending.
Real evidence lives under docs/journeys/cli-journeys/{keyframes,recordings}/.
Replace this block with:

  <ShotGallery
    title="<journey-id>: <user intent>"
    :shots='[
      {"src":"/docs/journeys/cli-journeys/keyframes/<journey-id>/frame-001.png","caption":"<state 1>"},
      {"src":"/docs/journeys/cli-journeys/keyframes/<journey-id>/frame-002.png","caption":"<state 2>"}
    ]' />

  <RecordingEmbed tape="<journey-id>" kind="cli" caption="<one-line summary>" />
-->
```
