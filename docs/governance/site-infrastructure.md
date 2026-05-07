# Site Infrastructure

hwLedger landing is served from Vercel at `https://hwledger.kooshapari.com` and
mirrored to GitHub Pages at `https://kooshapari.github.io/hwledger-landing/`.

The Pages workflow builds with `GITHUB_PAGES=true`, which makes Astro emit links
under `/hwledger-landing/`. The custom-domain build keeps `/` as the base path.

The shared visual base is GMK Arch teal, rooted at `#7ebab5`.
