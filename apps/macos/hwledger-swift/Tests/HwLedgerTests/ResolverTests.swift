import XCTest
@testable import HwLedger

// Traces to: FR-HF-001, FR-PLAN-003
final class ResolverTests: XCTestCase {

    // MARK: - hf_repo

    func testDecodeHfRepoVariant() throws {
        let json = """
        { "kind": "hf_repo", "repo_id": "meta-llama/Llama-3.1-8B", "revision": null }
        """
        let resolved = try HwLedger.decodeResolvedModel(json: json)
        guard case let .hfRepo(repoId, revision) = resolved else {
            return XCTFail("expected .hfRepo, got \(resolved)")
        }
        XCTAssertEqual(repoId, "meta-llama/Llama-3.1-8B")
        XCTAssertNil(revision)
        XCTAssertEqual(resolved.resolvedId, "meta-llama/Llama-3.1-8B")
        XCTAssertTrue(resolved.isResolved)
    }

    func testDecodeHfRepoWithRevision() throws {
        let json = """
        { "kind": "hf_repo", "repo_id": "org/repo", "revision": "v2" }
        """
        let resolved = try HwLedger.decodeResolvedModel(json: json)
        guard case let .hfRepo(_, revision) = resolved else {
            return XCTFail("expected .hfRepo")
        }
        XCTAssertEqual(revision, "v2")
    }

    // MARK: - golden_fixture

    func testDecodeGoldenFixtureVariant() throws {
        let json = """
        { "kind": "golden_fixture", "path": "/tmp/fixtures/llama-3-8b.json" }
        """
        let resolved = try HwLedger.decodeResolvedModel(json: json)
        guard case let .goldenFixture(url) = resolved else {
            return XCTFail("expected .goldenFixture, got \(resolved)")
        }
        XCTAssertEqual(url.path, "/tmp/fixtures/llama-3-8b.json")
        XCTAssertTrue(resolved.isResolved)
    }

    // MARK: - local_config

    func testDecodeLocalConfigVariant() throws {
        let json = """
        { "kind": "local_config", "path": "/Users/me/models/config.json" }
        """
        let resolved = try HwLedger.decodeResolvedModel(json: json)
        guard case let .localConfig(url) = resolved else {
            return XCTFail("expected .localConfig, got \(resolved)")
        }
        XCTAssertEqual(url.path, "/Users/me/models/config.json")
        XCTAssertTrue(resolved.isResolved)
    }

    // MARK: - ambiguous

    func testDecodeAmbiguousVariantWithCandidates() throws {
        let json = """
        {
          "kind": "ambiguous",
          "hint": "llama",
          "candidates": [
            {
              "repo_id": "meta-llama/Llama-3.1-8B",
              "param_count": 8000000000,
              "downloads": 2340000,
              "tags": ["llama"],
              "pipeline_tag": "text-generation",
              "library": "transformers"
            },
            {
              "repo_id": "meta-llama/Llama-3.1-70B",
              "tags": ["llama"]
            }
          ]
        }
        """
        let resolved = try HwLedger.decodeResolvedModel(json: json)
        guard case let .ambiguous(hint, candidates) = resolved else {
            return XCTFail("expected .ambiguous, got \(resolved)")
        }
        XCTAssertEqual(hint, "llama")
        XCTAssertEqual(candidates.count, 2)
        XCTAssertEqual(candidates[0].repoId, "meta-llama/Llama-3.1-8B")
        XCTAssertFalse(resolved.isResolved)
        XCTAssertNil(resolved.resolvedId)
    }

    // MARK: - Error surfaces

    func testDecodeErrorPayloadSurfacesAsRuntimeError() {
        let json = """
        { "error": "input must not be null" }
        """
        XCTAssertThrowsError(try HwLedger.decodeResolvedModel(json: json)) { error in
            if case let HwLedgerError.runtimeError(msg) = error {
                XCTAssertTrue(msg.contains("input must not be null"))
            } else {
                XCTFail("expected runtimeError, got \(error)")
            }
        }
    }

    func testDecodeUnknownKindThrows() {
        let json = """
        { "kind": "nonsense" }
        """
        XCTAssertThrowsError(try HwLedger.decodeResolvedModel(json: json))
    }
}
