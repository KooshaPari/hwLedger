import XCTest
import HwLedger

// Traces to: FR-UI-002, FR-PLAN-001
final class LibraryScreenTests: XCTestCase {

    // MARK: - Test 1: HwLedger FFI supports model configs
    // Traces to: FR-PLAN-001
    func testHwLedgerSupportsModelConfigs() throws {
        let llama8bJson = """
        {
          "model_type": "llama",
          "num_hidden_layers": 32,
          "hidden_size": 4096,
          "num_attention_heads": 32,
          "num_key_value_heads": 8
        }
        """

        let result = try HwLedger.plan(
            configJson: llama8bJson,
            seqLen: 4096,
            concurrentUsers: 1,
            batchSize: 1
        )

        XCTAssertNotNil(result)
        XCTAssert(result.totalBytes > 0, "Llama model should have total bytes")
    }

    // MARK: - Test 2: Model architecture diversity
    // Traces to: FR-PLAN-001
    func testModelArchitectureDiversity() throws {
        let configs = [
            ("Llama", "{\"model_type\": \"llama\", \"num_hidden_layers\": 32, \"hidden_size\": 4096, \"num_attention_heads\": 32, \"num_key_value_heads\": 8}"),
            ("Mixtral", "{\"model_type\": \"mixtral\", \"num_hidden_layers\": 32, \"hidden_size\": 4096, \"num_attention_heads\": 32, \"num_key_value_heads\": 8, \"num_local_experts\": 8, \"num_experts_per_tok\": 2}"),
            ("DeepSeek", "{\"model_type\": \"deepseek\", \"num_hidden_layers\": 62, \"hidden_size\": 4096, \"kv_lora_rank\": 512, \"qk_rope_head_dim\": 64}"),
        ]

        for (name, config) in configs {
            let result = try HwLedger.plan(configJson: config, seqLen: 4096, concurrentUsers: 1, batchSize: 1)
            XCTAssertNotNil(result, "\(name) should plan successfully")
            XCTAssert(result.totalBytes > 0, "\(name) should have non-zero memory")
        }
    }

    // MARK: - Test 3: KV quantization affects memory
    // Traces to: FR-PLAN-001
    func testQuantizationAffectsMemory() throws {
        let config = """
        {
          "model_type": "llama",
          "num_hidden_layers": 32,
          "hidden_size": 4096,
          "num_attention_heads": 32,
          "num_key_value_heads": 8
        }
        """

        let fp16 = try HwLedger.plan(configJson: config, seqLen: 4096, concurrentUsers: 1, batchSize: 1, kvQuantization: .fp16)
        let int4 = try HwLedger.plan(configJson: config, seqLen: 4096, concurrentUsers: 1, batchSize: 1, kvQuantization: .int4)

        XCTAssertGreaterThan(fp16.kvBytes, int4.kvBytes, "FP16 KV should be larger than INT4")
    }

    // MARK: - Test 4: Batch size affects memory
    // Traces to: FR-PLAN-001
    func testBatchSizeAffectsMemory() throws {
        let config = """
        {
          "model_type": "llama",
          "num_hidden_layers": 32,
          "hidden_size": 4096,
          "num_attention_heads": 32,
          "num_key_value_heads": 8
        }
        """

        let batch1 = try HwLedger.plan(configJson: config, seqLen: 4096, concurrentUsers: 1, batchSize: 1)
        let batch4 = try HwLedger.plan(configJson: config, seqLen: 4096, concurrentUsers: 1, batchSize: 4)

        XCTAssertGreaterThan(batch4.totalBytes, batch1.totalBytes, "Larger batch should use more memory")
    }

    // MARK: - Test 5: Sequence length affects KV cache
    // Traces to: FR-PLAN-001
    func testSequenceLengthAffectsKV() throws {
        let config = """
        {
          "model_type": "llama",
          "num_hidden_layers": 32,
          "hidden_size": 4096,
          "num_attention_heads": 32,
          "num_key_value_heads": 8
        }
        """

        let seq2k = try HwLedger.plan(configJson: config, seqLen: 2048, concurrentUsers: 1, batchSize: 1)
        let seq8k = try HwLedger.plan(configJson: config, seqLen: 8192, concurrentUsers: 1, batchSize: 1)

        XCTAssertGreaterThan(seq8k.kvBytes, seq2k.kvBytes, "Longer sequence should use more KV cache")
    }
}
