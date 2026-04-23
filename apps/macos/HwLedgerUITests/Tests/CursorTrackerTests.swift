//
//  CursorTrackerTests.swift
//  Verifies Deliverable 2 (XCUITest cursor-track capture) parallel to the
//  Playwright D3 side: JSONL schema parity, chronological ordering,
//  ts_ms rebased to journey start, synthetic tap-site instrumentation.
//  Traces to: feat/annotations-cursor-visible — D2.
//

import Foundation
import Testing
import CoreGraphics
@testable import HwLedgerUITestHarness

@Test("CursorTrackEvent encodes to the Playwright JSONL schema")
func testCursorTrackEventSchema() throws {
    let event = CursorTrackEvent(ts_ms: 123, x: 42.5, y: 77.0, action: "click")
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys]
    let data = try encoder.encode(event)
    let line = String(data: data, encoding: .utf8) ?? ""
    // Keys must match the Playwright side byte-for-byte so the Remotion
    // CursorOverlay needs zero changes.
    #expect(line.contains("\"ts_ms\":123"))
    #expect(line.contains("\"x\":42.5"))
    #expect(line.contains("\"y\":77"))
    #expect(line.contains("\"action\":\"click\""))
}

@Test("recordSynthetic appends events in chronological order")
func testRecordSyntheticOrdering() throws {
    let tracker = CursorTracker(startTime: Date())
    tracker.recordSynthetic(action: "move", at: CGPoint(x: 10, y: 10))
    Thread.sleep(forTimeInterval: 0.01)
    tracker.recordSynthetic(action: "click", at: CGPoint(x: 20, y: 20))
    Thread.sleep(forTimeInterval: 0.01)
    tracker.recordSynthetic(action: "release", at: CGPoint(x: 20, y: 20))

    let events = tracker.snapshot()
    #expect(events.count == 3)
    #expect(events[0].action == "move")
    #expect(events[1].action == "click")
    #expect(events[2].action == "release")
    // Monotonic non-decreasing.
    #expect(events[0].ts_ms <= events[1].ts_ms)
    #expect(events[1].ts_ms <= events[2].ts_ms)
    // Rebased to journey start (not wall-clock epoch).
    #expect(events[0].ts_ms < 10_000)
}

@Test("writeJSONL produces one-event-per-line, newline-terminated, parseable")
func testWriteJSONL() throws {
    let tracker = CursorTracker(startTime: Date())
    tracker.recordSynthetic(action: "move", at: CGPoint(x: 1, y: 2))
    tracker.recordSynthetic(action: "click", at: CGPoint(x: 3, y: 4))
    tracker.recordSynthetic(action: "release", at: CGPoint(x: 3, y: 4))

    let tmp = FileManager.default.temporaryDirectory
        .appendingPathComponent("hwl-cursor-\(UUID().uuidString)")
    try FileManager.default.createDirectory(at: tmp, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: tmp) }

    try tracker.writeJSONL(to: tmp)

    let url = tmp.appendingPathComponent("cursor-track.jsonl")
    let contents = try String(contentsOf: url, encoding: .utf8)
    #expect(contents.hasSuffix("\n"))
    let lines = contents.split(separator: "\n", omittingEmptySubsequences: false)
        .filter { !$0.isEmpty }
    #expect(lines.count == 3)

    // Each line must parse as a CursorTrackEvent with valid action.
    let decoder = JSONDecoder()
    var last: Int = -1
    for raw in lines {
        let data = raw.data(using: .utf8)!
        let ev = try decoder.decode(CursorTrackEvent.self, from: data)
        #expect(["move", "click", "release"].contains(ev.action))
        #expect(ev.ts_ms >= last)  // chronologically ordered
        last = ev.ts_ms
    }
}

@Test("writeJSONL is a no-op when no events were recorded")
func testWriteJSONLEmpty() throws {
    let tracker = CursorTracker(startTime: Date())
    let tmp = FileManager.default.temporaryDirectory
        .appendingPathComponent("hwl-cursor-empty-\(UUID().uuidString)")
    try FileManager.default.createDirectory(at: tmp, withIntermediateDirectories: true)
    defer { try? FileManager.default.removeItem(at: tmp) }

    try tracker.writeJSONL(to: tmp)
    let url = tmp.appendingPathComponent("cursor-track.jsonl")
    #expect(!FileManager.default.fileExists(atPath: url.path))
}
