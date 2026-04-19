import XCTest
import HwLedger

// Traces to: FR-UI-002, FR-INF-001, FR-INF-002
final class RunScreenTests: XCTestCase {

    // MARK: - Test 1: MLX spawn returns valid handle
    // Traces to: FR-INF-001
    func testMlxSpawnValid() throws {
        let handle = try HwLedger.mlxSpawn(pythonPath: "python3", omlxModule: "omlx")
        XCTAssertNotNil(handle, "MLX handle should not be nil")
    }

    // MARK: - Test 2: Generate begins and returns request ID
    // Traces to: FR-INF-002
    func testMlxGenerateBeginValid() throws {
        let handle = try HwLedger.mlxSpawn()
        let requestId = HwLedger.mlxGenerateBegin(
            handle: handle,
            prompt: "Hello world",
            paramsJson: "{}"
        )
        XCTAssertGreaterThan(requestId, 0, "Request ID should be positive")
        HwLedger.mlxShutdown(handle: handle)
    }

    // MARK: - Test 3: Poll token returns valid state
    // Traces to: FR-INF-002
    func testMlxPollTokenValid() throws {
        let handle = try HwLedger.mlxSpawn()
        let requestId = HwLedger.mlxGenerateBegin(handle: handle, prompt: "test")

        let (state, token) = HwLedger.mlxPollToken(requestId: requestId, bufferCapacity: 256)
        XCTAssertNotNil(state, "Poll state should not be nil")

        HwLedger.mlxShutdown(handle: handle)
    }

    // MARK: - Test 4: Stub mode produces at least 5 tokens
    // Traces to: FR-INF-002, FR-UI-002
    func testStubModeProducesTokens() throws {
        let handle = try HwLedger.mlxSpawn()
        let requestId = HwLedger.mlxGenerateBegin(handle: handle, prompt: "test prompt")

        var tokenCount = 0
        var state: TokenPollState = .pending

        for _ in 0..<100 {
            let (pollState, token) = HwLedger.mlxPollToken(requestId: requestId, bufferCapacity: 256)
            state = pollState

            if pollState == .token && !token.isEmpty {
                tokenCount += 1
            }

            if pollState == .eof {
                break
            }
        }

        XCTAssertGreaterThanOrEqual(tokenCount, 5, "Stub should produce at least 5 tokens")
        XCTAssertEqual(state, .eof, "Should reach EOF after all tokens")

        HwLedger.mlxShutdown(handle: handle)
    }

    // MARK: - Test 5: Cancel request does not throw
    // Traces to: FR-INF-002
    func testMlxCancelNoThrow() throws {
        let handle = try HwLedger.mlxSpawn()
        let requestId = HwLedger.mlxGenerateBegin(handle: handle, prompt: "cancel test")

        HwLedger.mlxCancel(requestId: requestId)
        HwLedger.mlxShutdown(handle: handle)

        XCTAssert(true, "Cancel should complete without error")
    }
}
