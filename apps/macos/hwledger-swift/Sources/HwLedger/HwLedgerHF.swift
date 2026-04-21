import Foundation

// MARK: - HuggingFace + Predictor Swift FFI Bridge
//
// These types mirror the JSON payloads that `hwledger_hf_search` and
// `hwledger_predict` will return once the sibling agent wires them up.
// Today the underlying FFI functions are NOT yet exported from the Rust
// core, so the live-call paths raise `fatalError("TODO: wire FFI")`.
// Pure JSON decoders are fully exercised by the Swift unit tests so we
// can validate the contract before the Rust side lands.

// MARK: - ModelCard (HF search result)

public struct ModelCard: Codable, Identifiable, Hashable {
    public let repoId: String
    public let displayName: String?
    public let paramCount: UInt64?
    public let downloads: UInt64?
    public let lastModified: String?
    public let pipelineTag: String?
    public let library: String?
    public let tags: [String]
    public let trending: Double?
    public let configJson: String?

    public var id: String { repoId }

    public init(
        repoId: String,
        displayName: String? = nil,
        paramCount: UInt64? = nil,
        downloads: UInt64? = nil,
        lastModified: String? = nil,
        pipelineTag: String? = nil,
        library: String? = nil,
        tags: [String] = [],
        trending: Double? = nil,
        configJson: String? = nil
    ) {
        self.repoId = repoId
        self.displayName = displayName
        self.paramCount = paramCount
        self.downloads = downloads
        self.lastModified = lastModified
        self.pipelineTag = pipelineTag
        self.library = library
        self.tags = tags
        self.trending = trending
        self.configJson = configJson
    }

    private enum CodingKeys: String, CodingKey {
        case repoId = "repo_id"
        case displayName = "display_name"
        case paramCount = "param_count"
        case downloads
        case lastModified = "last_modified"
        case pipelineTag = "pipeline_tag"
        case library
        case tags
        case trending
        case configJson = "config_json"
    }
}

public struct HfSearchResponse: Codable {
    public let models: [ModelCard]
    public let rateLimited: Bool
    public let nextCursor: String?

    public init(models: [ModelCard], rateLimited: Bool, nextCursor: String?) {
        self.models = models
        self.rateLimited = rateLimited
        self.nextCursor = nextCursor
    }

    private enum CodingKeys: String, CodingKey {
        case models
        case rateLimited = "rate_limited"
        case nextCursor = "next_cursor"
    }
}

// MARK: - Prediction (What-If)

public enum CompressionTechnique: String, Codable, Hashable, CaseIterable, Identifiable {
    case int4 = "INT4"
    case int8 = "INT8"
    case fp8 = "FP8"
    case lora = "LoRA"
    case qlora = "QLoRA"
    case reap = "REAP"
    case specDecode = "SpecDecode"
    case pagedAttention = "PagedAttention"
    case flashAttention3 = "FlashAttention-3"
    case kvInt8 = "KVInt8"
    case tp2 = "TP=2"
    case tp4 = "TP=4"
    case tp8 = "TP=8"

    public var id: String { rawValue }
}

public enum TransformationVerdict: String, Codable, Hashable {
    case pureConfigSwap = "pure_config_swap"
    case loraRequired = "lora_required"
    case fullFineTuneRequired = "full_finetune_required"
    case incompatible = "incompatible"

    public var humanReadable: String {
        switch self {
        case .pureConfigSwap: return "Pure config swap"
        case .loraRequired: return "LoRA required"
        case .fullFineTuneRequired: return "Full fine-tune required"
        case .incompatible: return "Incompatible"
        }
    }
}

public struct ConfidenceInterval: Codable, Hashable {
    public let value: Double
    public let low: Double
    public let high: Double

    public init(value: Double, low: Double, high: Double) {
        self.value = value
        self.low = low
        self.high = high
    }
}

public struct ModelMemoryBreakdown: Codable, Hashable {
    public let weightsBytes: UInt64
    public let kvBytes: UInt64
    public let prefillBytes: UInt64
    public let runtimeBytes: UInt64
    public let totalBytes: UInt64

    public init(
        weightsBytes: UInt64,
        kvBytes: UInt64,
        prefillBytes: UInt64,
        runtimeBytes: UInt64,
        totalBytes: UInt64
    ) {
        self.weightsBytes = weightsBytes
        self.kvBytes = kvBytes
        self.prefillBytes = prefillBytes
        self.runtimeBytes = runtimeBytes
        self.totalBytes = totalBytes
    }

    private enum CodingKeys: String, CodingKey {
        case weightsBytes = "weights_bytes"
        case kvBytes = "kv_bytes"
        case prefillBytes = "prefill_bytes"
        case runtimeBytes = "runtime_bytes"
        case totalBytes = "total_bytes"
    }
}

public struct Citation: Codable, Hashable, Identifiable {
    public let id: String
    public let title: String
    public let url: String?
    public let metric: String?

    public init(id: String, title: String, url: String? = nil, metric: String? = nil) {
        self.id = id
        self.title = title
        self.url = url
        self.metric = metric
    }
}

public struct TransformationDetails: Codable, Hashable {
    public let verdict: TransformationVerdict
    public let loraRank: UInt32?
    public let estimatedGpuHours: Double?
    public let rationale: String?

    public init(
        verdict: TransformationVerdict,
        loraRank: UInt32? = nil,
        estimatedGpuHours: Double? = nil,
        rationale: String? = nil
    ) {
        self.verdict = verdict
        self.loraRank = loraRank
        self.estimatedGpuHours = estimatedGpuHours
        self.rationale = rationale
    }

    private enum CodingKeys: String, CodingKey {
        case verdict
        case loraRank = "lora_rank"
        case estimatedGpuHours = "estimated_gpu_hours"
        case rationale
    }
}

public struct Prediction: Codable {
    public let baseline: ModelMemoryBreakdown
    public let candidate: ModelMemoryBreakdown
    public let decodeTps: ConfidenceInterval
    public let ttftMs: ConfidenceInterval
    public let throughput: ConfidenceInterval
    public let transformation: TransformationDetails
    public let citations: [Citation]

    public init(
        baseline: ModelMemoryBreakdown,
        candidate: ModelMemoryBreakdown,
        decodeTps: ConfidenceInterval,
        ttftMs: ConfidenceInterval,
        throughput: ConfidenceInterval,
        transformation: TransformationDetails,
        citations: [Citation]
    ) {
        self.baseline = baseline
        self.candidate = candidate
        self.decodeTps = decodeTps
        self.ttftMs = ttftMs
        self.throughput = throughput
        self.transformation = transformation
        self.citations = citations
    }

    private enum CodingKeys: String, CodingKey {
        case baseline
        case candidate
        case decodeTps = "decode_tps"
        case ttftMs = "ttft_ms"
        case throughput
        case transformation
        case citations
    }
}

// MARK: - Resolver (Planner model input)

/// Result of resolving a user-supplied Planner model input string through
/// `hwledger_resolve_model`. Mirrors the four JSON shapes that the Rust
/// resolver emits.
///
/// Traces to: FR-HF-001, FR-PLAN-003
public enum ResolvedModelSource: Equatable {
    case hfRepo(repoId: String, revision: String?)
    case goldenFixture(path: URL)
    case localConfig(path: URL)
    case ambiguous(hint: String, candidates: [ModelCard])

    /// Human-readable identifier for a resolved (non-ambiguous) source.
    /// Returns `nil` for `.ambiguous`.
    public var resolvedId: String? {
        switch self {
        case let .hfRepo(repoId, _): return repoId
        case let .goldenFixture(path): return path.lastPathComponent
        case let .localConfig(path): return path.lastPathComponent
        case .ambiguous: return nil
        }
    }

    /// Whether this resolution is unambiguous (safe to proceed to Plan).
    public var isResolved: Bool {
        if case .ambiguous = self { return false }
        return true
    }
}

// MARK: - Workload Inputs

public struct WhatIfWorkload: Codable {
    public let seqLen: UInt64
    public let batchSize: UInt32
    public let prefillTokens: UInt64
    public let decodeTokens: UInt64

    public init(seqLen: UInt64, batchSize: UInt32, prefillTokens: UInt64, decodeTokens: UInt64) {
        self.seqLen = seqLen
        self.batchSize = batchSize
        self.prefillTokens = prefillTokens
        self.decodeTokens = decodeTokens
    }

    private enum CodingKeys: String, CodingKey {
        case seqLen = "seq_len"
        case batchSize = "batch_size"
        case prefillTokens = "prefill_tokens"
        case decodeTokens = "decode_tokens"
    }
}

// MARK: - Public async FFI façade

extension HwLedger {
    /// Search HuggingFace for model cards matching a query.
    ///
    /// - Parameters:
    ///   - query: free-text search query
    ///   - library: optional library filter (gguf / transformers / mlx / vllm)
    ///   - pipelineTag: optional pipeline-tag filter (text-generation, etc.)
    ///   - sort: downloads / trending / recent
    /// - Returns: HfSearchResponse with models and rate-limit flag
    public static func searchHf(
        query: String,
        library: String? = nil,
        pipelineTag: String? = nil,
        sort: String = "downloads"
    ) async throws -> HfSearchResponse {
        // The FFI function `hwledger_hf_search` is not yet exported from
        // the Rust core. Decoding contract is already tested; real wiring
        // lands when the sibling PR merges.
        //
        // TODO: wire FFI — call `hwledger_hf_search(query, library, pipeline_tag, sort)`,
        // take the returned `*const c_char`, copy to Swift String, decode JSON to
        // HfSearchResponse, free the buffer with `hwledger_string_free`.
        fatalError("TODO: wire FFI — hwledger_hf_search not yet exported")
    }

    /// Run a plan for an HF repo-id (fetches config via HF client then plans).
    public static func planHf(
        repoId: String,
        seqLen: UInt64,
        concurrentUsers: UInt32
    ) async throws -> PlannerResult {
        // TODO: wire FFI — call `hwledger_plan_hf(repo_id, seq_len, users)`.
        fatalError("TODO: wire FFI — hwledger_plan_hf not yet exported")
    }

    /// Run a What-If prediction: baseline vs candidate with compression stack.
    public static func predict(
        baseline: String,
        candidate: String,
        techniques: [CompressionTechnique],
        workload: WhatIfWorkload
    ) async throws -> Prediction {
        let techJson = try String(
            data: JSONEncoder().encode(techniques.map { $0.rawValue }),
            encoding: .utf8
        ) ?? "[]"
        let workloadJson = try String(
            data: JSONEncoder().encode(workload),
            encoding: .utf8
        ) ?? "{}"
        return try await Task.detached(priority: .userInitiated) {
            try baseline.withCString { bp in
                try candidate.withCString { cp in
                    try techJson.withCString { tp in
                        try workloadJson.withCString { wp in
                            guard let raw = hwledger_predict_whatif_ffi(bp, cp, tp, wp) else {
                                throw HwLedgerError.invalidData("hwledger_predict_whatif returned null")
                            }
                            defer { hwledger_predict_free_ffi(raw) }
                            let json = String(cString: raw)
                            return try decodePrediction(json: json)
                        }
                    }
                }
            }
        }.value
    }

    // MARK: - Decoder helpers (testable, no FFI required)

    /// Decode a raw JSON payload as returned by `hwledger_hf_search`.
    /// Separated from the async call so unit tests can cover the wire
    /// contract without invoking the unimplemented FFI.
    public static func decodeHfSearchResponse(json: String) throws -> HfSearchResponse {
        guard let data = json.data(using: .utf8) else {
            throw HwLedgerError.invalidData("hf_search payload is not UTF-8")
        }
        let decoder = JSONDecoder()
        do {
            return try decoder.decode(HfSearchResponse.self, from: data)
        } catch {
            throw HwLedgerError.invalidData("hf_search decode failed: \(error)")
        }
    }

    /// Resolve a Planner model input string (free text, `org/repo`, HF URL,
    /// `gold:<name>`, or absolute `.json` path) into a structured source.
    ///
    /// Calls `hwledger_resolve_model` on the Rust FFI and decodes the JSON
    /// response. When `token` is non-nil the resolver will use it as the HF
    /// bearer token for the ambiguous-search fallback.
    ///
    /// Traces to: FR-HF-001, FR-PLAN-003
    public static func resolveModel(input: String, token: String? = nil) async throws -> ResolvedModelSource {
        let trimmed = input.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            throw HwLedgerError.invalidInput("resolve input must not be empty")
        }

        return try await Task.detached(priority: .userInitiated) { () -> ResolvedModelSource in
            let inputCStr = strdup(trimmed)
            defer { free(inputCStr) }
            let tokenCStr: UnsafeMutablePointer<Int8>? = token.flatMap { strdup($0) }
            defer { if let t = tokenCStr { free(t) } }

            guard let ptr = hwledger_resolve_model(inputCStr, tokenCStr) else {
                throw HwLedgerError.runtimeError("hwledger_resolve_model returned null")
            }
            defer { hwledger_hf_free_string(ptr) }

            let json = String(cString: ptr)
            return try HwLedger.decodeResolvedModel(json: json)
        }.value
    }

    /// Decode a raw JSON payload as returned by `hwledger_resolve_model`.
    /// Separated from the async call so unit tests can cover the four-variant
    /// wire contract without invoking the FFI.
    public static func decodeResolvedModel(json: String) throws -> ResolvedModelSource {
        guard let data = json.data(using: .utf8) else {
            throw HwLedgerError.invalidData("resolve_model payload is not UTF-8")
        }
        guard let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            throw HwLedgerError.invalidData("resolve_model payload is not a JSON object")
        }

        // `error_json` returns `{"error": "..."}` on failure — surface it.
        if let err = obj["error"] as? String {
            throw HwLedgerError.runtimeError("resolve_model: \(err)")
        }

        guard let kind = obj["kind"] as? String else {
            throw HwLedgerError.invalidData("resolve_model payload missing \"kind\"")
        }

        switch kind {
        case "hf_repo":
            guard let repoId = obj["repo_id"] as? String else {
                throw HwLedgerError.invalidData("hf_repo missing repo_id")
            }
            let revision = obj["revision"] as? String
            return .hfRepo(repoId: repoId, revision: revision)

        case "golden_fixture":
            guard let path = obj["path"] as? String else {
                throw HwLedgerError.invalidData("golden_fixture missing path")
            }
            return .goldenFixture(path: URL(fileURLWithPath: path))

        case "local_config":
            guard let path = obj["path"] as? String else {
                throw HwLedgerError.invalidData("local_config missing path")
            }
            return .localConfig(path: URL(fileURLWithPath: path))

        case "ambiguous":
            let hint = (obj["hint"] as? String) ?? ""
            let candidatesRaw = obj["candidates"] ?? []
            let candidatesData = try JSONSerialization.data(withJSONObject: candidatesRaw)
            let candidates: [ModelCard]
            do {
                candidates = try JSONDecoder().decode([ModelCard].self, from: candidatesData)
            } catch {
                // Rust `HfModelSummary` may use a slightly different key shape;
                // be lenient rather than failing the resolution entirely.
                candidates = []
            }
            return .ambiguous(hint: hint, candidates: candidates)

        default:
            throw HwLedgerError.invalidData("resolve_model unknown kind: \(kind)")
        }
    }

    /// Decode a raw JSON payload as returned by `hwledger_predict`.
    public static func decodePrediction(json: String) throws -> Prediction {
        guard let data = json.data(using: .utf8) else {
            throw HwLedgerError.invalidData("predict payload is not UTF-8")
        }
        let decoder = JSONDecoder()
        do {
            return try decoder.decode(Prediction.self, from: data)
        } catch {
            throw HwLedgerError.invalidData("predict decode failed: \(error)")
        }
    }
}

// MARK: - Resolver C FFI Declarations

@_silgen_name("hwledger_resolve_model")
internal func hwledger_resolve_model(
    _ input: UnsafePointer<Int8>?,
    _ token: UnsafePointer<Int8>?
) -> UnsafeMutablePointer<Int8>?

@_silgen_name("hwledger_hf_free_string")
internal func hwledger_hf_free_string(_ ptr: UnsafeMutablePointer<Int8>?)

@_silgen_name("hwledger_predict_whatif")
internal func hwledger_predict_whatif_ffi(
    _ baselineConfigJson: UnsafePointer<Int8>?,
    _ candidateConfigJson: UnsafePointer<Int8>?,
    _ techniquesJson: UnsafePointer<Int8>?,
    _ workloadJson: UnsafePointer<Int8>?
) -> UnsafeMutablePointer<Int8>?

@_silgen_name("hwledger_predict_free")
internal func hwledger_predict_free_ffi(_ ptr: UnsafeMutablePointer<Int8>?)
