# CLI: ingest --error handling

When model ingestion fails (missing file, network error, invalid format), hwLedger follows the fail-loudly invariant: no silent fallback, clear error message pointing to the problem.

## What you'll see

Running ingest with a missing file:
```bash
hwledger ingest gguf:///tmp/does-not-exist.gguf
```

<!-- SHOT-MISMATCH: caption="Error message + hint" expected=[error,message,hint] matched=[] -->
<Shot src="/cli-journeys/keyframes/ingest-error/frame-001.png"
      caption="Error message + hint"
      size="small" align="right"
      :annotations='[{"bbox":[60,220,480,32],"label":"E-INGEST-02","color":"#f38ba8","style":"dashed"}]' />

The tool returns:
- **Exit code**: 3 (Resource not found)
- **Error message**: "File not found: /tmp/does-not-exist.gguf"
- **Hint**: "Check the path and try again"

No silent retry, no hidden cache hit. The error is loud and actionable.

<JourneyViewer manifest="/cli-journeys/manifests/ingest-error/manifest.verified.json" />

## What to watch for

- **Clear error type**: "File not found" (not "I/O error" or "generic failure")
- **Full path printed**: Exact path that was looked up
- **Exit code**: Non-zero so scripts can detect failure
- **No hiding**: Not silently using a cached version or retrying forever
- **Actionable hint**: "Check the path" tells you what to do next

## Next steps

- [Ingest success flow](/reference/cli#ingest) — successful download and cache
- [Troubleshooting](/guides/troubleshooting#model-ingest-hangs) — common ingest issues
- [Exit Codes](/reference/exit-codes) — meaning of exit code 3

## Reproduce

```bash
# Fail explicitly (file doesn't exist)
hwledger ingest gguf:///tmp/does-not-exist.gguf
# Exit code: 3
# stderr: File not found: /tmp/does-not-exist.gguf
```

## Source

[Recorded journey tape on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-gui-recorder/tapes/ingest-error.verified.json)
