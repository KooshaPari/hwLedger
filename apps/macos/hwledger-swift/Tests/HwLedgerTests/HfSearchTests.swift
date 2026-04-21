import XCTest
@testable import HwLedger

final class HfSearchTests: XCTestCase {
    /// Sample HF search payload the Rust FFI will hand back via JSON.
    private let sampleJson = """
    {
      "models": [
        {
          "repo_id": "meta-llama/Llama-3.1-8B",
          "display_name": "Llama 3.1 8B",
          "param_count": 8000000000,
          "downloads": 2340000,
          "last_modified": "2025-09-14",
          "pipeline_tag": "text-generation",
          "library": "transformers",
          "tags": ["llama", "text-generation"],
          "trending": 0.9,
          "config_json": "{\\"model_type\\":\\"llama\\"}"
        },
        {
          "repo_id": "mistralai/Mistral-7B-v0.3",
          "param_count": 7000000000,
          "downloads": 1120000,
          "last_modified": "2025-08-01",
          "pipeline_tag": "text-generation",
          "library": "transformers",
          "tags": ["mistral"],
          "trending": 0.7
        }
      ],
      "rate_limited": false,
      "next_cursor": null
    }
    """

    func testDecodeHfSearchResponse() throws {
        let response = try HwLedger.decodeHfSearchResponse(json: sampleJson)
        XCTAssertEqual(response.models.count, 2)
        XCTAssertFalse(response.rateLimited)
        XCTAssertNil(response.nextCursor)

        let first = response.models[0]
        XCTAssertEqual(first.repoId, "meta-llama/Llama-3.1-8B")
        XCTAssertEqual(first.paramCount, 8_000_000_000)
        XCTAssertEqual(first.downloads, 2_340_000)
        XCTAssertEqual(first.pipelineTag, "text-generation")
        XCTAssertEqual(first.library, "transformers")
        XCTAssertEqual(first.tags, ["llama", "text-generation"])
        XCTAssertEqual(first.trending, 0.9)
        XCTAssertNotNil(first.configJson)
    }

    func testDecodeRateLimited() throws {
        let json = """
        { "models": [], "rate_limited": true, "next_cursor": null }
        """
        let response = try HwLedger.decodeHfSearchResponse(json: json)
        XCTAssertTrue(response.rateLimited)
        XCTAssertTrue(response.models.isEmpty)
    }

    func testDecodeInvalidJson() {
        XCTAssertThrowsError(try HwLedger.decodeHfSearchResponse(json: "not json")) { error in
            if case HwLedgerError.invalidData = error {
                // expected
            } else {
                XCTFail("expected invalidData, got \(error)")
            }
        }
    }

    func testModelCardIdentityMatchesRepoId() {
        let card = ModelCard(repoId: "a/b", paramCount: 1, tags: [])
        XCTAssertEqual(card.id, "a/b")
    }
}
