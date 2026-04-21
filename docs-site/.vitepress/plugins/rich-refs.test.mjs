// Smoke test for rich-refs plugin. Run with `bun run .vitepress/plugins/rich-refs.test.mjs`.
import MarkdownIt from 'markdown-it'
import { richRefs } from './rich-refs.ts'

const md = new MarkdownIt({ html: true }).use(richRefs)

const cases = [
  ['See §5.2 for details.', 'href="#5-2"'],
  ['Refer to ADR-0004.', 'href="/architecture/adrs/0004-math-core-dispatch"'],
  ['Requirement FR-PLAN-003 covers this.', 'href="/reference/fr#fr-plan-003"'],
  ['Paper arxiv:2412.19437 is cited.', 'href="https://arxiv.org/abs/2412.19437"'],
  ['See PLAN.md §5 for scope.', 'href="https://github.com/KooshaPari/hwLedger/blob/main/PLAN.md#5"'],
  ['issue github#hwLedger#42', 'href="https://github.com/KooshaPari/hwLedger/issues/42"'],
  ['bare NFR-006 here', 'href="/reference/fr#nfr-006"'],
]

let failed = 0
for (const [input, expect] of cases) {
  const html = md.render(input)
  const ok = html.includes(expect)
  console.log(ok ? 'PASS' : 'FAIL', '-', input)
  if (!ok) {
    console.log('  expected:', expect)
    console.log('  got:     ', html)
    failed++
  }
}
process.exit(failed)
