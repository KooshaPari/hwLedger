import Foundation
import HwLedger
import Security

enum Screen: String, CaseIterable, Identifiable {
    case library = "Library"
    case planner = "Planner"
    case hfSearch = "HF Search"
    case whatIf = "What-If"
    case fleet = "Fleet"
    case run = "Run"
    case ledger = "Ledger"
    case settings = "Settings"

    var id: String { self.rawValue }
}

/// Model metadata for Library screen.
struct IngestedModelInfo: Identifiable, Codable {
    let id: String
    let name: String
    let source: String
    let paramCount: UInt64
    let quantization: String
    let configJson: String

    init(name: String, source: String, paramCount: UInt64, quantization: String, configJson: String) {
        self.id = UUID().uuidString
        self.name = name
        self.source = source
        self.paramCount = paramCount
        self.quantization = quantization
        self.configJson = configJson
    }
}

@Observable
final class AppState {
    var selectedScreen: Screen = .planner
    var devices: [DeviceInfo] = []
    var coreVersion: String = ""
    var errorMessage: String?

    // Model library
    var libraryModels: [IngestedModelInfo] = []
    var selectedModel: IngestedModelInfo?

    // Fleet & server config
    var serverUrl: String {
        didSet {
            UserDefaults.standard.set(serverUrl, forKey: "serverUrl")
        }
    }
    var bootstrapToken: String = ""  // Session-only, never persisted

    // HuggingFace integration
    private(set) var hfTokenSet: Bool = false

    /// HF repo-id pre-filled from the HF Search screen into Planner.
    var pendingPlannerRepoId: String?

    /// Baseline/candidate preselected from HF Search → What-If.
    var whatIfBaseline: ModelCard?
    var whatIfCandidate: ModelCard?

    // Logging
    var logLevel: String {
        didSet {
            UserDefaults.standard.set(logLevel, forKey: "logLevel")
        }
    }

    init() {
        // Load persisted settings
        self.serverUrl = UserDefaults.standard.string(forKey: "serverUrl") ?? "http://localhost:8080"
        self.logLevel = UserDefaults.standard.string(forKey: "logLevel") ?? "info"

        // Check Keychain for HF token
        self.hfTokenSet = loadHfTokenFromKeychain() != nil

        // Load bundled models
        self.libraryModels = Self.loadBundledModels()

        Task {
            await initializeAppState()
        }
    }

    private func initializeAppState() async {
        coreVersion = HwLedger.coreVersion()
        await refreshDevices()
    }

    func refreshDevices() async {
        do {
            devices = try HwLedger.detectDevices()
            errorMessage = nil
        } catch {
            errorMessage = "Failed to detect devices: \(error)"
            devices = []
        }
    }

    // MARK: - HuggingFace Token Management

    func setHfToken(_ token: String) {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: "hwledger-hf",
            kSecAttrService as String: "com.hwledger.app",
        ]

        SecItemDelete(query as CFDictionary)

        let item: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: "hwledger-hf",
            kSecAttrService as String: "com.hwledger.app",
            kSecValueData as String: token.data(using: .utf8) ?? Data(),
        ]

        SecItemAdd(item as CFDictionary, nil)
        self.hfTokenSet = true
    }

    func getHfToken() -> String? {
        loadHfTokenFromKeychain()
    }

    private func loadHfTokenFromKeychain() -> String? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: "hwledger-hf",
            kSecAttrService as String: "com.hwledger.app",
            kSecReturnData as String: true,
        ]

        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)

        guard status == errSecSuccess, let data = result as? Data else {
            return nil
        }

        return String(data: data, encoding: .utf8)
    }

    // MARK: - Cache management

    /// Clear the on-disk HF client cache (model cards, config.json blobs).
    /// Best-effort: logs any IO error into `errorMessage` but does not throw.
    func clearHfCache() {
        let fm = FileManager.default
        let root = (fm.urls(for: .cachesDirectory, in: .userDomainMask).first)?
            .appendingPathComponent("hwledger/hf")
        guard let dir = root else { return }
        do {
            if fm.fileExists(atPath: dir.path) {
                try fm.removeItem(at: dir)
            }
        } catch {
            errorMessage = "Failed to clear HF cache: \(error)"
        }
    }

    /// Clear the predictor benchmarks cache (papers-with-code / citations
    /// responses, local prediction memos).
    func clearPredictorCache() {
        let fm = FileManager.default
        let root = (fm.urls(for: .cachesDirectory, in: .userDomainMask).first)?
            .appendingPathComponent("hwledger/predict")
        guard let dir = root else { return }
        do {
            if fm.fileExists(atPath: dir.path) {
                try fm.removeItem(at: dir)
            }
        } catch {
            errorMessage = "Failed to clear predictor cache: \(error)"
        }
    }

    // MARK: - Library Models

    static func loadBundledModels() -> [IngestedModelInfo] {
        let bundled: [(String, String, UInt64, String)] = [
            ("Llama 3.1 8B", "Local GGUF", 8_000_000_000, "FP16"),
            ("Llama 3.1 70B", "Local GGUF", 70_000_000_000, "FP16"),
            ("Mistral 7B", "Local GGUF", 7_000_000_000, "Q4"),
            ("DeepSeek V3", "Local GGUF", 236_000_000_000, "FP16"),
            ("Qwen 2 7B", "Local GGUF", 7_000_000_000, "FP16"),
            ("Mixtral 8x7B", "Local GGUF", 56_000_000_000, "FP16"),
            ("Gemma 3 12B", "Local GGUF", 12_000_000_000, "FP16"),
            ("Mamba2 2.7B", "Local GGUF", 2_700_000_000, "FP16"),
            ("Jamba v0.1", "Local GGUF", 12_000_000_000, "FP16"),
            ("Phi 3.5 Mini", "Local GGUF", 3_800_000_000, "FP16"),
        ]

        let mockConfigs = [
            "llama": "{\"model_type\":\"llama\",\"num_hidden_layers\":32,\"hidden_size\":4096,\"num_attention_heads\":32,\"num_key_value_heads\":8}",
            "deepseek": "{\"model_type\":\"deepseek\",\"num_hidden_layers\":62,\"hidden_size\":4096,\"kv_lora_rank\":512,\"qk_rope_head_dim\":64}",
            "mistral": "{\"model_type\":\"mistral\",\"num_hidden_layers\":32,\"hidden_size\":4096,\"num_attention_heads\":32,\"num_key_value_heads\":8}",
            "mixtral": "{\"model_type\":\"mixtral\",\"num_hidden_layers\":32,\"hidden_size\":4096,\"num_local_experts\":8,\"num_experts_per_tok\":2}",
            "mamba": "{\"model_type\":\"mamba2\",\"num_hidden_layers\":12,\"hidden_size\":2560,\"state_size\":64}",
        ]

        return bundled.map { name, source, params, quant in
            let modelKey = name.lowercased().split(separator: " ").first.map(String.init) ?? "llama"
            let config = mockConfigs[modelKey] ?? "{}"
            return IngestedModelInfo(name: name, source: source, paramCount: params, quantization: quant, configJson: config)
        }
    }
}
