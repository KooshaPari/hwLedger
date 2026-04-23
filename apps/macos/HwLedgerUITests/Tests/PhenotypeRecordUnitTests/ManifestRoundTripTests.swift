import XCTest
@testable import PhenotypeRecord

final class ManifestRoundTripTests: XCTestCase {

    func test_assembleAndRoundTrip() throws {
        let story = PhenotypeUserStory(
            journey_id: "gui-demo",
            title: "Demo",
            persona: "dev",
            given: "state",
            when: ["do"],
            then: ["see"],
            traces_to: ["FR-1"],
            record: true,
            blind_judge: "auto",
            backend: "swift",
            blind_eval: nil,
            family: "gui"
        )
        let kf1 = PhenotypeKeyframe(
            index: 0,
            activity_name: "launch",
            timestamp_iso: "2026-04-22T12:00:00.000Z",
            structural_path: "snapshots/launch.json",
            assertion: "attention-kind-label present"
        )
        let kf2 = PhenotypeKeyframe(
            index: 1,
            activity_name: "adjust",
            timestamp_iso: "2026-04-22T12:00:01.500Z",
            structural_path: "snapshots/adjust.json",
            assertion: nil
        )
        let started = Date(timeIntervalSince1970: 0)
        let finished = Date(timeIntervalSince1970: 5)

        let manifest = PhenotypeManifestAssembler.assemble(
            story: story,
            keyframes: [kf1, kf2],
            started: started,
            finished: finished,
            passed: true,
            failure: nil,
            recordingPath: "recording.mp4",
            recordingDenied: false
        )

        XCTAssertEqual(manifest.schema_version, "user-story.manifest.verified/1")
        XCTAssertEqual(manifest.journey_id, "gui-demo")
        XCTAssertEqual(manifest.keyframes.count, 2)
        XCTAssertEqual(manifest.story.traces_to, ["FR-1"])

        let data = try PhenotypeManifestAssembler.encode(manifest)
        let decoded = try PhenotypeManifestAssembler.decode(data)
        XCTAssertEqual(decoded, manifest, "encode → decode should round-trip")
    }

    func test_failurePathPopulatesFields() throws {
        let story = PhenotypeUserStory(
            journey_id: "gui-fail",
            title: "Fail",
            persona: "dev",
            given: "state",
            when: ["do"],
            then: ["see"],
            traces_to: ["FR-2"]
        )
        let manifest = PhenotypeManifestAssembler.assemble(
            story: story,
            keyframes: [],
            started: Date(),
            finished: Date(),
            passed: false,
            failure: "slider element not found",
            recordingPath: nil,
            recordingDenied: true
        )
        XCTAssertFalse(manifest.passed)
        XCTAssertEqual(manifest.failure, "slider element not found")
        XCTAssertTrue(manifest.recording_denied)
        XCTAssertNil(manifest.recording_path)

        // Round trip preserves optional nils.
        let data = try PhenotypeManifestAssembler.encode(manifest)
        let decoded = try PhenotypeManifestAssembler.decode(data)
        XCTAssertEqual(decoded, manifest)
    }
}
