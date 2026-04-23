//
//  AccessibilitySnapshot.swift
//  HwLedgerUITests / Tier 0 structural-capture (macOS family).
//
//  Recursive walker over an XCUIElement-compatible tree, producing a
//  `<frame-id>.structural.json` sibling to each journey keyframe.
//
//  Contract (`StructuralNode`):
//
//    {
//      "family": "macos",
//      "role": "Button",
//      "label": "Run planner",
//      "value": null,
//      "identifier": "planner.run",
//      "frame": { "x": 420.0, "y": 612.0, "w": 110.0, "h": 32.0 },
//      "children": [ ... ]
//    }
//
//  The snapshot is XCUIElement-agnostic: we abstract behind the
//  `AccessibilityNodeSource` protocol so the Swift tests can run against a
//  mocked in-memory tree without a live XCUIApplication (TCC is required to
//  execute XCUITests on this host; see Journey.swift for the production
//  hook). In production, an XCUIElement adapter maps:
//    role        <- elementType.rawValue stringified
//    label       <- label
//    value       <- value? as? String
//    identifier  <- identifier
//    frame       <- frame
//    children    <- children(matching: .any).allElementsBoundByIndex
//
//  Traces to: Tier 0 structural-capture (macOS family).

import Foundation
import CoreGraphics

/// Source of accessibility metadata for a single node in the tree. Both an
/// XCUIElement adapter and the in-memory mock implement this protocol.
public protocol AccessibilityNodeSource {
    /// Role / element type (e.g. "Button", "Window", "StaticText").
    var accRole: String { get }
    /// Spoken label (e.g. the button's title).
    var accLabel: String { get }
    /// Current value (e.g. text-field contents); may be nil.
    var accValue: String? { get }
    /// Developer-assigned identifier / accessibilityIdentifier.
    var accIdentifier: String { get }
    /// Screen-space frame in points.
    var accFrame: CGRect { get }
    /// Direct children in tree order.
    var accChildren: [any AccessibilityNodeSource] { get }
}

/// Serialised rectangle (x, y, w, h in points, truncated to 1 decimal).
public struct StructuralFrame: Codable, Equatable {
    public let x: Double
    public let y: Double
    public let w: Double
    public let h: Double

    public init(_ r: CGRect) {
        self.x = Self.round1(r.origin.x)
        self.y = Self.round1(r.origin.y)
        self.w = Self.round1(r.size.width)
        self.h = Self.round1(r.size.height)
    }

    public init(x: Double, y: Double, w: Double, h: Double) {
        self.x = x; self.y = y; self.w = w; self.h = h
    }

    private static func round1(_ v: CGFloat) -> Double {
        (Double(v) * 10).rounded() / 10
    }
}

/// One node of the accessibility tree as serialised to disk.
public struct StructuralNode: Codable, Equatable {
    public let family: String
    public let role: String
    public let label: String
    public let value: String?
    public let identifier: String
    public let frame: StructuralFrame
    public let children: [StructuralNode]

    public init(
        role: String,
        label: String,
        value: String?,
        identifier: String,
        frame: StructuralFrame,
        children: [StructuralNode]
    ) {
        self.family = "macos"
        self.role = role
        self.label = label
        self.value = value
        self.identifier = identifier
        self.frame = frame
        self.children = children
    }
}

/// Configurable walker — the root is only emitted with `family: "macos"` at
/// depth zero; children inherit the same family implicitly via the wrapping
/// StructuralNode.
public enum AccessibilitySnapshot {
    /// Walk the tree rooted at `source` and produce a single StructuralNode.
    public static func walk(_ source: any AccessibilityNodeSource) -> StructuralNode {
        let kids = source.accChildren.map { walk($0) }
        return StructuralNode(
            role: source.accRole,
            label: source.accLabel,
            value: source.accValue,
            identifier: source.accIdentifier,
            frame: StructuralFrame(source.accFrame),
            children: kids
        )
    }

    /// Serialize the tree to a pretty-printed, sorted-key JSON string.
    public static func encode(_ root: StructuralNode) throws -> Data {
        let enc = JSONEncoder()
        enc.outputFormatting = [.prettyPrinted, .sortedKeys]
        return try enc.encode(root)
    }

    /// Write a snapshot sibling next to a keyframe PNG:
    ///   keyframes/frame_003.png
    ///   keyframes/frame_003.structural.json
    public static func writeSibling(
        keyframePath: URL,
        source: any AccessibilityNodeSource
    ) throws -> URL {
        let tree = walk(source)
        let data = try encode(tree)
        let siblingPath = keyframePath
            .deletingPathExtension()
            .appendingPathExtension("structural.json")
        try data.write(to: siblingPath, options: .atomic)
        return siblingPath
    }
}

// MARK: - In-memory mock (used by Swift tests; reusable in fixtures)

/// Lightweight concrete node the tests construct directly. In production a
/// thin adapter over XCUIElement would conform to AccessibilityNodeSource;
/// this type lets us unit-test the walker with no XCTest host.
public final class MockAccessibilityNode: AccessibilityNodeSource {
    public var accRole: String
    public var accLabel: String
    public var accValue: String?
    public var accIdentifier: String
    public var accFrame: CGRect
    public var accChildren: [any AccessibilityNodeSource]

    public init(
        role: String,
        label: String = "",
        value: String? = nil,
        identifier: String = "",
        frame: CGRect = .zero,
        children: [MockAccessibilityNode] = []
    ) {
        self.accRole = role
        self.accLabel = label
        self.accValue = value
        self.accIdentifier = identifier
        self.accFrame = frame
        self.accChildren = children
    }
}
