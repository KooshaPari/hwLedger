# Testing Strategy

Commands run:
- `cargo test -p hwledger-journey-render`
- `bun run build` from `docs-site`
- `env CLANG_MODULE_CACHE_PATH=/tmp/clang-module-cache swift build --disable-sandbox`

Expected follow-up validation:
- Run `cargo run -p hwledger-journey-render -- all docs-site/public --force
  --voiceover auto --judge none` outside the nested sandbox.
- Re-run `bun run check:media` and confirm no warnings remain.
