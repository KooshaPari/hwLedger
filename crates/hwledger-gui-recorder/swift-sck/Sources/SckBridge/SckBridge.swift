import Foundation
import ScreenCaptureKit
import AVFoundation

// MARK: - Global Recording State

private class RecordingSession {
    var stream: SCStream?
    var assetWriter: AVAssetWriter?
    var videoInput: AVAssetWriterInput?
    var pixelBufferAdaptor: AVAssetWriterInputPixelBufferAdaptor?
    var isRecording = false
    var recordingDenied = false
}

private let recordingSession = RecordingSession()

// MARK: - C FFI Exports

/// Check if Screen Recording permission is granted (TCC).
/// Returns 1 if granted, 0 if denied.
@_cdecl("hwledger_sck_has_permission")
public func hasPermission() -> Int32 {
    // Try to request shareable content; if it fails, permission is denied
    Task {
        do {
            _ = try await SCShareableContent.excludingDesktopWindows(true, onScreenWindowsOnly: true)
            return
        } catch {
            // Permission denied or SCKit unavailable
            recordingSession.recordingDenied = true
        }
    }

    return recordingSession.recordingDenied ? 0 : 1
}

/// Start a screen recording session.
/// Returns 0 on success, non-zero on failure.
@_cdecl("hwledger_sck_start_recording")
public func startRecording(
    appBundleIdCStr: UnsafePointer<UInt8>,
    outputPathCStr: UnsafePointer<UInt8>,
    width: UInt32,
    height: UInt32,
    fps: UInt32
) -> Int32 {
    let appBundleId = String(cString: appBundleIdCStr)
    let outputPath = String(cString: outputPathCStr)

    Task {
        do {
            try await performStartRecording(
                appBundleId: appBundleId,
                outputPath: outputPath,
                width: width,
                height: height,
                fps: fps
            )
        } catch {
            NSLog("SCK start recording error: %@", error.localizedDescription)
            recordingSession.recordingDenied = true
        }
    }

    return 0
}

/// Stop the active screen recording session.
/// Returns 0 on success, non-zero on failure.
@_cdecl("hwledger_sck_stop_recording")
public func stopRecording() -> Int32 {
    guard recordingSession.isRecording else {
        return 1 // Not recording
    }

    Task {
        do {
            try await performStopRecording()
        } catch {
            NSLog("SCK stop recording error: %@", error.localizedDescription)
        }
    }

    return 0
}

// MARK: - Implementation

private func performStartRecording(
    appBundleId: String,
    outputPath: String,
    width: UInt32,
    height: UInt32,
    fps: UInt32
) async throws {
    // Check permission
    let availableContent = try await SCShareableContent.current

    // Find the target app window
    var targetWindow: SCWindow?
    for window in availableContent.windows {
        if window.owningApplication?.bundleIdentifier == appBundleId {
            targetWindow = window
            break
        }
    }

    guard let targetWindow = targetWindow else {
        throw RecorderError.windowNotFound(appBundleId)
    }

    // Create content filter for the app window
    let contentFilter = SCContentFilter(desktopWindows: [], applications: [targetWindow.owningApplication!], excludingWindows: [])

    // Configure stream
    let streamConfig = SCStreamConfiguration()
    streamConfig.sourceResolution = false
    streamConfig.width = Int(width)
    streamConfig.height = Int(height)
    streamConfig.frameRate = Int(fps)

    // Remove existing file
    try? FileManager.default.removeItem(atPath: outputPath)

    // Initialize asset writer
    let outputURL = URL(fileURLWithPath: outputPath)
    let assetWriter = try AVAssetWriter(outputURL: outputURL, fileType: .mp4)

    let videoSettings: [String: Any] = [
        AVVideoCodecKey: AVVideoCodecType.h264,
        AVVideoWidthKey: width,
        AVVideoHeightKey: height
    ]

    let videoInput = AVAssetWriterInput(mediaType: .video, outputSettings: videoSettings)
    videoInput.expectsMediaDataInRealTime = true

    guard assetWriter.canAdd(videoInput) else {
        throw RecorderError.cannotAddVideoInput
    }

    assetWriter.add(videoInput)

    guard assetWriter.startWriting() else {
        throw RecorderError.cannotStartWriting
    }

    assetWriter.startSession(atSourceTime: CMTime.zero)

    // Create stream delegate
    let delegate = StreamDelegate(assetWriter: assetWriter, videoInput: videoInput)

    // Create and start stream
    let stream = SCStream(filter: contentFilter, configuration: streamConfig, delegate: delegate)

    try await stream.startCapture()

    recordingSession.stream = stream
    recordingSession.assetWriter = assetWriter
    recordingSession.videoInput = videoInput
    recordingSession.isRecording = true

    NSLog("SCK recording started: %@", outputPath)
}

private func performStopRecording() async throws {
    guard recordingSession.isRecording else {
        throw RecorderError.notRecording
    }

    recordingSession.isRecording = false

    if let stream = recordingSession.stream {
        try await stream.stopCapture()
        recordingSession.stream = nil
    }

    if let assetWriter = recordingSession.assetWriter {
        recordingSession.videoInput?.markAsFinished()
        await assetWriter.finishWriting()
        recordingSession.assetWriter = nil
        recordingSession.videoInput = nil
    }

    NSLog("SCK recording stopped")
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

private enum RecorderError: LocalizedError {
    case windowNotFound(String)
    case cannotAddVideoInput
    case cannotStartWriting
    case notRecording

    var errorDescription: String? {
        switch self {
        case .windowNotFound(let appId):
            return "Window not found for app: \(appId)"
        case .cannotAddVideoInput:
            return "Cannot add video input to asset writer"
        case .cannotStartWriting:
            return "Cannot start writing to asset"
        case .notRecording:
            return "Recording not in progress"
        }
    }
}
