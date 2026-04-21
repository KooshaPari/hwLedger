"""
FFI extension layer for sibling-agent functions:
- hwledger_search_* (HF model search)
- hwledger_predict_* (what-if memory prediction)

Both surfaces are auto-detected at runtime. When the sibling FFI symbols are
not yet present in libhwledger_ffi, the module falls back to a deterministic
mock that matches the shape the real FFI will return, so pages render end-to-end.

Traces to: brief §3 (HF search + predict wiring), §4 (real HF search UI).
"""

from __future__ import annotations

import ctypes
import json
import time
from dataclasses import dataclass, field, asdict
from typing import List, Optional, Tuple

from lib.ffi import lib  # type: ignore


# =============================================================================
# Public dataclasses (stable across real / mock paths)
# =============================================================================

@dataclass
class HfModel:
    id: str
    author: str
    downloads: int
    likes: int
    library: str
    tags: List[str]
    last_modified: str
    pipeline_tag: str
    private: bool = False
    gated: bool = False


@dataclass
class HfSearchResult:
    models: List[HfModel]
    total: int
    rate_limit_remaining: Optional[int]
    rate_limited: bool
    next_retry_after_s: Optional[int]


@dataclass
class PredictBreakdown:
    weights_mb: float
    kv_mb: float
    prefill_mb: float
    runtime_mb: float

    @property
    def total_mb(self) -> float:
        return self.weights_mb + self.kv_mb + self.prefill_mb + self.runtime_mb


@dataclass
class TransformationCitation:
    technique: str
    title: str
    url: str
    arxiv_id: Optional[str] = None


@dataclass
class WhatIfResult:
    baseline: PredictBreakdown
    candidate: PredictBreakdown
    techniques: List[str]
    verdict: str
    delta_pct: float
    citations: List[TransformationCitation] = field(default_factory=list)


# =============================================================================
# Real FFI detection
# =============================================================================

def _has_search() -> bool:
    # Sibling agent shipped `hwledger_hf_search(query_json, token) -> *mut c_char`
    # with companion `hwledger_hf_free_string`.
    return lib is not None and hasattr(lib, "hwledger_hf_search") and \
        hasattr(lib, "hwledger_hf_free_string")


def _has_predict() -> bool:
    # Sibling agent shipped `hwledger_predict(baseline, candidate, techniques,
    # workload) -> *mut c_char`, freed by `hwledger_predict_free`.
    return lib is not None and hasattr(lib, "hwledger_predict") and \
        hasattr(lib, "hwledger_predict_free")


def search_available() -> bool:
    return _has_search()


def predict_available() -> bool:
    return _has_predict()


# =============================================================================
# HF search — real path
# =============================================================================

def _search_real(
    query: str,
    library: Optional[str],
    sort: str,
    limit: int,
    token: Optional[str],
) -> Optional[HfSearchResult]:
    # Real sibling contract (crates/hwledger-ffi):
    #   extern "C" fn hwledger_hf_search(
    #       query_json: *const c_char,   // JSON: {text, library, sort, limit, tags...}
    #       token:      *const c_char,   // nullable
    #   ) -> *mut c_char;                // UTF-8 JSON, freed via hwledger_hf_free_string
    try:
        lib.hwledger_hf_search.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
        lib.hwledger_hf_search.restype = ctypes.c_void_p
        lib.hwledger_hf_free_string.argtypes = [ctypes.c_void_p]
        lib.hwledger_hf_free_string.restype = None

        qj = {
            "text": query or None,
            "library": library,
            "sort": sort,
            "limit": int(limit),
        }
        ptr = lib.hwledger_hf_search(
            json.dumps(qj).encode("utf-8"),
            token.encode("utf-8") if token else None,
        )
        if not ptr:
            return None
        try:
            raw = ctypes.string_at(ptr).decode("utf-8")
        finally:
            lib.hwledger_hf_free_string(ptr)
        payload = json.loads(raw)

        # FFI may return {"error": "..."} on failure (incl. 429 from Hub).
        if isinstance(payload, dict) and "error" in payload:
            err = str(payload["error"]).lower()
            if "429" in err or "rate" in err:
                return HfSearchResult(models=[], total=0,
                                      rate_limit_remaining=0,
                                      rate_limited=True,
                                      next_retry_after_s=60)
            return None

        # Payload is a JSON array of ModelCard (per hwledger-hf-client).
        raw_list = payload if isinstance(payload, list) else payload.get("models", [])
        models: List[HfModel] = []
        for m in raw_list:
            models.append(HfModel(
                id=m.get("id") or m.get("model_id") or "unknown",
                author=m.get("author") or (m.get("id", "/").split("/")[0]),
                downloads=int(m.get("downloads", 0) or 0),
                likes=int(m.get("likes", 0) or 0),
                library=m.get("library") or m.get("library_name") or "",
                tags=list(m.get("tags", []) or []),
                last_modified=m.get("last_modified") or m.get("lastModified") or "",
                pipeline_tag=m.get("pipeline_tag") or "",
                private=bool(m.get("private", False)),
                gated=bool(m.get("gated", False)),
            ))
        return HfSearchResult(
            models=models, total=len(models),
            rate_limit_remaining=None,
            rate_limited=False, next_retry_after_s=None,
        )
    except Exception:
        return None


# =============================================================================
# HF search — mock fallback (quick-picks curated for 2026 Q2)
# =============================================================================

_QUICK_PICKS: List[HfModel] = [
    HfModel("meta-llama/Meta-Llama-4-70B", "meta-llama", 2_450_000, 8_120,
            "transformers", ["llama-4", "text-generation", "moe"], "2026-02-14",
            "text-generation", False, True),
    HfModel("deepseek-ai/DeepSeek-V3", "deepseek-ai", 3_120_000, 11_400,
            "transformers", ["mla", "moe", "text-generation"], "2025-12-20",
            "text-generation"),
    HfModel("Qwen/Qwen3.6-72B-Instruct", "Qwen", 1_980_000, 6_420,
            "transformers", ["qwen3", "chat", "text-generation"], "2026-03-01",
            "text-generation"),
    HfModel("google/gemma-3-27b", "google", 1_210_000, 4_100,
            "transformers", ["gemma", "text-generation"], "2026-01-15",
            "text-generation", False, True),
    HfModel("state-spaces/mamba-3-8b", "state-spaces", 240_000, 980,
            "transformers", ["mamba", "ssm", "text-generation"], "2026-02-28",
            "text-generation"),
    HfModel("mistralai/Mistral-Nemo-Instruct-2407", "mistralai", 890_000, 3_210,
            "transformers", ["mistral", "chat"], "2025-10-01", "text-generation"),
    HfModel("meta-llama/Llama-3.3-70B-Instruct", "meta-llama", 1_720_000, 5_430,
            "transformers", ["llama-3", "chat"], "2025-11-11", "text-generation",
            False, True),
    HfModel("microsoft/Phi-4-multimodal", "microsoft", 540_000, 2_100,
            "transformers", ["phi", "multimodal"], "2026-02-05", "text-generation"),
    HfModel("01-ai/Yi-1.5-34B", "01-ai", 410_000, 1_640,
            "transformers", ["yi", "chat"], "2025-07-20", "text-generation"),
    HfModel("NousResearch/Hermes-3-Llama-3.1-405B", "NousResearch", 180_000, 980,
            "transformers", ["llama", "chat", "405b"], "2025-09-01", "text-generation"),
    HfModel("CohereForAI/c4ai-command-r-plus-08-2024", "CohereForAI", 320_000, 1_210,
            "transformers", ["command-r", "rag"], "2024-08-01", "text-generation"),
    HfModel("tiiuae/falcon-180B", "tiiuae", 96_000, 710,
            "transformers", ["falcon"], "2023-09-06", "text-generation"),
    HfModel("Snowflake/snowflake-arctic-instruct", "Snowflake", 41_000, 410,
            "transformers", ["arctic", "moe"], "2024-04-24", "text-generation"),
    HfModel("databricks/dbrx-instruct", "databricks", 72_000, 840,
            "transformers", ["dbrx", "moe"], "2024-03-27", "text-generation"),
    HfModel("ibm-granite/granite-3.0-8b-instruct", "ibm-granite", 290_000, 720,
            "transformers", ["granite", "chat"], "2025-10-20", "text-generation"),
]


def _search_mock(query: str, library: Optional[str], sort: str,
                 limit: int) -> HfSearchResult:
    q = (query or "").lower().strip()
    rows = list(_QUICK_PICKS)
    if q:
        rows = [m for m in rows
                if q in m.id.lower()
                or q in m.author.lower()
                or any(q in t for t in m.tags)]
    if library:
        rows = [m for m in rows if m.library == library]
    if sort == "downloads":
        rows.sort(key=lambda m: m.downloads, reverse=True)
    elif sort == "likes":
        rows.sort(key=lambda m: m.likes, reverse=True)
    elif sort == "updated":
        rows.sort(key=lambda m: m.last_modified, reverse=True)
    return HfSearchResult(
        models=rows[:limit],
        total=len(rows),
        rate_limit_remaining=None,
        rate_limited=False,
        next_retry_after_s=None,
    )


def search_hf(
    query: str,
    library: Optional[str] = None,
    sort: str = "downloads",
    limit: int = 25,
    token: Optional[str] = None,
) -> HfSearchResult:
    """Search HuggingFace Hub. Falls back to curated quick-picks when sibling
    FFI isn't built yet."""
    if _has_search():
        res = _search_real(query, library, sort, limit, token)
        if res is not None:
            return res
    # Fallback
    return _search_mock(query, library, sort, limit)


def quick_picks() -> List[HfModel]:
    return list(_QUICK_PICKS)


# =============================================================================
# Predict / what-if
# =============================================================================

# Technique -> (weights_mult, kv_mult, prefill_mult, runtime_mult, citation)
_TECHNIQUES = {
    "INT4": (0.25, 1.00, 1.00, 1.00,
             TransformationCitation("INT4", "GPTQ: Post-Training Quantization",
                                    "https://arxiv.org/abs/2210.17323", "2210.17323")),
    "INT8": (0.50, 1.00, 1.00, 1.00,
             TransformationCitation("INT8", "LLM.int8() 8-bit Matrix Multiplication",
                                    "https://arxiv.org/abs/2208.07339", "2208.07339")),
    "KV-FP8": (1.00, 0.50, 1.00, 1.00,
               TransformationCitation("KV-FP8", "FP8 Formats for Deep Learning",
                                      "https://arxiv.org/abs/2209.05433", "2209.05433")),
    "KV-INT4": (1.00, 0.25, 1.00, 1.00,
                TransformationCitation("KV-INT4", "KIVI: A Tuning-Free Asymmetric 2bit KV Cache",
                                       "https://arxiv.org/abs/2402.02750", "2402.02750")),
    "LoRA": (1.02, 1.00, 1.00, 1.05,
             TransformationCitation("LoRA", "Low-Rank Adaptation of LLMs",
                                    "https://arxiv.org/abs/2106.09685", "2106.09685")),
    "REAP": (0.70, 1.00, 1.00, 1.00,
             TransformationCitation("REAP", "REAP: Pruning via Layerwise Reconstruction",
                                    "https://arxiv.org/abs/2310.06694", "2310.06694")),
    "SpecDecode": (1.08, 1.00, 1.00, 1.15,
                   TransformationCitation("SpecDecode", "Speculative Decoding",
                                          "https://arxiv.org/abs/2211.17192", "2211.17192")),
    "FlashAttn3": (1.00, 1.00, 0.80, 1.00,
                   TransformationCitation("FlashAttn3", "FlashAttention-3",
                                          "https://arxiv.org/abs/2407.08608", "2407.08608")),
}


def list_techniques() -> List[str]:
    return list(_TECHNIQUES.keys())


def _predict_real(baseline: PredictBreakdown,
                  techniques: List[str]) -> Optional[WhatIfResult]:
    # Real sibling FFI (crates/hwledger-ffi):
    #   extern "C" fn hwledger_predict(
    #       baseline_config_json:  *const c_char,
    #       candidate_config_json: *const c_char,
    #       techniques_json:       *const c_char,
    #       workload_json:         *const c_char,
    #   ) -> *mut c_char;  // freed via hwledger_predict_free
    try:
        from lib.ffi import predict as _ffi_predict  # local import avoids cycle
        # The real predict FFI expects model configs, not raw memory bands, so
        # we hand it the last-planned config on both sides and a default
        # workload. The FFI returns a Prediction dict; we marshal it back into
        # our WhatIfResult shape.
        latest = None
        try:
            import streamlit as st
            latest = st.session_state.get("latest_plan")
        except Exception:
            pass
        if not latest:
            return None
        payload = _ffi_predict(
            baseline_config_json=latest["config_json"],
            candidate_config_json=latest["config_json"],
            techniques=techniques,
            prefill_tokens=latest.get("seq_len", 4096),
            decode_tokens=256,
            batch=latest.get("batch_size", 1),
            seq_len=latest.get("seq_len", 4096),
        )
        if payload is None:
            return None
        if isinstance(payload, dict) and "error" in payload:
            return None
        # Real Prediction shape from crates/hwledger-predict:
        #   { "baseline": {...bytes fields...}, "candidate": {...},
        #     "techniques": [...], "citations": [...], "verdict": "...",
        #     "delta_pct": float }
        # Field names may use *_bytes — normalise to our *_mb shape.
        def _as_bd(d: dict) -> PredictBreakdown:
            def mb(v: float | int, is_bytes: bool) -> float:
                return float(v) / (1024 * 1024) if is_bytes else float(v)
            w = d.get("weights_mb", None)
            if w is None:
                w = mb(d.get("weights_bytes", 0), True)
            k = d.get("kv_mb", None)
            if k is None:
                k = mb(d.get("kv_bytes", 0), True)
            p = d.get("prefill_mb", None)
            if p is None:
                p = mb(d.get("prefill_activation_bytes", d.get("prefill_bytes", 0)), True)
            r = d.get("runtime_mb", None)
            if r is None:
                r = mb(d.get("runtime_overhead_bytes", d.get("runtime_bytes", 0)), True)
            return PredictBreakdown(w, k, p, r)

        base_d = payload.get("baseline") or payload.get("baseline_plan") or {}
        cand_d = payload.get("candidate") or payload.get("candidate_plan") or {}
        return WhatIfResult(
            baseline=_as_bd(base_d) if base_d else baseline,
            candidate=_as_bd(cand_d) if cand_d else baseline,
            techniques=payload.get("techniques", techniques),
            verdict=payload.get("verdict", payload.get("summary", "")),
            delta_pct=float(payload.get("delta_pct", payload.get("delta_percent", 0.0))),
            citations=[TransformationCitation(
                technique=c.get("technique", c.get("name", "")),
                title=c.get("title", ""),
                url=c.get("url", ""),
                arxiv_id=c.get("arxiv_id"),
            ) for c in payload.get("citations", [])],
        )
    except Exception:
        return None


def _predict_mock(baseline: PredictBreakdown,
                  techniques: List[str]) -> WhatIfResult:
    w, k, p, r = baseline.weights_mb, baseline.kv_mb, baseline.prefill_mb, baseline.runtime_mb
    citations: List[TransformationCitation] = []
    for t in techniques:
        spec = _TECHNIQUES.get(t)
        if spec is None:
            continue
        wm, km, pm, rm, cite = spec
        w *= wm
        k *= km
        p *= pm
        r *= rm
        citations.append(cite)
    candidate = PredictBreakdown(w, k, p, r)
    delta = ((candidate.total_mb - baseline.total_mb) / max(1.0, baseline.total_mb)) * 100
    if delta <= -25:
        verdict = f"Transformative: {abs(delta):.1f}% reduction enables significantly smaller target hardware."
    elif delta <= -10:
        verdict = f"Meaningful: {abs(delta):.1f}% reduction relieves VRAM pressure."
    elif delta <= 0:
        verdict = f"Marginal: {abs(delta):.1f}% reduction; evaluate if latency/quality trade-offs justify."
    else:
        verdict = f"Regression: +{delta:.1f}%; chosen techniques grow the footprint."
    return WhatIfResult(baseline=baseline, candidate=candidate,
                        techniques=techniques, verdict=verdict,
                        delta_pct=delta, citations=citations)


def whatif(baseline: PredictBreakdown, techniques: List[str]) -> WhatIfResult:
    if _has_predict():
        real = _predict_real(baseline, techniques)
        if real is not None:
            return real
    return _predict_mock(baseline, techniques)


# =============================================================================
# Backend status for banners
# =============================================================================

@dataclass
class BackendStatus:
    ffi_core: bool
    ffi_search: bool
    ffi_predict: bool

    @property
    def any_mocked(self) -> bool:
        return not (self.ffi_search and self.ffi_predict)


def backend_status() -> BackendStatus:
    return BackendStatus(
        ffi_core=lib is not None,
        ffi_search=_has_search(),
        ffi_predict=_has_predict(),
    )
