# Vendored `@phenotype/journey-viewer` tarballs

These tarballs are produced by `npm pack` from
`phenotype-journeys/npm/{journey-viewer,journey-playwright}`.

They exist here because the GitHub Packages npm registry requires a PAT
with `write:packages` scope to publish, and at the time of the migration
the available token did not have that scope. Once the package is
published to `npm.pkg.github.com`, `docs-site/package.json` should switch
from `file:../vendor/...` to `@phenotype/journey-viewer@^0.1.0` + a
`.npmrc` pointing at the GitHub Packages registry.

## Regenerate

```bash
cd ../../../phenotype-journeys/npm/journey-viewer
npm pack --pack-destination ../dist
cp ../dist/phenotype-journey-viewer-*.tgz \
  ../../hwLedger/vendor/phenotype-journeys/
cd ../journey-playwright
npm pack --pack-destination ../dist
cp ../dist/phenotype-journey-playwright-*.tgz \
  ../../hwLedger/vendor/phenotype-journeys/
```

See `phenotype-journeys/npm/PUBLISHING.md` for the full publish flow.
