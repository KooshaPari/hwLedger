import Foundation
import AppKit
import CoreGraphics

/// Single cursor-track event mirroring the Playwright JSONL schema
/// produced by `apps/streamlit/journeys/lib/journey.ts`:
///
/// ```
/// {"ts_ms": <int>, "x": <px>, "y": <px>, "action": "move"|"click"|"release"}
/// ```
///
/// Deliverable 2 of the `feat/annotations-cursor-visible` mandate: mirror
/// D3 (Playwright) so the Remotion `CursorOverlay` consumes identical bytes
/// regardless of which harness produced the recording.
public struct CursorTrackEvent: Codable, Equatable {
    public let ts_ms: Int
    public let x: Double
    public let y: Double
    public let action: String  // "move" | "click" | "release"

    public init(ts_ms: Int, x: Double, y: Double, action: String) {
        self.ts_ms = ts_ms
        self.x = x
        self.y = y
        self.action = action
    }
}

/// Thread-safe cursor-event recorder.
///
/// Captures two distinct event sources:
///
/// 1. **Real HID cursor motion** via a passive `CGEvent` tap (listenOnly),
///    picking up `.mouseMoved`, `.leftMouseDown`, `.leftMouseUp`. This is
///    the macOS equivalent of Playwright's in-page `mousemove` listener.
///    A passive tap does NOT require Accessibility permissions beyond what
///    the harness already has, and never blocks events. We prefer a global
///    CGEvent tap over `NSEvent.addGlobalMonitor` because the latter does
///    not fire for `.mouseMoved` without the app being foregrounded with
///    focus — the tap is robust across app-switch transitions that happen
///    during XCUI-style journeys.
/// 2. **Synthetic tap coordinates** from the XCUI driver itself. When
///    `AppDriver.clickElementViaCoordinates` posts a synthesized click via
///    `CGEvent`, those synthesized events *do* flow back through the HID
///    tap on macOS 14+ — but we additionally call
///    `recordSynthetic(action:at:)` at the tap site so that:
///      - the schema remains deterministic on CI where the tap API may be
///        denied without a loud failure,
///      - click/release events are guaranteed even if the tap is blocked
///        by a future OS tightening.
///
/// Timestamps are rebased to the tracker's `startTime`, matching the
/// Playwright `recorderStartMs` anchor.
public final class CursorTracker {
    private let queue = DispatchQueue(label: "hwledger.cursortracker", qos: .userInitiated)
    private var events: [CursorTrackEvent] = []
    private let startTime: Date
    private var eventTap: CFMachPort?
    private var runLoopSource: CFRunLoopSource?

    public init(startTime: Date = Date()) {
        self.startTime = startTime
    }

    /// Install a passive global event tap. Safe to call more than once;
    /// subsequent calls are no-ops.
    public func start() {
        queue.sync {
            guard self.eventTap == nil else { return }
            let mask: CGEventMask =
                (1 << CGEventType.mouseMoved.rawValue) |
                (1 << CGEventType.leftMouseDown.rawValue) |
                (1 << CGEventType.leftMouseUp.rawValue) |
                (1 << CGEventType.leftMouseDragged.rawValue)

            let selfPtr = Unmanaged.passUnretained(self).toOpaque()
            guard let tap = CGEvent.tapCreate(
                tap: .cgSessionEventTap,
                place: .headInsertEventTap,
                options: .listenOnly,
                eventsOfInterest: mask,
                callback: { _, type, cgEvent, refcon in
                    guard let refcon = refcon else {
                        return Unmanaged.passUnretained(cgEvent)
                    }
                    let tracker = Unmanaged<CursorTracker>.fromOpaque(refcon).takeUnretainedValue()
                    let loc = cgEvent.location
                    let action: String
                    switch type {
                    case .leftMouseDown:
                        action = "click"
                    case .leftMouseUp:
                        action = "release"
                    default:
                        action = "move"
                    }
                    tracker.append(x: Double(loc.x), y: Double(loc.y), action: action)
                    return Unmanaged.passUnretained(cgEvent)
                },
                userInfo: selfPtr
            ) else {
                // Tap creation can fail without Input Monitoring permission.
                // The harness still gets synthetic tap events via
                // recordSynthetic(...), so we continue silently.
                print("CursorTracker: CGEvent.tapCreate failed (missing Input Monitoring permission?); synthetic-only mode.")
                return
            }
            let source = CFMachPortCreateRunLoopSource(kCFAllocatorDefault, tap, 0)
            CFRunLoopAddSource(CFRunLoopGetMain(), source, .commonModes)
            CGEvent.tapEnable(tap: tap, enable: true)
            self.eventTap = tap
            self.runLoopSource = source
        }
    }

    /// Disable the tap and release its run-loop source.
    public func stop() {
        queue.sync {
            if let tap = self.eventTap {
                CGEvent.tapEnable(tap: tap, enable: false)
            }
            if let source = self.runLoopSource {
                CFRunLoopRemoveSource(CFRunLoopGetMain(), source, .commonModes)
            }
            self.eventTap = nil
            self.runLoopSource = nil
        }
    }

    /// Append a synthetic event at a known coordinate. Called from
    /// `AppDriver.clickElementViaCoordinates` so the JSONL is complete
    /// even if the global tap is denied by OS policy.
    public func recordSynthetic(action: String, at point: CGPoint) {
        append(x: Double(point.x), y: Double(point.y), action: action)
    }

    /// Snapshot a copy of recorded events (for testing and finalize).
    public func snapshot() -> [CursorTrackEvent] {
        queue.sync { Array(events) }
    }

    /// Serialize recorded events to `<dir>/cursor-track.jsonl` in the
    /// schema consumed by Remotion `CursorOverlay`. One JSON object per
    /// line, newline-terminated. No-op when the log is empty.
    public func writeJSONL(to directory: URL) throws {
        let snap = snapshot()
        guard !snap.isEmpty else { return }
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys]
        var lines: [String] = []
        lines.reserveCapacity(snap.count)
        for ev in snap {
            let data = try encoder.encode(ev)
            guard let line = String(data: data, encoding: .utf8) else { continue }
            lines.append(line)
        }
        let payload = lines.joined(separator: "\n") + "\n"
        let url = directory.appendingPathComponent("cursor-track.jsonl")
        try payload.data(using: .utf8)?.write(to: url)
    }

    private func append(x: Double, y: Double, action: String) {
        let ms = Int(Date().timeIntervalSince(startTime) * 1000.0)
        let event = CursorTrackEvent(ts_ms: max(0, ms), x: x, y: y, action: action)
        queue.sync {
            events.append(event)
        }
    }
}
