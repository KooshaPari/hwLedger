# Publishing status — @phenotype journey packages

## Staged tarballs (not yet published)

- `vendor/phenotype-journeys/phenotype-journey-viewer-0.1.0.tgz`
- `vendor/phenotype-journeys/phenotype-journey-playwright-0.1.0.tgz`

## Why they are not published yet

The current GitHub token used by `gh` / `npm` **does not carry the
`write:packages` scope** required to push to GitHub Packages.

`gh auth status` reports the token scopes:

```
Token scopes: 'delete_repo', 'gist', 'read:org', 'repo', 'workflow'
```

No `write:packages` -> `npm publish` against `npm.pkg.github.com` will
fail with `401 Unauthorized`.

## How to unblock (one command)

Run this in a terminal attached to the user's GitHub account, then retry
the publish:

```sh
gh auth refresh -s write:packages
```

After the browser re-auth round-trip, confirm the scope landed:

```sh
gh auth status
# expect: Token scopes: ... 'write:packages' ...
```

## Publish retry (once scope lands)

From repo root:

```sh
# Configure npm to use the GitHub Packages registry for @phenotype
npm config set @phenotype:registry https://npm.pkg.github.com

# Authenticate npm against the GitHub token
echo "//npm.pkg.github.com/:_authToken=$(gh auth token)" >> ~/.npmrc

# Publish both tarballs
npm publish vendor/phenotype-journeys/phenotype-journey-viewer-0.1.0.tgz --access restricted
npm publish vendor/phenotype-journeys/phenotype-journey-playwright-0.1.0.tgz --access restricted
```

## Verification

```sh
npm view @phenotype/journey-viewer version
npm view @phenotype/journey-playwright version
# expect both to print 0.1.0
```

Once both packages are live, the docs-site can drop the `vendor/` path
dependency and consume the registry versions directly.
