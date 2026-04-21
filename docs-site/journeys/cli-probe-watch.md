# CLI: probe --watch

Continuous GPU monitoring mode. Streams telemetry every 2 seconds until you press Ctrl+C. Perfect for watching GPU during inference or system stress tests.

## What you'll see

Running `hwledger probe --watch`:
- GPU memory usage updates every 2 seconds
- Temperature readings refresh in real-time
- Utilization percentage changes as workloads come and go
- Clean Ctrl+C exit (<200 ms) with no leftover processes

<Shot src="/cli-journeys/keyframes/probe-watch/frame-001.png"
      caption="Live-refresh header"
      size="small" align="right"
      :annotations='[{"bbox":[60,80,400,20],"label":"Refresh header"}]' />

<Shot src="/cli-journeys/keyframes/probe-watch/frame-002.png"
      caption="Second tick — memory row updates"
      size="small" align="left" />

<Shot src="/cli-journeys/keyframes/probe-watch/frame-003.png"
      caption="Clean Ctrl+C exit line"
      size="small" align="right" />

This is useful for:
- Watching a running inference job
- Stress testing (check if GPU throttles under load)
- Debugging OOM crashes (see memory spike)
- Monitoring during fleet job execution

<JourneyViewer manifest="/cli-journeys/manifests/probe-watch/manifest.verified.json" />

## What to watch for

- **Update interval**: ~2s between refreshes (notice the timestamp change)
- **Memory changes**: Watch as free VRAM drops when inference starts
- **Temperature**: Rises gradually as GPU heats up under load
- **Utilization**: 0% when idle, 85-100% during inference
- **Ctrl+C response**: Press Ctrl+C and notice immediate clean exit (no hanging processes)

## Next steps

- [Probe basic usage](/journeys/cli-probe-list) — one-time GPU listing
- [Plan before inference](/journeys/cli-plan-help) — determine if model fits
- [Probe command reference](/reference/cli#probe) — all flags

## Reproduce

```bash
# Watch GPU for 30 seconds, then Ctrl+C
hwledger probe --watch

# JSON output for scripting
hwledger probe --watch --json | while read line; do
  echo "$line" | jq '.gpus[0].memory_free_gb'
done
```

## Source

[Recorded journey tape on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-gui-recorder/tapes/probe-watch.verified.json)
