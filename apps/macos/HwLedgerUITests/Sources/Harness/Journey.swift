import Foundation
import AppKit
import HwLedgerGuiRecorder

/// Lightweight journey-authoring DSL for UI testing.
/// Each journey captures screenshots after steps and maintains a manifest of execution.
/// Can optionally record the screen using hwledger-gui-recorder.
public class Journey {
    public let id: String
    private var steps: [(slug: String, intent: String, closure: () async throws -> Void)] = []
    private var screenshots: [String] = []
    /// Parallel to `screenshots`: the structural-snapshot sibling file (if
    /// captured). `nil` placeholder for steps that did not produce one.
    /// Traces to: Tier 0 structural-capture (macOS family).
    private var structuralPaths: [String?] = []
    private var startTime: Date = Date()
    private var finishTime: Date?
    private var passed: Bool = false
    private var failureReason: String?
    private let appDriver: AppDriver
    private let journeyDirectory: URL
    private var screenRecorder: ScreenCaptureRecorder?
    private var recordingPath: URL?
    private var recordingDenied: Bool = false
    /// Cursor-track recorder. Owned by the journey; wired into
    /// `appDriver.cursorTracker` so synthesized click/release events at
    /// tap sites end up in the JSONL alongside real HID cursor motion.
    /// Emits `<journeyDir>/cursor-track.jsonl` in `finalize()`.
    /// Traces to: Deliverable 2 (XCUITest parallel to Playwright D3).
    private let cursorTracker: CursorTracker
    /// Per-step native window dimensions (in screen points), captured at
    /// step-start. Feeds `JourneyStep.native_width` / `native_height` so
    /// downstream Remotion `CursorOverlay` can scale coordinates from the
    /// capture-time window into the composition canvas.
    private var nativeSizes: [CGSize?] = []
    /// Optional accessibility source for structural-capture. When set, every
    /// `screenshot(...)` call also writes a `.structural.json` sibling next
    /// to the PNG via `AccessibilitySnapshot.writeSibling`. When nil, the
    /// structural hook is a no-op (journey runs in screenshot-only mode).
    public var accessibilityProvider: (() -> (any AccessibilityNodeSource)?)?

    /// Initialize a journey with an ID and app driver.
    /// - Parameters:
    ///   - id: Unique identifier for this journey (used as directory name)
    ///   - appDriver: AppDriver instance to control the app
    public init(id: String, appDriver: AppDriver) throws {
        self.id = id
        self.appDriver = appDriver
        self.journeyDirectory = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
            .appendingPathComponent("journeys")
            .appendingPathComponent(id)

        // Create journey directory
        try FileManager.default.createDirectory(at: journeyDirectory, withIntermediateDirectories: true)

        // Start cursor tracking for this journey and wire the driver so
        // synthesized tap events at tap sites are recorded in the JSONL.
        // `startTime` is anchored to now so ts_ms is rebased to the
        // journey's start — matching Playwright's recorderStartMs contract.
        self.cursorTracker = CursorTracker(startTime: Date())
        self.cursorTracker.start()
        appDriver.cursorTracker = self.cursorTracker
    }

    /// Enable screen recording for this journey.
    /// Must be called before run().
    /// - Throws: RecorderError if recording cannot start
    public func enableScreenRecording(appIdentifier: String) async throws {
        let recordingPath = journeyDirectory.appendingPathComponent("recording.mp4")
        let recorder = ScreenCaptureRecorder(outputPath: recordingPath)

        // Check permission
        let permission = await recorder.checkPermission()
        if permission == .denied {
            recordingDenied = true
            print("Journey: Screen recording permission denied. Continuing without recording.")
            self.screenRecorder = recorder
            self.recordingPath = recordingPath
            return
        }

        // Start recording
        try await recorder.startRecording(appIdentifier: appIdentifier)
        self.screenRecorder = recorder
        self.recordingPath = recordingPath
    }

    /// Add a step to the journey.
    func step(
        _ slug: String,
        intent: String = "",
        closure: @escaping () async throws -> Void
    ) {
        steps.append((slug: slug, intent: intent, closure: closure))
    }

    /// Capture a screenshot with an intent label. When
    /// `accessibilityProvider` is set, also emits a `<stem>.structural.json`
    /// sibling capturing the accessibility tree at this moment.
    func screenshot(intent: String = "") async throws {
        let index = screenshots.count
        let filename = String(format: "%02d-%@.png", index + 1, formatSlug(intent))
        let filepath = journeyDirectory.appendingPathComponent(filename)

        let pngData = try appDriver.screenshot()
        try pngData.write(to: filepath)
        screenshots.append(filename)

        // Tier 0 structural-capture hook. Best-effort: any failure is logged
        // but does not abort the journey (screenshot PNG is the primary
        // capture; structural JSON is supplementary).
        var structuralFilename: String? = nil
        if let provider = accessibilityProvider, let source = provider() {
            do {
                let siblingURL = try AccessibilitySnapshot.writeSibling(
                    keyframePath: filepath,
                    source: source
                )
                structuralFilename = siblingURL.lastPathComponent
            } catch {
                print("Journey: structural-capture failed for \(filename): \(error)")
            }
        }
        structuralPaths.append(structuralFilename)

        // Record the main-window bounds at capture time. Best-effort; a
        // nil value simply omits the native dims for this step.
        nativeSizes.append(appDriver.mainWindowBounds()?.size)
    }

    /// Post-step hook: explicitly request a structural snapshot tied to the
    /// most recent screenshot. Useful when the app state settles after the
    /// PNG capture (e.g. animation completes). Idempotent — overwrites the
    /// sibling file if called more than once.
    public func snapshotAccessibility(after index: Int, source: any AccessibilityNodeSource) throws {
        guard index >= 0, index < screenshots.count else { return }
        let filepath = journeyDirectory.appendingPathComponent(screenshots[index])
        let sibling = try AccessibilitySnapshot.writeSibling(
            keyframePath: filepath,
            source: source
        )
        // Pad structuralPaths if needed (defensive for mid-run reindex).
        while structuralPaths.count <= index {
            structuralPaths.append(nil)
        }
        structuralPaths[index] = sibling.lastPathComponent
    }

    /// Add an assertion to the journey. Throws if assertion fails.
    func assert(_ condition: @autoclosure () -> Bool, _ message: @autoclosure () -> String) throws {
        guard condition() else {
            throw JourneyError.assertionFailed(message())
        }
    }

    /// Execute the journey and record the manifest.
    func run() async throws {
        startTime = Date()

        do {
            for stepData in steps {
                try await stepData.closure()
                // Screenshot is captured within the step closure via the step DSL
            }
            passed = true
        } catch {
            failureReason = String(describing: error)
            throw error
        }

        finishTime = Date()

        // Stop recording if active (after journey, not in defer)
        if let recorder = screenRecorder {
            do {
                _ = try await recorder.stopRecording()
            } catch {
                print("Journey: Failed to stop recording: \(error)")
            }
        }

        // Stop cursor tracking and flush the JSONL sibling. Best-effort:
        // the screenshot + manifest are the primary deliverables; a write
        // failure is logged but does not fail the journey.
        cursorTracker.stop()
        do {
            try cursorTracker.writeJSONL(to: journeyDirectory)
        } catch {
            print("Journey: Failed to write cursor-track.jsonl: \(error)")
        }
    }

    /// Test seam: returns an immutable snapshot of recorded cursor events.
    /// Used by harness tests to assert at least one event was captured and
    /// that timestamps are monotonically non-decreasing.
    public func cursorEventsSnapshot() -> [CursorTrackEvent] {
        cursorTracker.snapshot()
    }

    /// Write the journey manifest (JSON) to disk.
    func writeManifest() throws {
        let manifest = JourneyManifest(
            id: id,
            steps: steps.enumerated().map { index, step in
                let size = nativeSizes.indices.contains(index) ? nativeSizes[index] : nil
                return JourneyStep(
                    index: index,
                    slug: step.slug,
                    intent: step.intent,
                    screenshot_path: screenshots.indices.contains(index) ? screenshots[index] : nil,
                    structural_path: structuralPaths.indices.contains(index) ? structuralPaths[index] : nil,
                    native_width: size.map { Int($0.width) },
                    native_height: size.map { Int($0.height) }
                )
            },
            started_at: ISO8601DateFormatter().string(from: startTime),
            finished_at: finishTime.map { ISO8601DateFormatter().string(from: $0) },
            passed: passed,
            failure: failureReason,
            recording: !recordingDenied && FileManager.default.fileExists(
                atPath: journeyDirectory.appendingPathComponent("recording.mp4").path
            ),
            recording_denied: recordingDenied
        )

        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        let jsonData = try encoder.encode(manifest)
        let manifestPath = journeyDirectory.appendingPathComponent("manifest.json")
        try jsonData.write(to: manifestPath)
    }

    /// Encode a slug safely (alphanumeric + dash).
    private func formatSlug(_ text: String) -> String {
        text.lowercased()
            .replacingOccurrences(of: "[^a-z0-9]+", with: "-", options: .regularExpression)
            .trimmingCharacters(in: CharacterSet(charactersIn: "-"))
    }
}

// MARK: - Data Structures

struct JourneyManifest: Codable {
    let id: String
    let steps: [JourneyStep]
    let started_at: String
    let finished_at: String?
    let passed: Bool
    let failure: String?
    let recording: Bool
    let recording_denied: Bool?
}

struct JourneyStep: Codable {
    let index: Int
    let slug: String
    let intent: String
    let screenshot_path: String?
    /// Tier 0 structural-capture: sibling JSON path next to `screenshot_path`.
    /// Present when the accessibility walker ran; nil otherwise. Downstream
    /// consumers: viewer's "Structural" toolbar pane + traceability gate.
    let structural_path: String?
    /// Main-window width at capture time, in screen points. Mirrors the
    /// Playwright manifest field so the Remotion `CursorOverlay` can scale
    /// cursor-track coordinates across capture and composition canvases.
    let native_width: Int?
    /// Main-window height at capture time, in screen points.
    let native_height: Int?
}

enum JourneyError: LocalizedError {
    case screenshotCaptureFailed
    case assertionFailed(String)

    var errorDescription: String? {
        switch self {
        case .screenshotCaptureFailed:
            return "Failed to capture screenshot"
        case .assertionFailed(let message):
            return "Assertion failed: \(message)"
        }
    }
}
