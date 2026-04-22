# CLI: ingest local GGUF

Ingest a GGUF model file from disk into the local hwLedger model registry. Extracts architecture metadata, KV layout, and quantisation from the file header without network I/O.

<JourneyViewer manifest="/cli-journeys/manifests/ingest-local-gguf/manifest.verified.json" />

## Reproduce

```bash
hwledger ingest local ./models/mistral-7b-instruct-q4_k_m.gguf
```

## Next steps

- [Ingest error UX](./cli-ingest-error.md) — fail-loudly behaviour on bad files (NFR-007)
- [HF search + ingest](./cli-hf-search-deepseek.md) — resolve from the Hugging Face hub

## Source

[Recorded tape on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/apps/cli-journeys/tapes/ingest-local-gguf.tape)
