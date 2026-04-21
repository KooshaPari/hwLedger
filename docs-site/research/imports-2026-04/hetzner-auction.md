---
title: Hetzner auction line — upgrade envelope and dead-end risks
description: What you can and can't upgrade on a Hetzner auction server post-purchase. CPU and GPU class are effectively frozen; RAM/NVMe are the only realistic expansion axes.
sources:
  - ChatGPT-Hetzner auction server upgrades.md
date_imported: 2026-04-20
---

# Hetzner auction — upgrade envelope

## Distilled findings

Hetzner auction servers are **fixed-platform chassis**. Post-purchase upgrades are limited to a per-line allowlist maintained in Hetzner Robot; anything outside that allowlist is a server replacement, not an upgrade.

### Supported post-purchase modifications (general pattern)

| Axis | Status | Notes |
|------|--------|-------|
| RAM | Sometimes | Line-specific. Many lines say "no RAM upgrades"; PX62-NVMe allows 64→128→256 GB DDR4 ECC |
| Drives (NVMe/SATA/HDD) | Sometimes | Up to chassis layout limit. PX62-NVMe: up to 4 DC NVMe, or 2 consumer NVMe, or 6 SATA SSD, or 3 HDD. Not all combinations possible |
| NIC / uplinks | Sometimes | Line-specific |
| CPU | **No** | No in-place swap path in Hetzner's upgrade flow. "Upgrade" = migrate to a different server |
| GPU (on non-GPU line) | **No** | GPU servers are a separate product line with fixed configs |

Source: [Hetzner Robot docs — price add-ons](https://docs.hetzner.com/robot/dedicated-server/dedicated-server-hardware/price-server-addons/), [PX Server configurations](https://docs.hetzner.com/robot/dedicated-server/server-lines/px-server/), [GPU server line](https://docs.hetzner.com/robot/dedicated-server/server-lines/gpu-server/).

### Procedural gotchas

- **Cannot add hardware during checkout** on auction orders. Post-delivery, supported add-ons are requested via Robot support ticket.
- **You do not do the hardware work yourself.** Hetzner DC engineers perform all physical changes on supported upgrades.
- Availability of supported SKUs rotates — a line that documented 256 GB RAM last year may only offer 128 GB today.

### Decision heuristic

- Want **more RAM / more NVMe** later → auction is fine. Check the exact line's add-on page first.
- Want **different CPU class** or **add a GPU** later → **skip auction**. Buy the line that already has what you want (GPU-capable GEX130 with RTX 6000 Ada, or AX/DX EPYC line for CPU), or plan an explicit migration.
- Budget-capped and GPU is only a maybe → treat the auction box as disposable infrastructure from day one.

### Observed live listings (Apr 2026)

- PX62-NVMe, Xeon E-2176G, 64 GB, 2× 960 GB U.2 NVMe — €43.70/mo — upgradeable to 256 GB RAM + 4× DC NVMe but CPU-frozen at 6-core 5th-gen Xeon-E.
- PX62-NVMe, Xeon E-2276G (newer CPU) — €51.70/mo — same envelope, €8/mo premium for a meaningful CPU bump.

## Citations

- [Hetzner server auction FAQs](https://docs.hetzner.com/robot/general/server-auction-faqs/)
- [Hetzner dedicated server upgrade policy](https://docs.hetzner.com/robot/dedicated-server/dedicated-server-hardware/dedicated-server-upgrade/)
- [Hetzner minimum hardware configuration (current lines)](https://docs.hetzner.com/robot/dedicated-server/dedicated-server-hardware/minimum-hardware-configuration/)
- [Hetzner EX server line](https://docs.hetzner.com/robot/dedicated-server/server-lines/ex-server/)

## hwLedger implications

- Fleet node catalogue should store **upgrade-envelope metadata** per node (max RAM, max drives, CPU/GPU locked-flag) — not just current specs. The planner's "grow this node to handle 32B inference" recommendation should short-circuit on locked-CPU or locked-GPU nodes.
- Cost amortization math (see [self-host-vs-api-cost.md](/research/imports-2026-04/self-host-vs-api-cost)) should use a **shorter depreciation window** for auction-line CPUs (already 2+ generations behind) and treat the chassis as disposable at t+2 years.
- Migration cost (~1 day of fleet-agent ops + data transfer) should be a first-class line item in the "buy vs rent" planner output when targeting auction nodes.

## See also

- [`research/imports-2026-04/vps-options.md`](/research/imports-2026-04/vps-options) — broader hosting tier comparison
- [`fleet/cloud-rentals.md`](/fleet/cloud-rentals)
