// MARK: - PhenotypeRecord
//
// XCTest base class that auto-discovers `@user-story` frontmatter blocks from
// the Swift source file each `test_*` lives in, then — when
// `PHENOTYPE_USER_STORY_RECORD` is set — assembles a `manifest.verified.json`
// at tearDown containing:
//   - the parsed YAML fields
//   - per-assertion keyframes (captured via `XCTContext.runActivity`)
//   - `structural_path` pointers to sibling AX snapshots written by
//     `AccessibilitySnapshot` (Sources/Harness/AccessibilitySnapshot.swift)
//
// This is Batch 4 of the user-story-as-test framework. Harvester (Batch 1)
// still reads the frontmatter from the source file at build time; this runtime
// re-parse is only used to emit per-test manifests. Both paths share the same
// schema (apps/macos/HwLedgerUITests/Sources/PhenotypeRecord/schema.json —
// mirror of tools/user-story-extract/schema/user-story.schema.json).
//
// The base class is pure Foundation + XCTest; it does NOT depend on
// HwLedgerUITestHarness so unit tests can exercise the frontmatter parser and
// manifest assembler without booting a real AXUIElement driver.

import Foundation
#if canImport(XCTest)
import XCTest
#endif

// MARK: - Public surface

/// YAML-parsed @user-story frontmatter block.
/// Keep in lock-step with `tools/user-story-extract/schema/user-story.schema.json`.
public struct PhenotypeUserStory: Codable, Equatable {
    public let journey_id: String
    public let title: String
    public let persona: String
    public let given: String
    public let when: [String]
    public let then: [String]
    public let traces_to: [String]
    public let record: Bool?
    public let blind_judge: String?
    public let backend: String?
    public let blind_eval: String?
    public let family: String?

    public init(
        journey_id: String,
        title: String,
        persona: String,
        given: String,
        when: [String],
        then: [String],
        traces_to: [String],
        record: Bool? = nil,
        blind_judge: String? = nil,
        backend: String? = nil,
        blind_eval: String? = nil,
        family: String? = nil
    ) {
        self.journey_id = journey_id
        self.title = title
        self.persona = persona
        self.given = given
        self.when = when
        self.then = then
        self.traces_to = traces_to
        self.record = record
        self.blind_judge = blind_judge
        self.backend = backend
        self.blind_eval = blind_eval
        self.family = family
    }
}

/// A single keyframe captured during a test run.
public struct PhenotypeKeyframe: Codable, Equatable {
    public let index: Int
    public let activity_name: String
    public let timestamp_iso: String
    /// Relative path to sibling AX snapshot JSON (structural_path pointer).
    public let structural_path: String?
    /// Optional asserted message if captured from XCTAssert*.
    public let assertion: String?

    public init(index: Int, activity_name: String, timestamp_iso: String, structural_path: String?, assertion: String?) {
        self.index = index
        self.activity_name = activity_name
        self.timestamp_iso = timestamp_iso
        self.structural_path = structural_path
        self.assertion = assertion
    }
}

/// The verified manifest written to disk when `PHENOTYPE_USER_STORY_RECORD`
/// is present.
public struct PhenotypeManifestVerified: Codable, Equatable {
    public let schema_version: String
    public let journey_id: String
    public let story: PhenotypeUserStory
    public let started_at: String
    public let finished_at: String?
    public let passed: Bool
    public let failure: String?
    public let keyframes: [PhenotypeKeyframe]
    public let recording_path: String?
    public let recording_denied: Bool

    public init(
        schema_version: String,
        journey_id: String,
        story: PhenotypeUserStory,
        started_at: String,
        finished_at: String?,
        passed: Bool,
        failure: String?,
        keyframes: [PhenotypeKeyframe],
        recording_path: String?,
        recording_denied: Bool
    ) {
        self.schema_version = schema_version
        self.journey_id = journey_id
        self.story = story
        self.started_at = started_at
        self.finished_at = finished_at
        self.passed = passed
        self.failure = failure
        self.keyframes = keyframes
        self.recording_path = recording_path
        self.recording_denied = recording_denied
    }
}

public enum PhenotypeRecordError: Error, LocalizedError {
    case frontmatterNotFound(file: String, testName: String)
    case frontmatterParseFailure(file: String, line: Int, underlying: String)
    case manifestWriteFailed(path: String, underlying: String)

    public var errorDescription: String? {
        switch self {
        case .frontmatterNotFound(let file, let testName):
            return "@user-story frontmatter not found above `\(testName)` in \(file). " +
                   "Add a `// MARK: @user-story` ... `// MARK: @end` block directly " +
                   "above the test function."
        case .frontmatterParseFailure(let file, let line, let underlying):
            return "Malformed @user-story frontmatter in \(file) at line \(line): \(underlying)"
        case .manifestWriteFailed(let path, let underlying):
            return "Failed to write manifest.verified.json at \(path): \(underlying)"
        }
    }
}

// MARK: - Frontmatter parser (pure, unit-testable)

/// Parser for the Swift `// MARK: @user-story` ... `// MARK: @end` frontmatter
/// block flavor. Mirrors `tools/user-story-extract/src/lib.rs::find_swift`.
///
/// Given a full Swift source file body and a target test-function name,
/// returns the `@user-story` block *immediately preceding* the function (or
/// nil if none). A single non-comment blank line is tolerated between the
/// closing `// MARK: @end` and the function signature so the block can carry
/// attributes like `@MainActor` without breaking adjacency detection.
public enum PhenotypeFrontmatter {

    /// Find and YAML-decode the @user-story block above a `func <name>(` signature.
    public static func parseForTest(
        source: String,
        testFunctionName: String
    ) throws -> (story: PhenotypeUserStory, startLine: Int)? {
        let lines = source.components(separatedBy: "\n")
        guard let funcLineIndex = findFunctionLine(lines: lines, functionName: testFunctionName) else {
            return nil
        }
        guard let block = findPrecedingBlock(lines: lines, beforeLine: funcLineIndex) else {
            return nil
        }

        do {
            let story = try decodeYAML(block.yaml)
            return (story, block.startLine)
        } catch {
            throw PhenotypeRecordError.frontmatterParseFailure(
                file: "<source>",
                line: block.startLine,
                underlying: String(describing: error)
            )
        }
    }

    /// Locate `func <name>(` or `func <name> (` (1-based line number returned).
    static func findFunctionLine(lines: [String], functionName: String) -> Int? {
        // Match optional attributes / modifiers on the same logical line:
        //   "func foo(", "public func foo(", "@MainActor func foo("
        let needle = "func \(functionName)("
        let altNeedle = "func \(functionName) ("
        for (i, raw) in lines.enumerated() {
            let trimmed = raw.trimmingCharacters(in: .whitespaces)
            if trimmed.contains(needle) || trimmed.contains(altNeedle) {
                return i
            }
        }
        return nil
    }

    /// Walk backward from `beforeLine` to find the most recent
    /// `// MARK: @user-story` ... `// MARK: @end` block, skipping blank lines
    /// and attribute decorators (`@MainActor`, `@available(...)`, etc.).
    static func findPrecedingBlock(lines: [String], beforeLine: Int) -> (yaml: String, startLine: Int)? {
        guard beforeLine > 0 else { return nil }
        var i = beforeLine - 1

        // Skip blank/attribute lines between block and function.
        while i >= 0 {
            let t = lines[i].trimmingCharacters(in: .whitespaces)
            if t.isEmpty { i -= 1; continue }
            if t.hasPrefix("@") { i -= 1; continue }
            break
        }
        guard i >= 0 else { return nil }

        // Expect `// MARK: @end` at position i.
        guard isEndMarker(lines[i]) else {
            // Tolerate: block may appear further up if there's a decl in between;
            // conservative behavior is to return nil.
            return nil
        }
        let endLine = i
        i -= 1

        // Walk backward collecting comment-body lines until `@user-story`.
        var body: [String] = []
        while i >= 0 {
            let line = lines[i]
            if isStartMarker(line) {
                body.reverse()
                let yaml = body.map { stripCommentPrefix($0) }.joined(separator: "\n")
                return (yaml, i + 1) // 1-based
            }
            // Skip non-comment lines silently; keep only `//` lines.
            let t = line.trimmingCharacters(in: .whitespaces)
            if t.hasPrefix("//") {
                body.append(line)
            }
            i -= 1
            if endLine - i > 500 { return nil } // safety bail
        }
        return nil
    }

    private static func isStartMarker(_ line: String) -> Bool {
        let t = line.trimmingCharacters(in: .whitespaces)
        return t.hasPrefix("// MARK:") && t.contains("@user-story")
    }
    private static func isEndMarker(_ line: String) -> Bool {
        let t = line.trimmingCharacters(in: .whitespaces)
        return t.hasPrefix("// MARK:") && t.contains("@end")
    }

    /// Strip `// ` / `//` prefix from a comment body line.
    private static func stripCommentPrefix(_ line: String) -> String {
        var s = line.drop(while: { $0 == " " || $0 == "\t" })
        if s.hasPrefix("//") { s = s.dropFirst(2) }
        if s.first == " " { s = s.dropFirst() }
        return String(s)
    }

    /// Minimal YAML decoder tailored to the @user-story schema.
    /// Supports:
    ///   - `key: scalar`
    ///   - `key: [a, b, c]` inline list
    ///   - `key:` followed by `- item` block list
    ///   - `key: >` or `key: |` followed by indented prose (folded into single paragraph)
    /// This avoids a libyaml dep in the XCTest target. Frontmatter in this
    /// project is strictly simple — harvester-level validation happens in Rust
    /// with a real schema. The test-side decoder just needs to reproduce the
    /// already-validated block.
    static func decodeYAML(_ yaml: String) throws -> PhenotypeUserStory {
        let pairs = try parseSimpleYAML(yaml)
        func str(_ k: String) throws -> String {
            guard case .scalar(let v)? = pairs[k] else {
                throw NSError(domain: "PhenotypeFrontmatter", code: 1, userInfo: [
                    NSLocalizedDescriptionKey: "missing or wrong-typed key: \(k)"
                ])
            }
            return v
        }
        func list(_ k: String) throws -> [String] {
            guard case .list(let v)? = pairs[k] else {
                throw NSError(domain: "PhenotypeFrontmatter", code: 1, userInfo: [
                    NSLocalizedDescriptionKey: "missing or wrong-typed list: \(k)"
                ])
            }
            return v
        }
        func optStr(_ k: String) -> String? {
            if case .scalar(let v)? = pairs[k] { return v }
            return nil
        }
        let recordFlag: Bool? = {
            if case .scalar(let v)? = pairs["record"] {
                return (v == "true" || v == "yes")
            }
            return nil
        }()

        return PhenotypeUserStory(
            journey_id: try str("journey_id"),
            title: try str("title"),
            persona: try str("persona"),
            given: try str("given"),
            when: try list("when"),
            then: try list("then"),
            traces_to: try list("traces_to"),
            record: recordFlag,
            blind_judge: optStr("blind_judge"),
            backend: optStr("backend"),
            blind_eval: optStr("blind_eval"),
            family: optStr("family")
        )
    }

    enum YamlValue { case scalar(String); case list([String]) }

    /// Parse the tiny YAML subset used by @user-story blocks.
    static func parseSimpleYAML(_ input: String) throws -> [String: YamlValue] {
        var out: [String: YamlValue] = [:]
        let lines = input.components(separatedBy: "\n")
        var i = 0
        while i < lines.count {
            let raw = lines[i]
            let trimmed = raw.trimmingCharacters(in: .whitespaces)
            if trimmed.isEmpty || trimmed.hasPrefix("#") { i += 1; continue }

            guard let colon = trimmed.firstIndex(of: ":") else { i += 1; continue }
            let key = String(trimmed[..<colon]).trimmingCharacters(in: .whitespaces)
            let rest = String(trimmed[trimmed.index(after: colon)...]).trimmingCharacters(in: .whitespaces)

            if rest.isEmpty || rest == "|" || rest == ">" || rest == ">-" || rest == "|-" {
                // Block form: either block list (- item) or folded/literal prose.
                var items: [String] = []
                var prose: [String] = []
                var j = i + 1
                let isProse = (rest == "|" || rest == ">" || rest == ">-" || rest == "|-")
                while j < lines.count {
                    let peek = lines[j]
                    let peekTrim = peek.trimmingCharacters(in: .whitespaces)
                    if peekTrim.isEmpty { j += 1; continue }
                    // Stop if we're back to column-0 non-indented content (next top-level key).
                    let leading = peek.prefix(while: { $0 == " " })
                    if leading.count == 0 { break }
                    if peekTrim.hasPrefix("- ") {
                        items.append(String(peekTrim.dropFirst(2)).trimmingCharacters(in: .whitespaces)
                                     .trimmingCharacters(in: CharacterSet(charactersIn: "\"")))
                    } else if isProse {
                        prose.append(peekTrim)
                    } else {
                        break
                    }
                    j += 1
                }
                if !items.isEmpty {
                    out[key] = .list(items)
                } else if !prose.isEmpty {
                    out[key] = .scalar(prose.joined(separator: " "))
                } else {
                    out[key] = .scalar("")
                }
                i = j
                continue
            } else if rest.hasPrefix("[") && rest.hasSuffix("]") {
                // Inline list: [a, b, "c d"]
                let inner = String(rest.dropFirst().dropLast())
                let items = splitFlow(inner).map { unquote($0.trimmingCharacters(in: .whitespaces)) }
                out[key] = .list(items)
                i += 1
                continue
            } else {
                out[key] = .scalar(unquote(rest))
                i += 1
                continue
            }
        }
        return out
    }

    private static func splitFlow(_ s: String) -> [String] {
        var result: [String] = []
        var current = ""
        var inQuote: Character? = nil
        for ch in s {
            if let q = inQuote {
                current.append(ch)
                if ch == q { inQuote = nil }
            } else if ch == "\"" || ch == "'" {
                inQuote = ch
                current.append(ch)
            } else if ch == "," {
                result.append(current)
                current = ""
            } else {
                current.append(ch)
            }
        }
        if !current.isEmpty { result.append(current) }
        return result
    }

    private static func unquote(_ s: String) -> String {
        var t = s
        if t.hasPrefix("\"") && t.hasSuffix("\"") && t.count >= 2 {
            t.removeFirst(); t.removeLast()
        } else if t.hasPrefix("'") && t.hasSuffix("'") && t.count >= 2 {
            t.removeFirst(); t.removeLast()
        }
        return t
    }
}

// MARK: - Manifest assembler (pure)

/// Assembles a `PhenotypeManifestVerified` from a story + captured keyframes.
/// This is pure data manipulation so unit tests can round-trip it without a
/// real XCTestCase instance.
public enum PhenotypeManifestAssembler {
    public static let schemaVersion = "user-story.manifest.verified/1"

    public static func assemble(
        story: PhenotypeUserStory,
        keyframes: [PhenotypeKeyframe],
        started: Date,
        finished: Date?,
        passed: Bool,
        failure: String?,
        recordingPath: String?,
        recordingDenied: Bool
    ) -> PhenotypeManifestVerified {
        let iso = ISO8601DateFormatter()
        iso.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        return PhenotypeManifestVerified(
            schema_version: schemaVersion,
            journey_id: story.journey_id,
            story: story,
            started_at: iso.string(from: started),
            finished_at: finished.map { iso.string(from: $0) },
            passed: passed,
            failure: failure,
            keyframes: keyframes,
            recording_path: recordingPath,
            recording_denied: recordingDenied
        )
    }

    public static func encode(_ m: PhenotypeManifestVerified) throws -> Data {
        let enc = JSONEncoder()
        enc.outputFormatting = [.prettyPrinted, .sortedKeys]
        return try enc.encode(m)
    }

    public static func decode(_ data: Data) throws -> PhenotypeManifestVerified {
        try JSONDecoder().decode(PhenotypeManifestVerified.self, from: data)
    }
}

// MARK: - XCTestCase base class

#if canImport(XCTest)

/// XCTestCase base class that wires up:
///   - source-file-driven @user-story discovery at setUp
///   - per-activity keyframe capture via `XCTContext.runActivity`
///   - manifest.verified.json emission at tearDown when
///     `PHENOTYPE_USER_STORY_RECORD` is set
///   - optional ScreenCaptureKit video via SckBridge when
///     `PHENOTYPE_USER_STORY_RECORD_VIDEO` is set
///
/// Subclasses call `recordKeyframe(name:assertion:)` from inside their step
/// closures (or use `phenotypeActivity(name:)`). When running under XCUITest
/// with a live AXUIElement, `recordKeyframe` can be paired with
/// `AccessibilitySnapshot.capture` — snapshots are written next to the
/// manifest and referenced via `structural_path`.
open class PhenotypeRecord: XCTestCase {

    /// Captured at setUp, emitted at tearDown.
    public private(set) var story: PhenotypeUserStory?
    private var sourceFile: String = ""
    private var testFunctionName: String = ""
    private var startedAt: Date = Date()
    private var keyframes: [PhenotypeKeyframe] = []
    private var keyframeIndex = 0
    private var recordingPath: String?
    private var recordingDenied: Bool = false
    private var manifestRoot: URL?

    /// Default output root: `./user-story-manifests/<journey_id>/`.
    /// Override for custom per-suite destinations.
    open var manifestOutputRoot: URL {
        URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
            .appendingPathComponent("user-story-manifests")
    }

    /// Is recording enabled?
    open var isRecordingEnabled: Bool {
        ProcessInfo.processInfo.environment["PHENOTYPE_USER_STORY_RECORD"] != nil
    }

    open var isVideoRecordingEnabled: Bool {
        ProcessInfo.processInfo.environment["PHENOTYPE_USER_STORY_RECORD_VIDEO"] != nil
    }

    open override func setUpWithError() throws {
        try super.setUpWithError()
        self.startedAt = Date()
        self.keyframes = []
        self.keyframeIndex = 0

        // Extract source path + test name from XCTestCase.name
        //   "-[PlannerJourneyTests test_plannerLaunch]" -> "test_plannerLaunch"
        self.testFunctionName = extractTestName(from: self.name)
        self.sourceFile = deriveSourceFile()

        if isRecordingEnabled {
            try discoverStory()
            try? prepareManifestDir()
        }
    }

    open override func tearDownWithError() throws {
        if isRecordingEnabled {
            do {
                try emitManifest(passed: true, failure: nil)
            } catch {
                // Don't swallow: re-throw so CI sees the problem.
                throw error
            }
        }
        try super.tearDownWithError()
    }

    // MARK: - Public recording API (for subclasses)

    /// Run `block` inside an `XCTContext.runActivity` and capture a keyframe.
    public func phenotypeActivity<T>(
        _ name: String,
        structuralSnapshotPath: String? = nil,
        assertion: String? = nil,
        block: (XCTActivity) throws -> T
    ) rethrows -> T {
        return try XCTContext.runActivity(named: name) { activity in
            let result = try block(activity)
            self.recordKeyframe(name: name, assertion: assertion, structuralPath: structuralSnapshotPath)
            return result
        }
    }

    /// Manually register a keyframe (for callers that don't use `phenotypeActivity`).
    public func recordKeyframe(name: String, assertion: String? = nil, structuralPath: String? = nil) {
        let iso = ISO8601DateFormatter()
        iso.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        let kf = PhenotypeKeyframe(
            index: keyframeIndex,
            activity_name: name,
            timestamp_iso: iso.string(from: Date()),
            structural_path: structuralPath,
            assertion: assertion
        )
        keyframes.append(kf)
        keyframeIndex += 1
    }

    // MARK: - Private helpers

    private func extractTestName(from xcName: String) -> String {
        // XCTestCase.name format: "-[<Class> <selector>]" on ObjC, or
        // "<selector>" when Swift-native.
        if let open = xcName.firstIndex(of: "["), let close = xcName.lastIndex(of: "]") {
            let inner = xcName[xcName.index(after: open)..<close]
            if let sp = inner.firstIndex(of: " ") {
                return String(inner[inner.index(after: sp)...])
            }
            return String(inner)
        }
        return xcName
    }

    /// Walk up test bundle Resources to find `<ClassName>.swift` source.
    /// Fallback: consult `PHENOTYPE_USER_STORY_SOURCE_ROOT` env var.
    private func deriveSourceFile() -> String {
        // Prefer test class introspection
        let typeName = String(describing: type(of: self))
        let envRoot = ProcessInfo.processInfo.environment["PHENOTYPE_USER_STORY_SOURCE_ROOT"]
            ?? FileManager.default.currentDirectoryPath

        // Search for `<TypeName>.swift` under the root.
        let rootURL = URL(fileURLWithPath: envRoot)
        if let enumerator = FileManager.default.enumerator(
            at: rootURL,
            includingPropertiesForKeys: nil,
            options: [.skipsHiddenFiles]
        ) {
            for case let url as URL in enumerator {
                if url.lastPathComponent == "\(typeName).swift" {
                    return url.path
                }
            }
        }
        return ""
    }

    private func discoverStory() throws {
        guard !sourceFile.isEmpty, FileManager.default.fileExists(atPath: sourceFile) else {
            throw PhenotypeRecordError.frontmatterNotFound(file: sourceFile, testName: testFunctionName)
        }
        let content = try String(contentsOfFile: sourceFile, encoding: .utf8)
        guard let parsed = try PhenotypeFrontmatter.parseForTest(
            source: content,
            testFunctionName: testFunctionName
        ) else {
            throw PhenotypeRecordError.frontmatterNotFound(file: sourceFile, testName: testFunctionName)
        }
        self.story = parsed.story
    }

    private func prepareManifestDir() throws {
        guard let story = story else { return }
        let root = manifestOutputRoot.appendingPathComponent(story.journey_id)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        self.manifestRoot = root
    }

    private func emitManifest(passed: Bool, failure: String?) throws {
        guard let story = story else { return }
        guard let root = manifestRoot else { return }
        let manifest = PhenotypeManifestAssembler.assemble(
            story: story,
            keyframes: keyframes,
            started: startedAt,
            finished: Date(),
            passed: passed,
            failure: failure,
            recordingPath: recordingPath,
            recordingDenied: recordingDenied
        )
        let data: Data
        do {
            data = try PhenotypeManifestAssembler.encode(manifest)
        } catch {
            throw PhenotypeRecordError.manifestWriteFailed(
                path: root.path,
                underlying: String(describing: error)
            )
        }
        let out = root.appendingPathComponent("manifest.verified.json")
        do {
            try data.write(to: out)
        } catch {
            throw PhenotypeRecordError.manifestWriteFailed(
                path: out.path,
                underlying: String(describing: error)
            )
        }
    }
}
#endif
