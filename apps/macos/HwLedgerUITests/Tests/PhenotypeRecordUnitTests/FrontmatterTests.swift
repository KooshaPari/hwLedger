import XCTest
@testable import PhenotypeRecord

/// Unit tests for the pure `PhenotypeFrontmatter` parser — no XCUITest harness
/// required. Validates that the Swift `// MARK: @user-story` flavor is parsed
/// into a `PhenotypeUserStory` matching the canonical JSON schema.
final class FrontmatterTests: XCTestCase {

    func test_parsesSimpleBlockImmediatelyAboveFunction() throws {
        let src = """
        import Foundation

        // MARK: @user-story
        // journey_id: gui-demo
        // title: "Demo flow"
        // persona: "solo dev"
        // given: "app is launched"
        // when:
        //   - "click foo"
        //   - "click bar"
        // then:
        //   - "baz appears"
        // traces_to:
        //   - "FR-DEMO-1"
        // family: gui
        // MARK: @end
        func test_demo() throws {
        }
        """
        let result = try PhenotypeFrontmatter.parseForTest(source: src, testFunctionName: "test_demo")
        XCTAssertNotNil(result)
        let story = result!.story
        XCTAssertEqual(story.journey_id, "gui-demo")
        XCTAssertEqual(story.title, "Demo flow")
        XCTAssertEqual(story.persona, "solo dev")
        XCTAssertEqual(story.given, "app is launched")
        XCTAssertEqual(story.when, ["click foo", "click bar"])
        XCTAssertEqual(story.then, ["baz appears"])
        XCTAssertEqual(story.traces_to, ["FR-DEMO-1"])
        XCTAssertEqual(story.family, "gui")
    }

    func test_toleratesBlankAndAttributeBetweenBlockAndFunction() throws {
        let src = """
        // MARK: @user-story
        // journey_id: gui-demo2
        // title: "Demo 2"
        // persona: "tester"
        // given: "state"
        // when:
        //   - "do"
        // then:
        //   - "see"
        // traces_to:
        //   - "FR-X"
        // MARK: @end

        @MainActor
        func test_withAttribute() throws {}
        """
        let result = try PhenotypeFrontmatter.parseForTest(source: src, testFunctionName: "test_withAttribute")
        XCTAssertNotNil(result)
        XCTAssertEqual(result?.story.journey_id, "gui-demo2")
    }

    func test_parsesFoldedProseForGiven() throws {
        let src = """
        // MARK: @user-story
        // journey_id: gui-prose
        // title: "Prose"
        // persona: "user"
        // given: |
        //   The app is launched
        //   and configured
        // when:
        //   - "do thing"
        // then:
        //   - "see result"
        // traces_to:
        //   - "FR-P"
        // MARK: @end
        func test_prose() {}
        """
        let result = try PhenotypeFrontmatter.parseForTest(source: src, testFunctionName: "test_prose")
        XCTAssertNotNil(result)
        XCTAssertTrue(result!.story.given.contains("app is launched"))
        XCTAssertTrue(result!.story.given.contains("and configured"))
    }

    func test_returnsNilWhenNoBlock() throws {
        let src = """
        func test_lonely() {}
        """
        let result = try PhenotypeFrontmatter.parseForTest(source: src, testFunctionName: "test_lonely")
        XCTAssertNil(result)
    }

    func test_returnsNilWhenFunctionMissing() throws {
        let src = """
        // MARK: @user-story
        // journey_id: gui-orphan
        // MARK: @end
        func test_other() {}
        """
        let result = try PhenotypeFrontmatter.parseForTest(source: src, testFunctionName: "test_notHere")
        XCTAssertNil(result)
    }

    func test_parsesInlineList() throws {
        let src = """
        // MARK: @user-story
        // journey_id: gui-inline
        // title: "Inline"
        // persona: "user"
        // given: "x"
        // when: ["a", "b"]
        // then: ["ok"]
        // traces_to: ["FR-INLINE"]
        // MARK: @end
        func test_inline() {}
        """
        let result = try PhenotypeFrontmatter.parseForTest(source: src, testFunctionName: "test_inline")
        XCTAssertEqual(result?.story.when, ["a", "b"])
        XCTAssertEqual(result?.story.traces_to, ["FR-INLINE"])
    }
}
