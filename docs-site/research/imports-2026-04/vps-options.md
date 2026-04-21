---
title: VPS and dedicated hosting for self-hosted workloads
description: Price/performance tiers for 64 GB + 8 core + 500 GB self-hosted infra — Contabo, Hetzner Cloud, Hetzner auction, OVH Eco. Where the shared-vCPU → bare-metal cliff sits.
sources:
  - ChatGPT-VPS for Self-hosted Services.md
  - ChatGPT-VPS for Heavy Workloads.md
date_imported: 2026-04-20
---

# VPS and dedicated hosting

## Distilled findings

For a 64 GB / 8+ core / 500 GB target (Nextcloud, Neo4j, Postgres, CPU inference sidecars), the market has clear tiers. Shared-vCPU VPS below ~$35/mo **will CPU-steal during peak hours**, which is fatal for CPU inference and graph-DB query latency.

### Price/performance tiers (Apr 2026)

| Tier | Provider | Product | Specs | $/mo | Notes |
|------|----------|---------|-------|------|-------|
| Cheapest (shared vCPU) | Contabo | Cloud VPS 50 / XL | 10–16 vCPU, 60–64 GB, 600 GB SSD / 300 GB NVMe | $35–46 | Overprovisioned; US DCs in Seattle/LA; single-core perf low |
| Performance king (bare metal) | Hetzner | Server Auction (Ryzen 5 / i7) | 64 GB ECC, 2× 512 GB NVMe | €38–50 | DE/FI only (~140 ms from US-West); 100% dedicated threads |
| Dedicated cloud vCPU | Hetzner | CCX43 (Cloud) | 16 dedicated AMD EPYC, 64 GB, 360 GB NVMe | ~$125 | US Oregon; needs attached block storage for 500 GB |
| North America bare metal | OVHcloud Eco (Rise/Kimsufi) | Ryzen 5 | 64 GB, 2× 512 GB NVMe | $70–80 | US-West POPs; solves EU-latency problem |

### Hetzner auction PX62-NVMe observed listings

- Xeon E-2176G, 64 GB, 2× 960 GB U.2 DC NVMe — **€43.70/mo**
- Xeon E-2276G (same chassis, newer CPU) — **€51.70/mo**

Same upgrade envelope: RAM 64→128→256 GB, up to 4 DC NVMe, no CPU/GPU upgrade on either ([PX Server config page](https://docs.hetzner.com/robot/dedicated-server/server-lines/px-server/)). €8/mo delta for the newer CPU is the right call unless strictly cost-bound.

### Proxmox carve-up pattern

The standard recipe for a Hetzner bare-metal auction box: install Proxmox, isolate Nextcloud + generic web services in one VM, dedicate remaining threads to a Neo4j + CPU inference VM. This gets you "two servers" of behaviour on one €43/mo chassis, with unshared NVMe queues per VM.

## Citations

- [Hetzner PX Server configurations](https://docs.hetzner.com/robot/dedicated-server/server-lines/px-server/)
- [Hetzner server auction FAQs](https://docs.hetzner.com/robot/general/server-auction-faqs/)
- [Hetzner minimum hardware configuration](https://docs.hetzner.com/robot/dedicated-server/dedicated-server-hardware/minimum-hardware-configuration/)

## hwLedger implications

- Fleet node-class catalogue should distinguish **shared vCPU** vs **dedicated vCPU** vs **bare metal** as a first-class property — the planner's latency tail predictions should assume p99 >> p50 on shared vCPU providers (Contabo), p99 ≈ p50 × 1.5 on dedicated bare metal.
- Cross-region latency note: the €40/mo Hetzner auction tier is only usable for async/background workloads from US clients (140 ms baseline RTT). Interactive fleet agents in US should default to OVH Eco US-West or Hetzner Cloud CCX (Oregon).
- Ledger should record node-class per host so cost reconciliation can normalise €/mo vs $/hr equivalents cleanly.

## See also

- [`fleet/cloud-rentals.md`](/fleet/cloud-rentals) — GPU rental tier (complementary, not substitute)
- [`research/imports-2026-04/hetzner-auction.md`](/research/imports-2026-04/hetzner-auction) — deeper treatment of auction-line constraints
- [`research/imports-2026-04/self-host-vs-api-cost.md`](/research/imports-2026-04/self-host-vs-api-cost) — break-even framing
