---
title: Self-host vs subscription — break-even math
description: Convert chat/coding subscriptions into rented-GPU-hour equivalents. $20 subs are product wrappers, not infra substitutes. Self-host only breaks even above 8h/day utilization.
sources:
  - ChatGPT-Subscription vs Self-Hosted Cost.md
  - ChatGPT-API Pricing and Model Efficiency.md
date_imported: 2026-04-20
---

# Self-host vs subscription — break-even math

## Distilled findings

Stop comparing "$20 subscription vs $20 GPU" — unit mismatch. Convert both to **GPU-hour equivalents** at marketplace pricing, then compare to **amortized owned-hardware hours**.

### Subscription anchor prices (April 2026)

| Plan | Price | Included value |
|------|-------|----------------|
| GitHub Copilot Pro | $10/mo | 300 premium req (≈$12 at overage) |
| ChatGPT Plus | $20/mo | bundled usage limits |
| Cursor Pro | $20/mo | ~$20 of frontier usage at API price |
| Copilot Pro+ | $39/mo | 1,500 premium req (≈$60 at overage) |
| Cursor Pro+ | $60/mo | 3× Pro usage |
| Claude Max 5x | $100/mo | 5× Pro session usage |
| ChatGPT Pro / Cursor Ultra / Claude Max 20x | $200/mo | 20× session usage |

Sources: [OpenAI rate card](https://help.openai.com/en/articles/11481834-chatgpt-rate-card-business-enterpriseedu), [Cursor pricing](https://cursor.com/pricing), [GitHub Copilot billing](https://docs.github.com/en/copilot/concepts/billing/billing-for-individuals), [Claude Max](https://claude.com/pricing/max).

### Marketplace GPU rental (RunPod spot/community, Apr 2026)

| Card | VRAM | $/hr |
|------|------|------|
| A40 | 48 GB | $0.35 |
| L40 | 48 GB | $0.69 |
| RTX 6000 Ada | 48 GB | $0.74 |
| L40S | 48 GB | $0.79 |
| A100 PCIe | 80 GB | $1.19 |
| H100 PCIe | 80 GB | $1.99 |
| H200 | 141 GB | $3.59 |

Lambda on comparable cards is 2–3× more ([Lambda instances](https://lambda.ai/instances)).

### Subscription → GPU-hours table

| Plan | A40-48GB hrs | L40S-48GB hrs | A100-80GB hrs | H100-80GB hrs |
|------|--------------|----------------|----------------|----------------|
| $20/mo | 57 | 25 | 17 | 10 |
| $100/mo | 286 | 127 | 84 | 50 |
| $200/mo | 571 | 253 | 168 | 101 |

### Self-hosted amortization

Assume $4K system, $1.2K expected resale over 2 years → $2,800 depreciated. Power at 450–600 W × $0.32/kWh = $0.14–$0.19/hr electricity.

- 8 hrs/day utilization → ~**$0.62/hr all-in**
- 16 hrs/day utilization → ~**$0.38/hr all-in**

**Break-even:** an idle local 24 GB card loses to a $20 subscription. A saturated local card beats marketplace 48 GB rental only above ~8 hrs/day of genuine load.

## Citations

- [RunPod GPU pricing](https://www.runpod.io/gpu-pricing)
- [Lambda instances](https://lambda.ai/instances)
- [Vast.ai RTX 4090 pricing](https://vast.ai/pricing/gpu/RTX-4090)

## hwLedger implications

- The "cost reconciliation" event-sourced ledger should report **three parallel cost axes** on every run: (a) raw $ spent, (b) equivalent GPU-hours at current spot, (c) amortized-owned-hardware cost. Today only (a) is tracked; ADR-0010 should extend the ledger schema.
- Planner's "should I rent" recommendation needs a utilization-aware cutoff: recommend rent for <8 h/day expected use at the target VRAM class; recommend buy only after 2–3 months of measured sustained utilization (per handoff compute plan).
- Subscription plans are **product wrappers**, not fungible GPU spend — the planner should not offer $-to-$ swaps between a Claude Max sub and a rental budget without a workflow-value caveat.

## See also

- [`fleet/cloud-rentals.md`](/fleet/cloud-rentals) — rental provider integration
- [`research/imports-2026-04/vps-options.md`](/research/imports-2026-04/vps-options) — non-GPU infra tier
- [`research/imports-2026-04/hetzner-auction.md`](/research/imports-2026-04/hetzner-auction) — bare-metal bottom of the market
