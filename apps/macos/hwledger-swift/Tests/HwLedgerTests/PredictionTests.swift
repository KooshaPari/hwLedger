import XCTest
@testable import HwLedger

final class PredictionTests: XCTestCase {
    private let sampleJson = """
    {
      "baseline": {
        "weights_bytes": 16000000000,
        "kv_bytes": 2000000000,
        "prefill_bytes": 500000000,
        "runtime_bytes": 250000000,
        "total_bytes": 18750000000
      },
      "candidate": {
        "weights_bytes": 4000000000,
        "kv_bytes": 500000000,
        "prefill_bytes": 125000000,
        "runtime_bytes": 60000000,
        "total_bytes": 4685000000
      },
      "decode_tps": {"value": 85.0, "low": 72.0, "high": 98.0},
      "ttft_ms":    {"value": 180.0, "low": 150.0, "high": 230.0},
      "throughput": {"value": 1200.0, "low": 1050.0, "high": 1400.0},
      "transformation": {
        "verdict": "lora_required",
        "lora_rank": 16,
        "estimated_gpu_hours": 12.0,
        "rationale": "INT4 quantization with LoRA adapters."
      },
      "citations": [
        {"id": "lora-2021", "title": "LoRA: Low-Rank Adaptation", "url": "https://arxiv.org/abs/2106.09685", "metric": "training"},
        {"id": "int4-gptq-2023", "title": "GPTQ", "url": "https://arxiv.org/abs/2210.17323", "metric": "memory"}
      ]
    }
    """

    func testDecodePrediction() throws {
        let prediction = try HwLedger.decodePrediction(json: sampleJson)
        XCTAssertEqual(prediction.baseline.totalBytes, 18_750_000_000)
        XCTAssertEqual(prediction.candidate.totalBytes, 4_685_000_000)
        XCTAssertEqual(prediction.decodeTps.value, 85.0, accuracy: 0.001)
        XCTAssertEqual(prediction.decodeTps.low, 72.0, accuracy: 0.001)
        XCTAssertEqual(prediction.decodeTps.high, 98.0, accuracy: 0.001)
        XCTAssertEqual(prediction.citations.count, 2)
    }

    func testTransformationEnumMapping() throws {
        let prediction = try HwLedger.decodePrediction(json: sampleJson)
        XCTAssertEqual(prediction.transformation.verdict, .loraRequired)
        XCTAssertEqual(prediction.transformation.humanReadableVerdict, "LoRA required")
        XCTAssertEqual(prediction.transformation.loraRank, 16)
        XCTAssertEqual(prediction.transformation.estimatedGpuHours ?? 0, 12.0, accuracy: 0.001)
    }

    func testAllVerdictValues() throws {
        let cases: [(String, TransformationVerdict, String)] = [
            ("pure_config_swap", .pureConfigSwap, "Pure config swap"),
            ("lora_required", .loraRequired, "LoRA required"),
            ("full_finetune_required", .fullFineTuneRequired, "Full fine-tune required"),
            ("incompatible", .incompatible, "Incompatible")
        ]
        for (raw, expected, human) in cases {
            XCTAssertEqual(TransformationVerdict(rawValue: raw), expected)
            XCTAssertEqual(expected.humanReadable, human)
        }
    }

    func testDecodeInvalidJson() {
        XCTAssertThrowsError(try HwLedger.decodePrediction(json: "{}")) { error in
            if case HwLedgerError.invalidData = error {
                // expected
            } else {
                XCTFail("expected invalidData, got \(error)")
            }
        }
    }
}

private extension TransformationDetails {
    /// Helper that mirrors the `verdict.humanReadable` accessor for tests.
    var humanReadableVerdict: String { verdict.humanReadable }
}
