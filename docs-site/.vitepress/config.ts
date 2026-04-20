import { defineConfig } from 'vitepress'
import mdMathjax3 from 'markdown-it-mathjax3'
import { withMermaid } from 'vitepress-plugin-mermaid'
import { readdirSync, readFileSync } from 'fs'
import { join } from 'path'
import matter from 'gray-matter'

// Auto-generate sidebar for /research/ by reading frontmatter from research/*.md
function buildResearchSidebar() {
  const researchDir = join(__dirname, '../research')
  const files = readdirSync(researchDir)
    .filter(f => f.endsWith('.md') && f !== 'index.md')
    .sort()

  if (files.length === 0) {
    // Fallback to static list if no files found
    return [
      { text: 'Research Index', link: '/research/' }
    ]
  }

  const items = files.map(filename => {
    const filepath = join(researchDir, filename)
    const content = readFileSync(filepath, 'utf-8')
    const { data } = matter(content)
    const slug = filename.replace('.md', '')

    return {
      text: (data.title as string) || slug,
      link: `/research/${slug}`
    }
  })

  return [
    { text: 'Research Index', link: '/research/' },
    ...items
  ]
}

// Auto-generate sidebar for /architecture/adrs/ by reading frontmatter from architecture/adrs/*.md
function buildAdrSidebar() {
  const adrDir = join(__dirname, '../architecture/adrs')
  const files = readdirSync(adrDir)
    .filter(f => f.endsWith('.md'))
    .sort()

  if (files.length === 0) {
    return [{ text: 'ADRs', link: '/architecture/adrs' }]
  }

  const items = files.map(filename => {
    const filepath = join(adrDir, filename)
    const content = readFileSync(filepath, 'utf-8')
    const { data } = matter(content)
    const slug = filename.replace('.md', '')

    return {
      text: (data.title as string) || slug,
      link: `/architecture/adrs/${slug}`
    }
  })

  return items
}

export default withMermaid(defineConfig({
  title: 'hwLedger',
  // Relaxed to external-only: block internal dead links (catches regressions),
  // accept external links without verification (too slow). ADR-0006 previously
  // had two dead ../../ links; they now resolve to GitHub blob URLs.
  ignoreDeadLinks: [
    'localhostLinks',
    // Journeys file paths that will exist after UI tests run
    /HwLedgerUITests/,
  ],
  description: 'LLM capacity planner + fleet ledger + desktop inference runtime',
  base: process.env.GITHUB_ACTIONS ? '/hwLedger/' : '/',
  lang: 'en-US',
  cleanUrls: true,
  srcDir: '.',

  // VitePress's built-in `math: true` already wires markdown-it-mathjax3.
  // Adding it again via `config.use()` double-processes the AST and produces
  // literal unescaped LaTeX as "code" (renders yellow/red in the syntax
  // highlighter). We also drop the MathJax CDN <script> below — VitePress
  // renders math to SVG at build time, no client-side engine needed.
  markdown: {
    math: true
  },

  themeConfig: {
    logo: '/logo.svg',
    siteTitle: 'hwLedger',

    nav: [
      { text: 'Home', link: '/' },
      { text: 'Architecture', link: '/architecture/' },
      { text: 'Math', link: '/math/kv-cache' },
      { text: 'Fleet', link: '/fleet/overview' },
      { text: 'Getting Started', link: '/getting-started/install' },
      { text: 'Research', link: '/research/' },
      { text: 'GitHub', link: 'https://github.com/KooshaPari/hwLedger' }
    ],

    sidebar: {
      '/': [
        { text: 'Overview', link: '/' },
        {
          text: 'Documentation',
          items: [
            { text: 'Architecture', link: '/architecture/' },
            { text: 'Architecture Decisions', link: '/architecture/adrs' },
            { text: 'Math Core', link: '/math/kv-cache' },
            { text: 'Fleet Ledger', link: '/fleet/overview' },
            { text: 'Getting Started', link: '/getting-started/install' },
            { text: 'Research', link: '/research/' }
          ]
        },
        {
          text: 'UI Journeys',
          link: '/journeys/',
          collapsed: true
        }
      ],

      '/architecture/': [
        { text: 'Architecture Overview', link: '/architecture/' },
        { text: 'Component Map', link: '/architecture/#component-map' },
        { text: 'ADRs', link: '/architecture/adrs' }
      ],

      '/math/': [
        { text: 'KV Cache Formulas', link: '/math/kv-cache' }
      ],

      '/fleet/': [
        { text: 'Fleet Overview', link: '/fleet/overview' }
      ],

      '/getting-started/': [
        { text: 'Installation', link: '/getting-started/install' }
      ],

      '/research/': buildResearchSidebar(),

      '/journeys/': [
        { text: 'Overview', link: '/journeys/' },
        {
          text: 'CLI journeys',
          collapsed: false,
          items: [
            { text: 'plan — DeepSeek-V3', link: '/journeys/cli-plan-deepseek' },
            { text: 'plan --help', link: '/journeys/cli-plan-help' },
            { text: 'probe list', link: '/journeys/cli-probe-list' },
            { text: 'probe watch', link: '/journeys/cli-probe-watch' },
            { text: 'ingest error UX', link: '/journeys/cli-ingest-error' }
          ]
        },
        {
          text: 'Web (Streamlit) journeys',
          collapsed: false,
          items: [
            { text: 'Planner — seq length sweep', link: '/journeys/streamlit-planner' },
            { text: 'Probe — device inventory', link: '/journeys/streamlit-probe' },
            { text: 'Fleet — offline fail-loudly', link: '/journeys/streamlit-fleet' },
            { text: 'Exports — vLLM / llama.cpp / MLX', link: '/journeys/streamlit-exports' }
          ]
        }
      ]
    },

    socialLinks: [
      { icon: 'github', link: 'https://github.com/KooshaPari/hwLedger' }
    ],

    editLink: {
      pattern: 'https://github.com/KooshaPari/hwLedger/edit/main/docs-site/:path',
      text: 'Edit this page on GitHub'
    },

    lastUpdated: {
      text: 'Last updated',
      formatOptions: {
        dateStyle: 'short',
        timeStyle: 'short'
      }
    },

    footer: {
      message: 'Released under the Apache 2.0 License.',
      copyright: 'Copyright © 2024-2026 hwLedger Contributors'
    },

    outline: 'deep',

    search: {
      provider: 'local',
      options: {
        miniSearch: {
          options: {
            processTerm: (t) => t.toLowerCase()
          }
        }
      }
    }
  },

  head: [
    ['meta', { name: 'theme-color', content: '#3c3c44' }],
    ['link', { rel: 'icon', href: '/favicon.ico', type: 'image/ico' }],
    // MathJax rendered server-side via markdown-it-mathjax3 (see markdown block).
    // No runtime CDN script — would re-process the already-rendered SVGs.
  ]
}))
