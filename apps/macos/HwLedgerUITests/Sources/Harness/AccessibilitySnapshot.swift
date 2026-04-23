import Foundation
#if canImport(ApplicationServices)
import ApplicationServices
#endif

/// Abstraction over the "thing a snapshot is captured from". Lets unit tests
/// pass in a synthetic tree instead of a live AXUIElement. XCUITest callers
/// wrap `AXUIElement` via `AXNodeSource.ax(...)`.
public protocol AccessibilityNodeSource {
    /// Render this node and its descendants into a deterministic JSON-ready
    /// structure. Implementations should limit depth to avoid loops.
    func snapshotTree() -> AccessibilityNode
}

/// Flat, Codable tree node. Matches the subset of AX attributes the
/// downstream VLM-judge pipeline consumes (role, identifier, value, label,
/// children).
public struct AccessibilityNode: Codable, Equatable {
    public let role: String
    public let identifier: String?
    public let label: String?
    public let value: String?
    public let children: [AccessibilityNode]

    public init(role: String, identifier: String? = nil, label: String? = nil,
                value: String? = nil, children: [AccessibilityNode] = []) {
        self.role = role
        self.identifier = identifier
        self.label = label
        self.value = value
        self.children = children
    }
}

/// Utility that persists snapshots to disk and returns a relative
/// `structural_path` pointer suitable for embedding in a
/// `PhenotypeManifestVerified`.
public enum AccessibilitySnapshot {

    /// Capture `source` and write it to `rootDir/snapshots/<name>.json`.
    /// Returns the path relative to `rootDir` (e.g. `snapshots/launch.json`).
    @discardableResult
    public static func capture(
        _ source: AccessibilityNodeSource,
        named name: String,
        rootDir: URL
    ) throws -> String {
        let tree = source.snapshotTree()
        let dir = rootDir.appendingPathComponent("snapshots")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let filename = "\(sanitize(name)).json"
        let out = dir.appendingPathComponent(filename)
        let enc = JSONEncoder()
        enc.outputFormatting = [.prettyPrinted, .sortedKeys]
        let data = try enc.encode(tree)
        try data.write(to: out)
        return "snapshots/\(filename)"
    }

    private static func sanitize(_ s: String) -> String {
        var out = ""
        for ch in s.lowercased() {
            if ch.isLetter || ch.isNumber { out.append(ch) }
            else if ch == " " || ch == "-" || ch == "_" { out.append("-") }
        }
        while out.contains("--") { out = out.replacingOccurrences(of: "--", with: "-") }
        return out.trimmingCharacters(in: CharacterSet(charactersIn: "-"))
    }
}

#if canImport(ApplicationServices)
/// Live AX node source wrapping an `AXUIElement`. Not used by unit tests.
public struct AXElementNodeSource: AccessibilityNodeSource {
    public let element: AXUIElement
    public let maxDepth: Int

    public init(element: AXUIElement, maxDepth: Int = 8) {
        self.element = element
        self.maxDepth = maxDepth
    }

    public func snapshotTree() -> AccessibilityNode {
        return walk(element, depth: 0)
    }

    private func walk(_ el: AXUIElement, depth: Int) -> AccessibilityNode {
        let role = copyAttr(el, kAXRoleAttribute as CFString) ?? "AXUnknown"
        let identifier = copyAttr(el, kAXIdentifierAttribute as CFString)
        let label = copyAttr(el, kAXTitleAttribute as CFString)
        let value = copyAttr(el, kAXValueAttribute as CFString)

        var kids: [AccessibilityNode] = []
        if depth < maxDepth {
            var childrenRef: CFTypeRef?
            AXUIElementCopyAttributeValue(el, kAXChildrenAttribute as CFString, &childrenRef)
            if let arr = childrenRef as? [AXUIElement] {
                kids = arr.map { walk($0, depth: depth + 1) }
            }
        }
        return AccessibilityNode(role: role, identifier: identifier, label: label, value: value, children: kids)
    }

    private func copyAttr(_ el: AXUIElement, _ key: CFString) -> String? {
        var out: CFTypeRef?
        let err = AXUIElementCopyAttributeValue(el, key, &out)
        guard err == .success else { return nil }
        if let s = out as? String { return s }
        if let n = out as? NSNumber { return n.stringValue }
        return nil
    }
}
#endif
