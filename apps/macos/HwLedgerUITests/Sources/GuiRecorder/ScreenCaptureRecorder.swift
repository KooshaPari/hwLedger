import Foundation
import SckBridge
import ScreenCaptureKit
import AVFoundation

/// ScreenCaptureRecorder wraps ScreenCaptureKit for app recording.
/// Provides async-friendly Swift API for recording app interactions.
public class ScreenCaptureRecorder {
    private let outputPath: URL
    private var stream: SCStream?
    private var assetWriter: AVAssetWriter?
    private var videoInput: AVAssetWriterInput?
    private var pixelBufferAdaptor: AVAssetWriterInputPixelBufferAdaptor?
    private var isRecording = false
    private var recordingDenied = false

    /// Initialize with output path.
    /// - Parameter outputPath: URL where the MP4 will be saved
    public init(outputPath: URL) {
        self.outputPath = outputPath
    }

    /// Check if Screen Recording permission is granted.
    /// - Returns: RecordingPermission enum indicating permission status
    public func checkPermission() async -> RecordingPermission {
        do {
            _ = try await SCShareableContent.excludingDesktopWindows(true, onScreenWindowsOnly: true)
            return .granted
        } catch {
            return .denied
        }
    }

    /// Start recording the app window.
    /// - Parameters:
    ///   - appIdentifier: Bundle identifier of the app (e.g., "com.kooshapari.hwLedger")
    ///   - width: Output width in pixels (default: 1440)
    ///   - height: Output height in pixels (default: 900)
    ///   - fps: Frames per second (default: 30)
    public func startRecording(appIdentifier: String, width: UInt32 = 1440, height: UInt32 = 900, fps: UInt32 = 30) async throws {
        // Check permission first
        let permission = await checkPermission()
        if permission == .denied {
            recordingDenied = true
            print("ScreenCaptureKit: Recording permission denied. Continuing without recording.")
            return
        }

        try await performStartRecording(appIdentifier: appIdentifier, width: width, height: height, fps: fps)
    }

    /// Stop recording and finalize the MP4.
    public func stopRecording() async throws -> URL {
        guard isRecording else {
            if recordingDenied {
                return outputPath
            }
            throw RecorderError.notRecording
        }

        try await performStopRecording()
        return outputPath
    }

    /// Check if recording was denied and skipped.
    public var wasRecordingDenied: Bool {
        recordingDenied
    }

    // MARK: - Private

    private func performStartRecording(appIdentifier: String, width: UInt32, height: UInt32, fps: UInt32) async throws {
        // Get available content
        let availableContent = try await SCShareableContent.current

        // Find the target app window
        var targetWindow: SCWindow?
        for window in availableContent.windows {
            if window.owningApplication?.bundleIdentifier == appIdentifier {
                targetWindow = window
                break
            }
        }

        guard let targetWindow = targetWindow else {
            throw RecorderError.startFailed("Window not found for app: \(appIdentifier)")
        }

        // Get main display
        guard let mainDisplay = availableContent.displays.first else {
            throw RecorderError.startFailed("No display found")
        }

        // Create content filter
        let contentFilter = SCContentFilter(display: mainDisplay, including: [targetWindow.owningApplication!], exceptingWindows: [])

        // Configure stream
        let streamConfig = SCStreamConfiguration()
        streamConfig.captureResolution = .automatic
        streamConfig.width = Int(width)
        streamConfig.height = Int(height)

        // Initialize asset writer
        try? FileManager.default.removeItem(at: outputPath)

        let assetWriter = try AVAssetWriter(outputURL: outputPath, fileType: .mp4)

        let videoSettings: [String: Any] = [
            AVVideoCodecKey: AVVideoCodecType.h264,
            AVVideoWidthKey: width,
            AVVideoHeightKey: height
        ]

        let videoInput = AVAssetWriterInput(mediaType: .video, outputSettings: videoSettings)
        videoInput.expectsMediaDataInRealTime = true

        guard assetWriter.canAdd(videoInput) else {
            throw RecorderError.startFailed("Cannot add video input")
        }

        assetWriter.add(videoInput)

        guard assetWriter.startWriting() else {
            throw RecorderError.startFailed("Cannot start writing")
        }

        assetWriter.startSession(atSourceTime: CMTime.zero)

        // Create stream delegate
        let delegate = StreamDelegate(assetWriter: assetWriter, videoInput: videoInput)

        // Create and start stream
        let stream = SCStream(filter: contentFilter, configuration: streamConfig, delegate: delegate)
        try await stream.startCapture()

        self.stream = stream
        self.assetWriter = assetWriter
        self.videoInput = videoInput
        self.isRecording = true
    }

    private func performStopRecording() async throws {
        guard isRecording else {
            throw RecorderError.notRecording
        }

        isRecording = false

        if let stream = stream {
            try await stream.stopCapture()
            self.stream = nil
        }

        if let assetWriter = assetWriter {
            videoInput?.markAsFinished()
            await assetWriter.finishWriting()
            self.assetWriter = nil
            self.videoInput = nil
        }
    }
}

// MARK: - Stream Delegate

private class StreamDelegate: NSObject, SCStreamDelegate {
    let assetWriter: AVAssetWriter
    let videoInput: AVAssetWriterInput

    init(assetWriter: AVAssetWriter, videoInput: AVAssetWriterInput) {
        self.assetWriter = assetWriter
        self.videoInput = videoInput
    }

    nonisolated func stream(_ stream: SCStream, didStopWithError error: Error) {
        NSLog("SCK stream error: %@", error.localizedDescription)
    }
}

// MARK: - Error Types

public enum RecorderError: LocalizedError {
    case startFailed(String)
    case stopFailed(String)
    case notRecording

    public var errorDescription: String? {
        switch self {
        case .startFailed(let msg):
            return "Failed to start recording: \(msg)"
        case .stopFailed(let msg):
            return "Failed to stop recording: \(msg)"
        case .notRecording:
            return "Recording not in progress"
        }
    }
}

public enum RecordingPermission: Equatable {
    case granted
    case denied
    case unknown
}
