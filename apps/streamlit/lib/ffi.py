"""
ctypes FFI bindings to hwledger-ffi.dylib / .so

Provides high-level Python interface to C ABI functions:
- hwledger_plan() for memory planning
- hwledger_probe_detect() for device enumeration
- hwledger_probe_sample() for telemetry sampling
"""

import ctypes
import json
import os
import platform
from pathlib import Path
from dataclasses import dataclass
from typing import Optional, List


# Load the shared library
def _load_library() -> Optional[ctypes.CDLL]:
    """Load libhwledger_ffi from target/release."""
    system = platform.system()
    if system == "Darwin":
        libname = "libhwledger_ffi.dylib"
    elif system == "Linux":
        libname = "libhwledger_ffi.so"
    elif system == "Windows":
        libname = "hwledger_ffi.dll"
    else:
        return None

    # Try relative paths from this file
    base = Path(__file__).parent.parent.parent.parent
    candidates = [
        base / "target" / "release" / libname,
        Path.home() / ".cargo" / "target" / "release" / libname,
        Path("/usr/local/lib") / libname,
    ]

    for path in candidates:
        if path.exists():
            try:
                return ctypes.CDLL(str(path))
            except Exception:
                continue

    return None


lib = _load_library()


# =============================================================================
# C Struct Definitions (repr(C) matching Rust originals)
# =============================================================================

class KvQuant(ctypes.c_uint8):
    """Quantization for KV cache: 0=Fp16, 1=Fp8, 2=Int8, 3=Int4, 4=ThreeBit"""
    pass


class WeightQuant(ctypes.c_uint8):
    """Quantization for weights: 0=Fp16, 1=Bf16, 2=Int8, 3=Int4, 4=ThreeBit"""
    pass


class PlannerInput(ctypes.Structure):
    """Input to hwledger_plan()"""
    _fields_ = [
        ("config_json", ctypes.c_char_p),
        ("seq_len", ctypes.c_uint64),
        ("concurrent_users", ctypes.c_uint32),
        ("batch_size", ctypes.c_uint32),
        ("kv_quant", ctypes.c_uint8),
        ("weight_quant", ctypes.c_uint8),
    ]


class PlannerResult(ctypes.Structure):
    """Result from hwledger_plan()"""
    _fields_ = [
        ("weights_bytes", ctypes.c_uint64),
        ("kv_bytes", ctypes.c_uint64),
        ("prefill_activation_bytes", ctypes.c_uint64),
        ("runtime_overhead_bytes", ctypes.c_uint64),
        ("total_bytes", ctypes.c_uint64),
        ("attention_kind_label", ctypes.c_char_p),
        ("effective_batch", ctypes.c_uint32),
    ]


class DeviceInfo(ctypes.Structure):
    """Detected GPU device"""
    _fields_ = [
        ("id", ctypes.c_uint32),
        ("backend", ctypes.c_char_p),
        ("name", ctypes.c_char_p),
        ("uuid", ctypes.c_char_p),
        ("total_vram_bytes", ctypes.c_uint64),
    ]


class TelemetrySample(ctypes.Structure):
    """Single telemetry sample for a device"""
    _fields_ = [
        ("device_id", ctypes.c_uint32),
        ("free_vram_bytes", ctypes.c_uint64),
        ("util_percent", ctypes.c_float),
        ("temperature_c", ctypes.c_float),
        ("power_watts", ctypes.c_float),
        ("captured_at_ms", ctypes.c_uint64),
    ]


# =============================================================================
# Python Dataclasses (user-facing API)
# =============================================================================

@dataclass
class PlanResult:
    """Result of a memory plan"""
    weights_mb: float
    kv_mb: float
    prefill_mb: float
    runtime_mb: float
    total_mb: float
    attention_kind: str
    effective_batch: int

    @property
    def total_gb(self) -> float:
        return self.total_mb / 1024


@dataclass
class Device:
    """Detected GPU device"""
    id: int
    backend: str
    name: str
    uuid: str
    vram_gb: float


@dataclass
class Telemetry:
    """Current device telemetry"""
    device_id: int
    free_vram_gb: float
    util_percent: float
    temperature_c: float
    power_watts: float
    captured_at_ms: int


# =============================================================================
# High-Level API
# =============================================================================

def plan(
    config_json: str,
    seq_len: int,
    concurrent_users: int,
    batch_size: int,
    kv_quant: int = 0,
    weight_quant: int = 0,
) -> Optional[PlanResult]:
    """
    Run memory planner via FFI.

    Args:
        config_json: Model config as JSON string
        seq_len: Sequence length (tokens)
        concurrent_users: Number of concurrent users
        batch_size: Batch size
        kv_quant: KV quantization (0=Fp16, 1=Fp8, 2=Int8, 3=Int4, 4=ThreeBit)
        weight_quant: Weight quantization (0=Fp16, 1=Bf16, 2=Int8, 3=Int4, 4=ThreeBit)

    Returns:
        PlanResult with breakdown, or None on error
    """
    if lib is None:
        return None

    lib.hwledger_plan.argtypes = [ctypes.POINTER(PlannerInput)]
    lib.hwledger_plan.restype = ctypes.POINTER(PlannerResult)
    lib.hwledger_plan_free.argtypes = [ctypes.POINTER(PlannerResult)]

    # Encode config as UTF-8 bytes
    config_bytes = config_json.encode('utf-8')

    input_struct = PlannerInput(
        config_json=config_bytes,
        seq_len=seq_len,
        concurrent_users=concurrent_users,
        batch_size=batch_size,
        kv_quant=kv_quant,
        weight_quant=weight_quant,
    )

    result_ptr = lib.hwledger_plan(ctypes.byref(input_struct))
    if result_ptr is None:
        return None

    # Read ALL fields BEFORE free — the Rust PlannerResult owns its
    # attention_kind_label CString; calling hwledger_plan_free drops it and
    # leaves the pointer dangling. Previously reading label after free gave
    # UnicodeDecodeError on garbage bytes (e.g. 0xb0 at position 0).
    result = result_ptr.contents
    weights_bytes = result.weights_bytes
    kv_bytes = result.kv_bytes
    prefill_bytes = result.prefill_activation_bytes
    runtime_bytes = result.runtime_overhead_bytes
    total_bytes = result.total_bytes
    effective_batch = result.effective_batch

    raw_label = result.attention_kind_label  # ctypes c_char_p -> bytes or None
    if raw_label is None:
        attention_label = "unknown"
    else:
        try:
            attention_label = raw_label.decode("utf-8")
        except UnicodeDecodeError:
            attention_label = "unknown"

    # Now safe to free — we've copied everything out.
    lib.hwledger_plan_free(result_ptr)

    return PlanResult(
        weights_mb=weights_bytes / (1024 * 1024),
        kv_mb=kv_bytes / (1024 * 1024),
        prefill_mb=prefill_bytes / (1024 * 1024),
        runtime_mb=runtime_bytes / (1024 * 1024),
        total_mb=total_bytes / (1024 * 1024),
        attention_kind=attention_label,
        effective_batch=effective_batch,
    )


def detect_devices() -> List[Device]:
    """
    Detect all GPU devices on the system.

    Returns:
        List of Device objects
    """
    if lib is None:
        return []

    lib.hwledger_probe_detect.argtypes = [ctypes.POINTER(ctypes.c_size_t)]
    lib.hwledger_probe_detect.restype = ctypes.POINTER(DeviceInfo)
    lib.hwledger_probe_detect_free.argtypes = [ctypes.POINTER(DeviceInfo), ctypes.c_size_t]

    count = ctypes.c_size_t(0)
    devices_ptr = lib.hwledger_probe_detect(ctypes.byref(count))

    if devices_ptr is None or count.value == 0:
        return []

    devices = []
    for i in range(count.value):
        dev = devices_ptr[i]
        devices.append(Device(
            id=dev.id,
            backend=dev.backend.decode('utf-8') if dev.backend else "unknown",
            name=dev.name.decode('utf-8') if dev.name else "unknown",
            uuid=dev.uuid.decode('utf-8') if dev.uuid else "",
            vram_gb=dev.total_vram_bytes / (1024 * 1024 * 1024),
        ))

    lib.hwledger_probe_detect_free(devices_ptr, count.value)
    return devices


def is_available() -> bool:
    """Check if FFI library is loaded and available."""
    return lib is not None
