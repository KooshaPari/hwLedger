/**
 * rich-refs: markdown-it plugin that linkifies spec-style references.
 *
 * Supported patterns (inline text only — never inside code/links):
 *   §5.2 or §5           → #5-or #5-2 anchor on current page
 *   PLAN.md §5           → GitHub blob URL (PLAN.md lives at repo root)
 *   PRD.md §3.1          → GitHub blob URL
 *   ADR-0004             → /architecture/adrs/0004-*
 *   FR-PLAN-003          → /reference/fr#fr-plan-003
 *   NFR-006              → /reference/fr#nfr-006
 *   arxiv:2412.19437     → https://arxiv.org/abs/2412.19437
 *   github#repo#123      → https://github.com/KooshaPari/repo/issues/123
 *
 * Registered via markdown hook in config.ts; runs after other inline parsers
 * so it safely walks `text` tokens without disturbing links, code, or html.
 */
import type MarkdownIt from 'markdown-it'
import type Token from 'markdown-it/lib/token'

// ADR slug lookup — filenames under docs-site/architecture/adrs.
// Keep in sync by regenerating from disk if needed; hardcoded for perf.
const ADR_SLUGS: Record<string, string> = {
  '0001': '0001-rust-core-three-native-guis',
  '0002': '0002-oMlx-fat-fork',
  '0003': '0003-fleet-wire-axum-not-grpc',
  '0004': '0004-math-core-dispatch',
  '0005': '0005-shared-crate-reuse',
  '0006': '0006-macos-codesign-notarize-sparkle',
  '0007': '0007-ffi-raw-c-over-uniffi',
  '0008': '0008-wp21-deferred-pending-apple-dev',
  '0009': '0009-fleet-mtls-admin-cn',
}

const GH_REPO_BASE = 'https://github.com/KooshaPari/hwLedger/blob/main'

// Combined regex. Order matters — longer/more-specific patterns first.
// Groups:
//  1 = github repo, 2 = github issue
//  3 = arxiv id
//  4 = ADR number
//  5 = FR/NFR identifier
//  6 = spec doc (PLAN/PRD/FUNCTIONAL_REQUIREMENTS), 7 = §N[.M]
//  8 = bare §N[.M]
const RE = new RegExp(
  [
    'github#([A-Za-z0-9_.-]+)#(\\d+)',
    'arxiv:(\\d{4}\\.\\d{4,5})',
    '\\b(ADR-(?:\\d{4}))\\b',
    '\\b((?:FR|NFR)(?:-[A-Z]+)?-\\d{3,4})\\b',
    '\\b(PLAN\\.md|PRD\\.md|FUNCTIONAL_REQUIREMENTS\\.md)\\s*§(\\d+(?:\\.\\d+)?)',
    '§(\\d+(?:\\.\\d+)?)',
  ].join('|'),
  'g',
)

interface Replacement {
  match: string
  href: string
  external: boolean
}

function resolve(m: RegExpExecArray): Replacement | null {
  const full = m[0]
  if (m[1] && m[2]) {
    return { match: full, href: `https://github.com/KooshaPari/${m[1]}/issues/${m[2]}`, external: true }
  }
  if (m[3]) {
    return { match: full, href: `https://arxiv.org/abs/${m[3]}`, external: true }
  }
  if (m[4]) {
    const num = m[4].slice(4) // strip "ADR-"
    const slug = ADR_SLUGS[num] ?? `${num}`
    return { match: full, href: `/architecture/adrs/${slug}`, external: false }
  }
  if (m[5]) {
    return { match: full, href: `/reference/fr#${m[5].toLowerCase()}`, external: false }
  }
  if (m[6] && m[7]) {
    return { match: full, href: `${GH_REPO_BASE}/${m[6]}#${m[7].replace('.', '')}`, external: true }
  }
  if (m[8]) {
    return { match: full, href: `#${m[8].replace('.', '-')}`, external: false }
  }
  return null
}

function processText(text: string): Replacement[] {
  const out: Replacement[] = []
  let match: RegExpExecArray | null
  RE.lastIndex = 0
  while ((match = RE.exec(text)) !== null) {
    const r = resolve(match)
    if (r) out.push(r)
  }
  return out
}

export function richRefs(md: MarkdownIt): void {
  const walk = (tokens: Token[]): void => {
    for (let i = 0; i < tokens.length; i++) {
      const tok = tokens[i]
      if (tok.type === 'inline' && tok.children) {
        tok.children = transformChildren(md, tok.children)
      }
    }
  }

  md.core.ruler.push('rich_refs', (state) => {
    walk(state.tokens)
  })
}

function transformChildren(md: MarkdownIt, children: Token[]): Token[] {
  const result: Token[] = []
  let linkDepth = 0
  for (const child of children) {
    if (child.type === 'link_open') linkDepth++
    if (child.type === 'link_close') linkDepth = Math.max(0, linkDepth - 1)

    if (child.type === 'text' && linkDepth === 0) {
      const replaced = splitTextToken(md, child)
      if (replaced) {
        result.push(...replaced)
        continue
      }
    }
    result.push(child)
  }
  return result
}

function splitTextToken(md: MarkdownIt, token: Token): Token[] | null {
  const text = token.content
  if (!text) return null
  RE.lastIndex = 0
  let last = 0
  let match: RegExpExecArray | null
  const pieces: Token[] = []
  let touched = false

  while ((match = RE.exec(text)) !== null) {
    const r = resolve(match)
    if (!r) continue
    touched = true
    const start = match.index
    if (start > last) {
      const t = new (token.constructor as any)('text', '', 0) as Token
      t.content = text.slice(last, start)
      pieces.push(t)
    }
    const open = new (token.constructor as any)('link_open', 'a', 1) as Token
    open.attrs = [['href', r.href], ['class', 'rich-ref']]
    if (r.external) {
      open.attrs.push(['target', '_blank'])
      open.attrs.push(['rel', 'noopener noreferrer'])
    }
    pieces.push(open)
    const inner = new (token.constructor as any)('text', '', 0) as Token
    inner.content = r.match
    pieces.push(inner)
    const close = new (token.constructor as any)('link_close', 'a', -1) as Token
    pieces.push(close)
    last = start + r.match.length
  }
  if (!touched) return null
  if (last < text.length) {
    const t = new (token.constructor as any)('text', '', 0) as Token
    t.content = text.slice(last)
    pieces.push(t)
  }
  return pieces
}

export default richRefs
