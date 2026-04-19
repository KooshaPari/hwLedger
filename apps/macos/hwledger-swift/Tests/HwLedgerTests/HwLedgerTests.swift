import XCTest
@testable import HwLedger

final class HwLedgerTests: XCTestCase {
    /// Test that core version returns a non-empty string.
    /// Smoke test for basic FFI functionality.
    func testCoreVersion() throws {
        let version = HwLedger.coreVersion()
        XCTAssertFalse(version.isEmpty, "version should not be empty")
        XCTAssertTrue(version.count > 0, "version should be a valid string")
    }

    /// Test that device detection does not throw.
    /// Empty list (no GPUs) is acceptable on some systems.
    func testDetectDevicesDoesNotThrow() throws {
        let devices = try HwLedger.detectDevices()
        XCTAssertTrue(devices is [DeviceInfo], "should return DeviceInfo array")
        // Device list may be empty on some systems or when no GPUs are present
    }

    /// Test memory planning with a minimal DeepSeek-V3-like config.
    /// Traces to: FR-PLAN-003
    func testPlanDeepSeekV3() throws {
        let configJson = """
        {
          "model_type": "deepseek",
          "num_hidden_layers": 62,
          "hidden_size": 4096,
          "kv_lora_rank": 512,
          "qk_rope_head_dim": 64
        }
        """

        let result = try HwLedger.plan(
            configJson: configJson,
            seqLen: 4096,
            concurrentUsers: 2,
            batchSize: 1,
            kvQuantization: .fp16,
            weightQuantization: .fp16
        )

        XCTAssertGreaterThan(result.totalBytes, 0, "total_bytes should be > 0")
        XCTAssertGreaterThan(result.weightsBytes, 0, "weights_bytes should be > 0")
        XCTAssertEqual(result.effectiveBatch, 1, "effective_batch = min(1, 2) = 1")
        XCTAssertEqual(result.attentionKindLabel, "Mla", "should detect MLA for DeepSeek")
    }

    /// Test quantization modes for weight and KV cache.
    func testQuantizationModes() throws {
        let configJson = """
        {
          "model_type": "llama",
          "num_hidden_layers": 32,
          "hidden_size": 4096,
          "num_attention_heads": 32,
          "num_key_value_heads": 32
        }
        """

        // Test with different quantization modes
        let result_fp16_fp16 = try HwLedger.plan(
            configJson: configJson,
            seqLen: 2048,
            concurrentUsers: 1,
            batchSize: 1,
            kvQuantization: .fp16,
            weightQuantization: .fp16
        )

        let result_int4_int4 = try HwLedger.plan(
            configJson: configJson,
            seqLen: 2048,
            concurrentUsers: 1,
            batchSize: 1,
            kvQuantization: .int4,
            weightQuantization: .int4
        )

        // Int4 should use fewer bytes than FP16
        XCTAssertLessThan(
            result_int4_int4.totalBytes,
            result_fp16_fp16.totalBytes,
            "int4 should use less memory than fp16"
        )
    }

    /// Test that invalid JSON is handled gracefully.
    func testPlanInvalidJSON() {
        let invalidJson = "not valid json {{{{"

        XCTAssertThrowsError(
            try HwLedger.plan(
                configJson: invalidJson,
                seqLen: 1024,
                concurrentUsers: 1,
                batchSize: 1
            ),
            "should throw on invalid JSON"
        ) { error in
            if case HwLedgerError.invalidInput = error {
                // Expected
            } else {
                XCTFail("expected invalidInput error, got \(error)")
            }
        }
    }

    /// Test batch size clamping.
    func testEffectiveBatchClamping() throws {
        let configJson = """
        {
          "model_type": "qwen",
          "num_hidden_layers": 32,
          "hidden_size": 4096,
          "num_attention_heads": 32,
          "num_key_value_heads": 8
        }
        """

        let result = try HwLedger.plan(
            configJson: configJson,
            seqLen: 1024,
            concurrentUsers: 8,
            batchSize: 4
        )

        // effective_batch = min(batch_size, concurrent_users) = min(4, 8) = 4
        XCTAssertEqual(result.effectiveBatch, 4, "effective_batch should clamp to batch_size")
    }
}
