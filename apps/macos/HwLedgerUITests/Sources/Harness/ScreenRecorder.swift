import Foundation
import HwLedgerGuiRecorder

/// ScreenRecorder is a re-export of ScreenCaptureRecorder from HwLedgerGuiRecorder.
/// It wraps the Rust+Swift FFI bridge for ScreenCaptureKit recording via hwledger-gui-recorder.
///
/// Usage:
/// ```swift
/// let recorder = ScreenRecorder(outputPath: URL(fileURLWithPath: "/tmp/recording.mp4"))
/// try await recorder.startRecording(appIdentifier: "com.kooshapari.hwLedger")
/// // ... interact with app ...
/// let recordingPath = try await recorder.stopRecording()
/// ```
public typealias ScreenRecorder = ScreenCaptureRecorder
