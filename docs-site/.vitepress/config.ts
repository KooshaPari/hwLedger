import { defineConfig } from 'vitepress'
import mdMathjax3 from 'markdown-it-mathjax3'
import { withMermaid } from 'vitepress-plugin-mermaid'

export default withMermaid(defineConfig({
  title: 'hwLedger',
  description: 'LLM capacity planner + fleet ledger + desktop inference runtime',
  base: process.env.GITHUB_ACTIONS ? '/hwLedger/' : '/',
  lang: 'en-US',
  cleanUrls: true,
  srcDir: '.',

  markdown: {
    math: true,
    config: (md) => {
      md.use(mdMathjax3)
    }
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

      '/research/': [
        { text: 'Research Index', link: '/research/' }
      ],

      '/journeys/': [
        { text: 'UI Journeys', link: '/journeys/' }
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
    ['script', { src: 'https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-mml-chtml.js' }]
  ]
}))
