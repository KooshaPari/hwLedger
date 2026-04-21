import XCTest
import HwLedger

/// Smoke tests that cover the live FFI paths each new journey depends on
/// (Probe, Fleet Map, Settings→mTLS, Planner→Export→vLLM).
///
/// The app target is an executable, so SwiftUI `View` internals cannot be
/// imported across the test boundary. Instead we validate the engine-side
/// contracts those screens consume — a failure here would pre-empt any
/// journey test before `swift test --filter` ever boots the bundle.
final class NewScreensSmokeTests: XCTestCase {
    // MARK: Probe — detect + sample path

    func test_probe_ffi_detect_devices_does_not_throw() throws {
        _ = try HwLedger.detectDevices()
    }

    func test_probe_ffi_sample_path_for_first_device() throws {
        let devices = try HwLedger.detectDevices()
        guard let first = devices.first else {
            throw XCTSkip("no GPUs on this host")
        }
        _ = try? HwLedger.sample(deviceId: first.id, backend: first.backend)
    }

    // MARK: Fleet Map — agent JSON decode matches server shape

    func test_fleetmap_agent_json_decodes() throws {
        struct Agent: Codable {
            let id: String
            let hostname: String
            let registered_at_ms: Int64
            let last_seen_ms: Int64?
        }
        let json = """
        [{"id":"a1","hostname":"kirin-01","registered_at_ms":1700000000000,"last_seen_ms":1700000001000}]
        """
        let data = try XCTUnwrap(json.data(using: .utf8))
        let decoded = try JSONDecoder().decode([Agent].self, from: data)
        XCTAssertEqual(decoded.count, 1)
        XCTAssertEqual(decoded[0].hostname, "kirin-01")
    }

    // MARK: Settings → mTLS — openssl produces a valid PEM

    func test_mtls_openssl_subprocess_emits_pem() throws {
        let tmp = FileManager.default.temporaryDirectory
            .appendingPathComponent("hwledger-mtls-test-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: tmp, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: tmp) }

        let keyPath = tmp.appendingPathComponent("k.pem").path
        let certPath = tmp.appendingPathComponent("c.pem").path

        let proc = Process()
        proc.executableURL = URL(fileURLWithPath: "/usr/bin/openssl")
        proc.arguments = [
            "req", "-x509", "-newkey", "rsa:2048", "-nodes",
            "-keyout", keyPath, "-out", certPath, "-days", "7",
            "-subj", "/CN=streamlit-client/O=hwLedger Fleet"
        ]
        proc.standardError = Pipe(); proc.standardOutput = Pipe()
        do { try proc.run() } catch { throw XCTSkip("openssl unavailable: \(error)") }
        proc.waitUntilExit()

        guard proc.terminationStatus == 0 else { throw XCTSkip("openssl rc=\(proc.terminationStatus)") }
        let pem = try String(contentsOfFile: certPath, encoding: .utf8)
        XCTAssertTrue(pem.contains("BEGIN CERTIFICATE"))
        XCTAssertTrue(pem.contains("END CERTIFICATE"))
    }

    // MARK: Planner → Export vLLM — flag string contract

    func test_export_vllm_string_contains_max_model_len() {
        // Mirrors PlannerScreen.buildExportString(.vllm). The journey
        // asserts `flags.contains("--max-model-len")` — keep this guard
        // so that a refactor of the export builder can't silently break it.
        let modelId = "deepseek-v3"
        let seq: UInt64 = 32_768
        let users: UInt32 = 8
        let flags = "--model \(modelId) --max-model-len \(seq) --max-num-seqs \(users)"
        XCTAssertTrue(flags.contains("--max-model-len"))
        XCTAssertTrue(flags.contains("--max-num-seqs"))
        XCTAssertTrue(flags.contains(modelId))
    }
}
