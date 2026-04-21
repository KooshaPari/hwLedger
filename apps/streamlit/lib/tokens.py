"""
Token-count formatting + log-spaced sequence-length tick list.

Shared across the Streamlit Planner / WhatIf pages so the slider semantics stay
identical. Kept dependency-free for unit tests.

Traces to: FR-PLAN-003
"""

# Log-spaced ticks from 128 → 10M tokens. Extra ticks at human milestones
# (128, 1K, 4K, 32K, 128K, 1M, 10M) so the slider lands cleanly on the
# values operators actually care about.
LOG_TICKS = [
    128, 256, 512, 1024, 2048, 4096, 8192,
    16_384, 32_768, 65_536, 131_072, 262_144,
    524_288, 1_048_576, 2_097_152, 4_194_304,
    10_000_000,
]


def fmt_tokens(v: int) -> str:
    """Format a token count with K/M suffixes — e.g. 4096→'4K', 1_048_576→'1M'."""
    if v >= 1_048_576 and v % 1_048_576 == 0:
        return f"{v // 1_048_576}M"
    if v >= 1_048_576:
        return f"{v / 1_048_576:.1f}M"
    if v >= 1024 and v % 1024 == 0:
        return f"{v // 1024}K"
    if v >= 1024:
        return f"{v / 1024:.1f}K"
    return str(v)


def ticks_up_to(max_tokens: int | None) -> list[int]:
    """Return LOG_TICKS filtered to <= max_tokens (plus max_tokens itself).

    When max_tokens is None or 0, the full list is returned unchanged.
    """
    if not max_tokens:
        return list(LOG_TICKS)
    filtered = [t for t in LOG_TICKS if t <= max_tokens]
    if max_tokens not in filtered:
        filtered.append(max_tokens)
    if not filtered:
        filtered = [min(LOG_TICKS)]
    return filtered
