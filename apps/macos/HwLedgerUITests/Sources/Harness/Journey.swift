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
    private var startTime: Date = Date()
    private var finishTime: Date?
    private var passed: Bool = false
    private var failureReason: String?
    private let appDriver: AppDriver
    private let journeyDirectory: URL
    private var screenRecorder: ScreenCaptureRecorder?
    private var recordingPath: URL?
    private var recordingDenied: Bool = false

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

    /// Capture a screenshot with an intent label.
    func screenshot(intent: String = "") async throws {
        let index = screenshots.count
        let filename = String(format: "%02d-%@.png", index + 1, formatSlug(intent))
        let filepath = journeyDirectory.appendingPathComponent(filename)

        let pngData = try appDriver.screenshot()
        try pngData.write(to: filepath)
        screenshots.append(filename)
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
    }

    /// Write the journey manifest (JSON) to disk.
    func writeManifest() throws {
        let manifest = JourneyManifest(
            id: id,
            steps: steps.enumerated().map { index, step in
                JourneyStep(
                    index: index,
                    slug: step.slug,
                    intent: step.intent,
                    screenshot_path: screenshots.indices.contains(index) ? screenshots[index] : nil
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
