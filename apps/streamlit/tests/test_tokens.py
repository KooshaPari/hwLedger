"""Unit tests for lib.tokens.fmt_tokens / ticks_up_to.

Traces to: FR-PLAN-003
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.abspath(os.path.join(HERE, "..")))

from lib.tokens import LOG_TICKS, fmt_tokens, ticks_up_to  # noqa: E402


def test_fmt_tokens_small():
    assert fmt_tokens(128) == "128"
    assert fmt_tokens(0) == "0"


def test_fmt_tokens_kilo():
    assert fmt_tokens(4096) == "4K"
    assert fmt_tokens(128 * 1024) == "128K"


def test_fmt_tokens_mega():
    assert fmt_tokens(1_048_576) == "1M"
    assert fmt_tokens(10 * 1_048_576) == "10M"


def test_fmt_tokens_fractional_mega():
    # 1.5M tokens (non-power-of-two) should display the fractional form.
    assert fmt_tokens(int(1.5 * 1_048_576)) == "1.5M"


def test_ticks_up_to_none_is_full_range():
    assert ticks_up_to(None) == list(LOG_TICKS)
    assert ticks_up_to(0) == list(LOG_TICKS)


def test_ticks_up_to_model_cap_trims_and_appends():
    out = ticks_up_to(32_768)
    assert out[-1] == 32_768
    assert all(t <= 32_768 for t in out)


def test_ticks_up_to_non_tick_cap_is_appended():
    out = ticks_up_to(12_345)
    assert 12_345 in out
    assert out[-1] == 12_345


def test_ticks_up_to_below_min_returns_min():
    out = ticks_up_to(64)  # below min tick (128)
    assert out == [64]


if __name__ == "__main__":
    # Lightweight runner for environments without pytest installed.
    import inspect
    ns = dict(globals())
    fns = [v for k, v in ns.items() if k.startswith("test_") and callable(v)]
    failed = 0
    for fn in fns:
        try:
            fn()
            print(f"OK   {fn.__name__}")
        except AssertionError as exc:
            failed += 1
            print(f"FAIL {fn.__name__}: {exc}")
    sys.exit(1 if failed else 0)
