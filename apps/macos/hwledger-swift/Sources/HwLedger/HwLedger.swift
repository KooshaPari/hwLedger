import Foundation

// MARK: - Re-exported C Types
//
// These types mirror the C repr(C) types from hwledger-ffi.
// They serve as Swift-friendly wrappers over the raw C pointers.

/// Quantization mode for KV cache.
public enum KvQuantization: Int32 {
    case fp16 = 0
    case fp8 = 1
    case int8 = 2
    case int4 = 3
    case threeBit = 4
}

/// Quantization mode for model weights.
public enum WeightQuantization: Int32 {
    case fp16 = 0
    case bf16 = 1
    case int8 = 2
    case int4 = 3
    case threeBit = 4
}

/// Memory planning result.
public struct PlannerResult {
    public let weightsBytes: UInt64
    public let kvBytes: UInt64
    public let prefillActivationBytes: UInt64
    public let runtimeOverheadBytes: UInt64
    public let totalBytes: UInt64
    public let attentionKindLabel: String
    public let effectiveBatch: UInt32

    internal init(from cResult: hwledger_PlannerResult) throws {
        guard let labelPtr = cResult.attention_kind_label else {
            throw HwLedgerError.invalidData("attention_kind_label is null")
        }

        self.weightsBytes = cResult.weights_bytes
        self.kvBytes = cResult.kv_bytes
        self.prefillActivationBytes = cResult.prefill_activation_bytes
        self.runtimeOverheadBytes = cResult.runtime_overhead_bytes
        self.totalBytes = cResult.total_bytes
        self.attentionKindLabel = String(cString: labelPtr)
        self.effectiveBatch = cResult.effective_batch
    }
}

/// Detected GPU device.
public struct DeviceInfo {
    public let id: UInt32
    public let backend: String
    public let name: String
    public let uuid: String
    public let totalVramBytes: UInt64

    internal init(from cDevice: hwledger_DeviceInfo) throws {
        guard let backendPtr = cDevice.backend, let namePtr = cDevice.name else {
            throw HwLedgerError.invalidData("device backend or name is null")
        }

        self.id = cDevice.id
        self.backend = String(cString: backendPtr)
        self.name = String(cString: namePtr)
        if let uuidPtr = cDevice.uuid {
            self.uuid = String(cString: uuidPtr)
        } else {
            self.uuid = ""
        }
        self.totalVramBytes = cDevice.total_vram_bytes
    }
}

/// Single telemetry sample for a device.
public struct TelemetrySample {
    public let deviceId: UInt32
    public let freeVramBytes: UInt64
    public let utilizationPercent: Float
    public let temperatureCelsius: Float
    public let powerWatts: Float
    public let capturedAtMs: UInt64

    internal init(from cSample: hwledger_TelemetrySample) {
        self.deviceId = cSample.device_id
        self.freeVramBytes = cSample.free_vram_bytes
        self.utilizationPercent = cSample.util_percent
        self.temperatureCelsius = cSample.temperature_c
        self.powerWatts = cSample.power_watts
        self.capturedAtMs = cSample.captured_at_ms
    }
}

/// Input to the memory planner.
internal struct CPlannerInput {
    var configJson: UnsafeMutablePointer<Int8>
    var seqLen: UInt64
    var concurrentUsers: UInt32
    var batchSize: UInt32
    var kvQuant: UInt8
    var weightQuant: UInt8
}

// MARK: - Error Handling

/// HwLedger FFI error.
public enum HwLedgerError: Error, Equatable {
    case classifyError(String)
    case ingestError(String)
    case probeError(String)
    case runtimeError(String)
    case invalidInput(String)
    case invalidData(String)
    case unknown(String)

    public static func == (lhs: HwLedgerError, rhs: HwLedgerError) -> Bool {
        switch (lhs, rhs) {
        case let (.classifyError(a), .classifyError(b)):
            return a == b
        case let (.ingestError(a), .ingestError(b)):
            return a == b
        case let (.probeError(a), .probeError(b)):
            return a == b
        case let (.runtimeError(a), .runtimeError(b)):
            return a == b
        case let (.invalidInput(a), .invalidInput(b)):
            return a == b
        case let (.invalidData(a), .invalidData(b)):
            return a == b
        case let (.unknown(a), .unknown(b)):
            return a == b
        default:
            return false
        }
    }
}

// MARK: - Main HwLedger API

/// Public Swift API for hwLedger.
///
/// This is a thin wrapper over the C FFI surface. All methods are static.
/// Memory management is handled transparently via automatic freeing in deinit patterns.
public struct HwLedger {
    /// Get the FFI crate version.
    public static func coreVersion() -> String {
        let versionPtr = hwledger_core_version()
        return String(cString: versionPtr)
    }

    /// Plan memory requirements for a model.
    ///
    /// - Parameters:
    ///   - configJson: JSON string with model config (from HF, GGUF, etc.)
    ///   - seqLen: sequence length
    ///   - concurrentUsers: number of concurrent inference requests
    ///   - batchSize: inference batch size
    ///   - kvQuantization: KV cache quantization mode
    ///   - weightQuantization: model weight quantization mode
    /// - Returns: PlannerResult with memory breakdown
    /// - Throws: HwLedgerError if planning fails
    public static func plan(
        configJson: String,
        seqLen: UInt64,
        concurrentUsers: UInt32,
        batchSize: UInt32,
        kvQuantization: KvQuantization = .fp16,
        weightQuantization: WeightQuantization = .fp16
    ) throws -> PlannerResult {
        let configJsonCStr = UnsafeMutablePointer<Int8>(
            mutating: (configJson as NSString).utf8String!
        )

        var cInput = hwledger_PlannerInput(
            config_json: configJsonCStr,
            seq_len: seqLen,
            concurrent_users: concurrentUsers,
            batch_size: batchSize,
            kv_quant: UInt8(kvQuantization.rawValue),
            weight_quant: UInt8(weightQuantization.rawValue)
        )

        guard let resultPtr = hwledger_plan(&cInput) else {
            throw HwLedgerError.invalidInput("plan failed: invalid configuration")
        }

        defer {
            hwledger_plan_free(resultPtr)
        }

        return try PlannerResult(from: resultPtr.pointee)
    }

    /// Detect all available GPU devices.
    ///
    /// - Returns: Array of detected DeviceInfo
    /// - Throws: HwLedgerError if detection fails
    public static func detectDevices() throws -> [DeviceInfo] {
        var count: UInt = 0
        guard let devicesPtr = hwledger_probe_detect(&count) else {
            // Empty list is not an error; systems may have no GPUs
            return []
        }

        defer {
            hwledger_probe_detect_free(devicesPtr, count)
        }

        let deviceSlice = UnsafeBufferPointer(start: devicesPtr, count: Int(count))
        var devices: [DeviceInfo] = []

        for cDevice in deviceSlice {
            devices.append(try DeviceInfo(from: cDevice))
        }

        return devices
    }

    /// Sample telemetry for a device.
    ///
    /// - Parameters:
    ///   - deviceId: GPU device ID
    ///   - backend: backend name (e.g., "nvidia", "metal", "amd")
    /// - Returns: TelemetrySample with current VRAM, utilization, temperature, power
    /// - Throws: HwLedgerError if sampling fails
    public static func sample(deviceId: UInt32, backend: String) throws -> TelemetrySample {
        let backendCStr = UnsafeMutablePointer<Int8>(
            mutating: (backend as NSString).utf8String!
        )

        guard let samplePtr = hwledger_probe_sample(deviceId, backendCStr) else {
            throw HwLedgerError.probeError(
                "telemetry sampling failed for device \(deviceId) on \(backend)"
            )
        }

        defer {
            hwledger_probe_sample_free(samplePtr)
        }

        return TelemetrySample(from: samplePtr.pointee)
    }
}

// MARK: - C FFI Declarations
//
// These are the raw C declarations imported from libhwledger_ffi.
// Do not use directly; use the HwLedger struct instead.

@_silgen_name("hwledger_plan")
internal func hwledger_plan(_ input: UnsafeMutablePointer<hwledger_PlannerInput>?) -> UnsafeMutablePointer<hwledger_PlannerResult>?

@_silgen_name("hwledger_plan_free")
internal func hwledger_plan_free(_ result: UnsafeMutablePointer<hwledger_PlannerResult>?)

@_silgen_name("hwledger_probe_detect")
internal func hwledger_probe_detect(_ outCount: UnsafeMutablePointer<UInt>?) -> UnsafeMutablePointer<hwledger_DeviceInfo>?

@_silgen_name("hwledger_probe_detect_free")
internal func hwledger_probe_detect_free(_ devices: UnsafeMutablePointer<hwledger_DeviceInfo>?, _ count: UInt)

@_silgen_name("hwledger_probe_sample")
internal func hwledger_probe_sample(_ deviceId: UInt32, _ backend: UnsafePointer<Int8>?) -> UnsafeMutablePointer<hwledger_TelemetrySample>?

@_silgen_name("hwledger_probe_sample_free")
internal func hwledger_probe_sample_free(_ sample: UnsafeMutablePointer<hwledger_TelemetrySample>?)

@_silgen_name("hwledger_core_version")
internal func hwledger_core_version() -> UnsafePointer<Int8>

// MARK: - C Type Declarations

internal struct hwledger_PlannerInput {
    var config_json: UnsafeMutablePointer<Int8>?
    var seq_len: UInt64
    var concurrent_users: UInt32
    var batch_size: UInt32
    var kv_quant: UInt8
    var weight_quant: UInt8
}

internal struct hwledger_PlannerResult {
    var weights_bytes: UInt64
    var kv_bytes: UInt64
    var prefill_activation_bytes: UInt64
    var runtime_overhead_bytes: UInt64
    var total_bytes: UInt64
    var attention_kind_label: UnsafeMutablePointer<Int8>?
    var effective_batch: UInt32
}

internal struct hwledger_DeviceInfo {
    var id: UInt32
    var backend: UnsafeMutablePointer<Int8>?
    var name: UnsafeMutablePointer<Int8>?
    var uuid: UnsafeMutablePointer<Int8>?
    var total_vram_bytes: UInt64
}

internal struct hwledger_TelemetrySample {
    var device_id: UInt32
    var free_vram_bytes: UInt64
    var util_percent: Float
    var temperature_c: Float
    var power_watts: Float
    var captured_at_ms: UInt64
}
