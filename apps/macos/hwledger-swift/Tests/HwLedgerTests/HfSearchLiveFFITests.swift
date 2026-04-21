import XCTest
@testable import HwLedger

/// Covers the live-FFI decode path for `hwledger_hf_search`.
///
/// We drive [`decodeHfSearchFfiPayload`] with the raw JSON shapes that the
/// Rust FFI actually emits (array of HF `ModelCard`, or `{"error":"..."}`).
/// This gives us coverage over the wire contract without hitting the network,
/// which is the "mock-server fixture" equivalent: the fixture IS the JSON
/// byte-for-byte as it would leave `hwledger_hf_search`.
///
/// Traces to: FR-HF-001
final class HfSearchLiveFFITests: XCTestCase {
    /// Fixture matches `serde_json::to_string(&Vec<hwledger_hf_client::ModelCard>)`
    /// with two representative entries.
    private let successPayload = """
    [
      {
        "id": "meta-llama/Llama-3.1-8B",
        "downloads": 2340000,
        "likes": 4100,
        "tags": ["llama", "text-generation"],
        "library_name": "transformers",
        "pipeline_tag": "text-generation",
        "last_modified": "2025-09-14T12:00:00Z",
        "params_estimate": 8000000000
      },
      {
        "id": "mistralai/Mistral-7B-v0.3",
        "downloads": 1120000,
        "likes": 2200,
        "tags": ["mistral"],
        "library_name": "transformers",
        "pipeline_tag": "text-generation",
        "last_modified": "2025-08-01T00:00:00Z",
        "params_estimate": 7000000000
      }
    ]
    """

    /// HF returns 401 when a token is required but missing/invalid. The FFI
    /// wraps this as `{"error":"... 401 ..."}`.
    private let unauthorizedPayload = """
    {"error": "Hugging Face endpoint `/api/models` requires authentication. HTTP 401 Unauthorized"}
    """

    /// A malformed repo-id from the UI surfaces as a descriptive non-rate-limit
    /// error string that must bubble out as `runtimeError`.
    private let malformedRepoPayload = """
    {"error": "invalid repo_id: '////'"}
    """

    /// Test 1 — success decode: the FFI JSON maps cleanly to Swift
    /// `HfSearchResponse` + `ModelCard` shapes.
    func testSuccessDecodeMapsFfiShapeToSwiftModelCard() throws {
        let response = try decodeHfSearchFfiPayload(json: successPayload)

        XCTAssertEqual(response.models.count, 2)
        XCTAssertFalse(response.rateLimited)
        XCTAssertNil(response.nextCursor)

        let first = response.models[0]
        XCTAssertEqual(first.repoId, "meta-llama/Llama-3.1-8B")
        XCTAssertEqual(first.downloads, 2_340_000)
        XCTAssertEqual(first.paramCount, 8_000_000_000)
        XCTAssertEqual(first.pipelineTag, "text-generation")
        XCTAssertEqual(first.library, "transformers")
        XCTAssertEqual(first.tags, ["llama", "text-generation"])
        XCTAssertEqual(first.lastModified, "2025-09-14T12:00:00Z")

        let second = response.models[1]
        XCTAssertEqual(second.repoId, "mistralai/Mistral-7B-v0.3")
        XCTAssertEqual(second.paramCount, 7_000_000_000)
    }

    /// Test 2 — 401 / unauthorized maps to the rate-limited enum surface so
    /// the UI can render the "add a token in Settings" banner instead of a
    /// generic error.
    func testUnauthorizedResponseMapsToRateLimited() throws {
        let response = try decodeHfSearchFfiPayload(json: unauthorizedPayload)
        XCTAssertTrue(response.rateLimited)
        XCTAssertTrue(response.models.isEmpty)
        XCTAssertNil(response.nextCursor)
    }

    /// Test 3 — malformed repo-id / other descriptive errors propagate as
    /// `HwLedgerError.runtimeError` so the UI shows the real cause.
    func testMalformedRepoIdThrowsRuntimeError() {
        XCTAssertThrowsError(try decodeHfSearchFfiPayload(json: malformedRepoPayload)) { error in
            guard case HwLedgerError.runtimeError(let message) = error else {
                XCTFail("expected runtimeError, got \(error)")
                return
            }
            XCTAssertTrue(message.contains("invalid repo_id"))
        }
    }

    /// Sanity: legacy structured `HfSearchResponse` JSON still round-trips
    /// through the live-path decoder (keeps pre-FFI unit tests meaningful).
    func testStructuredLegacyPayloadStillDecodes() throws {
        let legacy = """
        { "models": [], "rate_limited": true, "next_cursor": null }
        """
        let response = try decodeHfSearchFfiPayload(json: legacy)
        XCTAssertTrue(response.rateLimited)
        XCTAssertTrue(response.models.isEmpty)
    }
}
