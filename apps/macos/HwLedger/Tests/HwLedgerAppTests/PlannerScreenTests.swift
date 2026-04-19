import XCTest
import HwLedger

// Traces to: FR-PLAN-003, FR-PLAN-004, FR-UI-002
final class PlannerScreenTests: XCTestCase {

    let llama8bJson = """
    {
      "architectures": ["LlamaForCausalLM"],
      "model_type": "llama",
      "num_hidden_layers": 32,
      "hidden_size": 4096,
      "num_attention_heads": 32,
      "num_key_value_heads": 8,
      "intermediate_size": 11008
    }
    """

    let deepseekJson = """
    {
      "architectures": ["DeepseekV3ForCausalLM"],
      "model_type": "deepseek",
      "num_hidden_layers": 61,
      "hidden_size": 7680,
      "num_attention_heads": 60,
      "num_key_value_heads": 1,
      "kv_lora_rank": 512,
      "qk_rope_head_dim": 64,
      "intermediate_size": 20480,
      "num_local_experts": 256,
      "num_experts_per_tok": 21
    }
    """

    let mixtralJson = """
    {
      "architectures": ["MixtralForCausalLM"],
      "model_type": "mixtral",
      "num_hidden_layers": 32,
      "hidden_size": 4096,
      "num_attention_heads": 32,
      "num_key_value_heads": 8,
      "intermediate_size": 14336,
      "num_local_experts": 8,
      "num_experts_per_tok": 2
    }
    """

    let mambaJson = """
    {
      "architectures": ["Mamba2ForCausalLM"],
      "model_type": "mamba2",
      "num_hidden_layers": 12,
      "hidden_size": 2560,
      "state_size": 64,
      "expand": 2
    }
    """

    // MARK: - Test 1: DeepSeekV3 classifies as MLA
    // Traces to: FR-PLAN-002, FR-PLAN-003
    func testDeepSeekV3ClassifiesMLA() throws {
        let result = try HwLedger.plan(
            configJson: deepseekJson,
            seqLen: 8192,
            concurrentUsers: 1,
            batchSize: 1
        )

        XCTAssertEqual(
            result.attentionKindLabel,
            "Mla",
            "DeepSeekV3 should be classified as MLA"
        )
    }

    // MARK: - Test 2: Mixtral detects MoE and calculates correctly
    // Traces to: FR-PLAN-003
    func testMixtralMemoryPlanning() throws {
        let result = try HwLedger.plan(
            configJson: mixtralJson,
            seqLen: 4096,
            concurrentUsers: 1,
            batchSize: 1
        )

        XCTAssertNotNil(result)
        XCTAssert(result.weightsBytes > 0, "Mixtral should have non-zero weights")
        XCTAssert(result.kvBytes > 0, "Mixtral should have non-zero KV cache")
    }

    // MARK: - Test 3: SSM/Mamba classifies correctly
    // Traces to: FR-PLAN-002
    func testSSMModelClassifiesSSM() throws {
        let result = try HwLedger.plan(
            configJson: mambaJson,
            seqLen: 8192,
            concurrentUsers: 1,
            batchSize: 1
        )

        XCTAssertEqual(
            result.attentionKindLabel,
            "Ssm",
            "Mamba2 should be classified as SSM"
        )
    }

    // MARK: - Test 4: Sequence length impacts KV cache size
    // Traces to: FR-PLAN-004, FR-PLAN-003
    func testSequenceLengthImpactsKVCache() throws {
        let result1 = try HwLedger.plan(
            configJson: llama8bJson,
            seqLen: 2048,
            concurrentUsers: 1,
            batchSize: 1
        )

        let result2 = try HwLedger.plan(
            configJson: llama8bJson,
            seqLen: 8192,
            concurrentUsers: 1,
            batchSize: 1
        )

        XCTAssert(
            result2.kvBytes > result1.kvBytes,
            "KV cache should increase with sequence length"
        )
    }

    // MARK: - Test 5: Concurrent users impacts KV cache size
    // Traces to: FR-PLAN-004, FR-PLAN-003
    func testConcurrentUsersImpactKVCache() throws {
        let result1 = try HwLedger.plan(
            configJson: llama8bJson,
            seqLen: 4096,
            concurrentUsers: 1,
            batchSize: 1
        )

        let result2 = try HwLedger.plan(
            configJson: llama8bJson,
            seqLen: 4096,
            concurrentUsers: 4,
            batchSize: 1
        )

        XCTAssert(
            result2.kvBytes > result1.kvBytes,
            "KV cache should increase with concurrent users"
        )
    }

    // MARK: - Test 6: KV quantization reduces memory
    // Traces to: FR-PLAN-004, NFR-004
    func testKVQuantizationReducesMemory() throws {
        let fp16Result = try HwLedger.plan(
            configJson: llama8bJson,
            seqLen: 2048,
            concurrentUsers: 1,
            batchSize: 1,
            kvQuantization: .fp16
        )

        let int4Result = try HwLedger.plan(
            configJson: llama8bJson,
            seqLen: 2048,
            concurrentUsers: 1,
            batchSize: 1,
            kvQuantization: .int4
        )

        XCTAssert(
            int4Result.kvBytes < fp16Result.kvBytes,
            "INT4 KV quantization should reduce KV cache size compared to FP16"
        )
    }

    // MARK: - Test 7: Weight quantization reduces memory
    // Traces to: FR-PLAN-004, NFR-004
    func testWeightQuantizationReducesMemory() throws {
        let fp16Result = try HwLedger.plan(
            configJson: llama8bJson,
            seqLen: 2048,
            concurrentUsers: 1,
            batchSize: 1,
            weightQuantization: .fp16
        )

        let int4Result = try HwLedger.plan(
            configJson: llama8bJson,
            seqLen: 2048,
            concurrentUsers: 1,
            batchSize: 1,
            weightQuantization: .int4
        )

        XCTAssert(
            int4Result.weightsBytes < fp16Result.weightsBytes,
            "INT4 weight quantization should reduce weight size compared to FP16"
        )
    }

    // MARK: - Test 8: Custom JSON loading with valid config
    // Traces to: FR-PLAN-001
    func testCustomJsonLoading() throws {
        let testJson = """
        {
          "model_type": "llama",
          "num_hidden_layers": 32,
          "hidden_size": 4096,
          "num_attention_heads": 32,
          "num_key_value_heads": 8,
          "intermediate_size": 11008
        }
        """

        let result = try HwLedger.plan(
            configJson: testJson,
            seqLen: 2048,
            concurrentUsers: 1,
            batchSize: 1
        )

        XCTAssertNotNil(result)
        XCTAssert(result.totalBytes > 0, "Custom config should produce valid plan")
    }

    // MARK: - Test 9: Device detection does not throw
    // Traces to: FR-PLAN-006
    func testDeviceDetectionDoesNotThrow() throws {
        let devices = try HwLedger.detectDevices()
        XCTAssertNotNil(devices, "Device list should never be nil")
    }

    // MARK: - Test 10: Effective batch calculation
    // Traces to: FR-PLAN-003, FR-PLAN-004
    func testEffectiveBatchCalculation() throws {
        let result = try HwLedger.plan(
            configJson: llama8bJson,
            seqLen: 2048,
            concurrentUsers: 1,
            batchSize: 8
        )

        XCTAssert(result.effectiveBatch > 0, "Effective batch should be non-zero")
        XCTAssert(
            result.effectiveBatch <= 8,
            "Effective batch should not exceed requested batch size"
        )
    }
}
