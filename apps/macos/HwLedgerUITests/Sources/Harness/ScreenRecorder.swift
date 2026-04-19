import Foundation
import AVFoundation
import ScreenCaptureKit

/// Records app interactions to MP4 using ScreenCaptureKit (macOS 14+).
/// Note: This is a placeholder implementation demonstrating the recorder interface.
/// Full integration requires proper SCStreamDelegate handling and TCC permissions.
public class ScreenRecorder: NSObject {
    private var stream: SCStream?
    private var assetWriter: AVAssetWriter?
    private var videoInput: AVAssetWriterInput?
    private var isRecording: Bool = false
    private let outputPath: URL
    private var recordingDenied: Bool = false

    /// Initialize ScreenRecorder with an output path.
    /// - Parameter outputPath: URL where the MP4 will be saved
    public init(outputPath: URL) {
        self.outputPath = outputPath
        super.init()
    }

    /// Start recording the app window.
    /// - Parameter appIdentifier: Bundle identifier of the app (e.g., "com.kooshapari.hwLedger")
    public func startRecording(appIdentifier: String) async throws {
        // Request screen recording permission
        do {
            _ = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: true)
        } catch {
            // TCC denial or error
            recordingDenied = true
            print("ScreenCaptureKit: Recording permission denied or unavailable. Continuing without recording.")
            return
        }

        // Get the app's windows
        let availableContent = try await SCShareableContent.current
        guard let display = availableContent.displays.first else {
            throw RecorderError.noDisplayFound
        }

        // Create content filter for the app
        let filter = SCContentFilter(display: display, excludingWindows: [])

        // Configure stream
        let streamConfig = SCStreamConfiguration()
        streamConfig.width = 1440
        streamConfig.height = 900
        streamConfig.captureResolution = .automatic

        // Initialize asset writer
        try initializeAssetWriter()

        // Create and start the stream
        let stream = SCStream(filter: filter, configuration: streamConfig, delegate: self)
        self.stream = stream

        try await stream.startCapture()
        isRecording = true
    }

    /// Stop recording and finalize the MP4.
    public func stopRecording() async throws -> URL {
        guard isRecording else {
            if recordingDenied {
                // Return a path even if recording was denied
                return outputPath
            }
            throw RecorderError.notRecording
        }

        isRecording = false

        // Stop the stream
        if let stream = stream {
            try await stream.stopCapture()
            self.stream = nil
        }

        // Finalize asset writer
        if let assetWriter = assetWriter {
            videoInput?.markAsFinished()
            await assetWriter.finishWriting()
            self.assetWriter = nil
            self.videoInput = nil
        }

        return outputPath
    }

    // MARK: - Private

    private func initializeAssetWriter() throws {
        // Remove existing file
        try? FileManager.default.removeItem(at: outputPath)

        let assetWriter = try AVAssetWriter(outputURL: outputPath, fileType: .mp4)

        let videoSettings: [String: Any] = [
            AVVideoCodecKey: AVVideoCodecType.h264,
            AVVideoWidthKey: 1440,
            AVVideoHeightKey: 900
        ]

        let videoInput = AVAssetWriterInput(mediaType: .video, outputSettings: videoSettings)
        videoInput.expectsMediaDataInRealTime = true

        guard assetWriter.canAdd(videoInput) else {
            throw RecorderError.cannotAddVideoInput
        }

        assetWriter.add(videoInput)
        self.assetWriter = assetWriter
        self.videoInput = videoInput

        if !assetWriter.startWriting() {
            throw RecorderError.cannotStartWriting
        }

        assetWriter.startSession(atSourceTime: CMTime.zero)
    }
}

extension ScreenRecorder: SCStreamDelegate {
    nonisolated public func stream(_ stream: SCStream, didStopWithError error: Error) {
        print("ScreenRecorder stream error: \(error)")
    }
}

// MARK: - Error Types

public enum RecorderError: LocalizedError {
    case noDisplayFound
    case contentFilterFailed
    case cannotAddVideoInput
    case cannotStartWriting
    case notRecording

    public var errorDescription: String? {
        switch self {
        case .noDisplayFound:
            return "No display found for recording"
        case .contentFilterFailed:
            return "Failed to create content filter"
        case .cannotAddVideoInput:
            return "Cannot add video input to asset writer"
        case .cannotStartWriting:
            return "Cannot start writing to asset"
        case .notRecording:
            return "Recording not in progress"
        }
    }
}
