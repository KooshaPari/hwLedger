//
//  AccessibilitySnapshotTests.swift
//  Verifies Tier 0 structural-capture (macOS family) against a mocked tree.
//  Traces to: Tier 0 structural-capture (macOS family).
//

import Foundation
import Testing
@testable import HwLedgerUITestHarness

@Test("walker produces nested StructuralNode with correct role/label/frame")
func testWalkerNestedTree() throws {
    let root = MockAccessibilityNode(
        role: "Window",
        label: "hwLedger",
        identifier: "main.window",
        frame: CGRect(x: 0, y: 0, width: 1280, height: 800),
        children: [
            MockAccessibilityNode(
                role: "Button",
                label: "Run planner",
                identifier: "planner.run",
                frame: CGRect(x: 420, y: 612, width: 110, height: 32)
            ),
            MockAccessibilityNode(
                role: "TextField",
                label: "Sequence length",
                value: "4096",
                identifier: "planner.seqlen",
                frame: CGRect(x: 220, y: 180, width: 200, height: 24)
            )
        ]
    )
    let node = AccessibilitySnapshot.walk(root)
    #expect(node.family == "macos")
    #expect(node.role == "Window")
    #expect(node.label == "hwLedger")
    #expect(node.identifier == "main.window")
    #expect(node.frame == StructuralFrame(x: 0, y: 0, w: 1280, h: 800))
    #expect(node.children.count == 2)
    #expect(node.children[0].role == "Button")
    #expect(node.children[0].identifier == "planner.run")
    #expect(node.children[1].value == "4096")
}

@Test("encoded JSON is stable, sorted, includes `family: macos`")
func testEncodeStableShape() throws {
    let leaf = MockAccessibilityNode(
        role: "Button",
        label: "Go",
        identifier: "go.btn",
        frame: CGRect(x: 1, y: 2, width: 3, height: 4)
    )
    let node = AccessibilitySnapshot.walk(leaf)
    let data = try AccessibilitySnapshot.encode(node)
    let json = String(data: data, encoding: .utf8) ?? ""
    // sortedKeys output groups alphabetically, so the family key is present
    // and the frame has all four members.
    #expect(json.contains("\"family\" : \"macos\""))
    #expect(json.contains("\"role\" : \"Button\""))
    #expect(json.contains("\"identifier\" : \"go.btn\""))
    #expect(json.contains("\"x\" : 1"))
    #expect(json.contains("\"h\" : 4"))
    // Decode round-trip — same shape back.
    let decoded = try JSONDecoder().decode(StructuralNode.self, from: data)
    #expect(decoded == node)
}
