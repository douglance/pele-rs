import Foundation
import XCTest
import Pele

private final class AsyncWidthFailure: Error {}

private actor AsyncStartGate {
    private var opened = false
    private var waiters: [CheckedContinuation<Void, Never>] = []
    func wait() async {
        if opened { return }
        await withCheckedContinuation { waiters.append($0) }
    }
    func open() {
        opened = true
        let pending = waiters; waiters.removeAll()
        for waiter in pending { waiter.resume() }
    }
}

@MainActor
final class AsyncEngineTests: XCTestCase {
    func testRegistryLoadSelectsAllExactBuiltinIDsAndRetainsDocuments() throws {
        let registry = try Pele.Registry.empty()
        XCTAssertFalse(try registry.load(nil))
        for unknown in ["", "graph", "Pie", "pie ", "unknown-native-type"] {
            XCTAssertFalse(try registry.load(unknown), unknown)
        }
        XCTAssertTrue(try registry.capabilities().allSatisfy { !$0.registered })
        let kinds = try registry.capabilities().map(\.type)
        XCTAssertEqual(kinds.count, 33)
        for kind in kinds {
            XCTAssertTrue(try registry.load(kind), kind)
            XCTAssertTrue(try registry.load(kind), kind)
            XCTAssertTrue(try registry.registered(kind), kind)
        }
        let document = try registry.parseMutable("pie\n\"Retained\":1")
        defer { try? document.close() }
        let expected = try document.render(outputLimit: .undefined)
        XCTAssertTrue(try registry.load("pie"))
        try registry.close(); try registry.close()
        XCTAssertEqual(try document.render(outputLimit: .undefined).svg, expected.svg)
        XCTAssertThrowsError(try registry.load(nil)) {
            XCTAssertEqual(($0 as? PeleError)?.message, "Registry is closed")
        }
    }

    func testConcurrentLoadsAndRendersUseOneIsolatedOwner() async throws {
        let engine = try Pele.AsyncEngine(), separate = try Pele.AsyncEngine()
        let empty = try await engine.capabilities()
        XCTAssertEqual(empty.filter(\.registered).count, 0)
        let loaded = try await withThrowingTaskGroup(of: Bool.self) { group in
            for _ in 0..<12 { group.addTask { try await engine.load("pie") } }
            var values: [Bool] = []
            for try await value in group { values.append(value) }
            return values
        }
        XCTAssertEqual(loaded.count, 12); XCTAssertTrue(loaded.allSatisfy { $0 })
        let selected = try await engine.capabilities()
        XCTAssertEqual(selected.filter(\.registered).map(\.type), ["pie"])
        let independentlyRegistered = try await separate.registered("pie")
        XCTAssertFalse(independentlyRegistered)
        let source = "pie title Async\n\"A\":2\n\"B\":3"
        let expected = try Pele.Engine().render(source, outputLimit: .undefined)
        let rendered = try await withThrowingTaskGroup(of: FullRenderedDiagram.self) { group in
            for _ in 0..<8 {
                group.addTask { try await engine.renderAsync(source, outputLimit: .undefined) }
            }
            var values: [FullRenderedDiagram] = []
            for try await value in group { values.append(value) }
            return values
        }
        XCTAssertEqual(rendered.count, 8)
        for result in rendered {
            XCTAssertEqual(result.type, expected.type)
            XCTAssertEqual(result.svg, expected.svg)
            XCTAssertEqual(result.width.bitPattern, expected.width.bitPattern)
            XCTAssertEqual(result.height.bitPattern, expected.height.bitPattern)
        }
        try await engine.close(); try await engine.close(); try await separate.close()
        XCTAssertEqual(rendered[0].svg, expected.svg, "Returned values outlive the owner")
        do {
            _ = try await engine.renderAsync(source)
            XCTFail("Closed async owner rendered")
        } catch { XCTAssertEqual((error as? PeleError)?.message, "Registry is closed") }
    }

    func testLoadingPrecedesParseErrorsAndFailuresRecover() async throws {
        let engine = try Pele.AsyncEngine()
        let source = "info\n%%" + String(repeating: "x", count: 60_000)
        do {
            _ = try await engine.renderAsync(source)
            XCTFail("Semantic source limit did not reject")
        } catch {
            XCTAssertEqual((error as? CompatibilityError)?.message, "Diagram source is longer than the limit of 50000 characters.")
        }
        let selectedInfo = try await engine.registered("info")
        XCTAssertTrue(selectedInfo, "Detection/load happen before the parse limit")
        let recovered = try await engine.renderAsync(source, limits: .init(source: .number(.infinity)), outputLimit: .undefined)
        XCTAssertEqual(recovered.type, "info")

        let badYAML = "---\ntitle: \"\\q\"\n---\npie\n\"A\":1"
        var expected: CompatibilityError?
        do { _ = try Pele.Engine().render(badYAML) } catch { expected = error as? CompatibilityError }
        let expectedError = try XCTUnwrap(expected)
        do {
            _ = try await engine.renderAsync(badYAML)
            XCTFail("Malformed front matter rendered")
        } catch {
            let actual = try XCTUnwrap(error as? CompatibilityError)
            XCTAssertEqual(actual.name, expectedError.name); XCTAssertEqual(actual.code, expectedError.code)
            XCTAssertEqual(actual.message, expectedError.message); XCTAssertEqual(actual.type, expectedError.type)
            XCTAssertEqual(actual.line, expectedError.line); XCTAssertEqual(actual.column, expectedError.column)
            XCTAssertEqual(actual.snippet, expectedError.snippet)
        }
        let pieSelected = try await engine.registered("pie")
        XCTAssertFalse(pieSelected, "Failed detection must not select a family")
        do {
            _ = try await engine.renderAsync("flowchart TD\nA --> [")
            XCTFail("Invalid grammar rendered")
        } catch { XCTAssertEqual((error as? CompatibilityError)?.code, "syntax") }
        let flowSelected = try await engine.registered("flowchart")
        XCTAssertTrue(flowSelected, "Grammar failure keeps successful selection")
        let valid = try await engine.renderAsync("flowchart TD\nA --> B")
        XCTAssertEqual(valid.type, "flowchart")
        try await engine.close()
    }

    func testExactPoliciesNumericOptionsAndOriginalHostErrorCompose() async throws {
        let engine = try Pele.AsyncEngine()
        let source = "flowchart TD\nA --> B --> C"
        var options = Pele.Engine.Options()
        options.limit = -1; options.maxEdges = -1; options.outputLimit = -1
        options.fontSize = 13.5; options.padding = -0.0; options.now = .nan
        options.config = ["flowchart": .object(["useMaxWidth": .bool(false)])]
        let exact = Pele.ParseLimits(source: .undefined, maxEdges: .number(1.5))
        let rendered = try await engine.renderAsync(source, limits: exact, outputLimit: .number(.infinity), options: options)
        let document = try Pele.Engine().parseMutable(source, limits: exact, options: {
            var parse = Pele.Engine.ParseOptions(); parse.limit = -1; parse.maxEdges = -1; parse.config = options.config; return parse
        }())
        defer { try? document.close() }
        var retainedOptions = options
        retainedOptions.limit = nil; retainedOptions.maxEdges = nil; retainedOptions.config = [:]
        XCTAssertEqual(rendered.svg, try document.render(outputLimit: .number(.infinity), options: retainedOptions).svg)
        do {
            _ = try await engine.renderAsync(source, limits: exact, outputLimit: .number(0), options: options)
            XCTFail("Exact output limit did not reject")
        } catch {
            XCTAssertEqual((error as? CompatibilityError)?.message, "Diagram output is longer than the limit of 0 characters.")
        }
        let sentinel = AsyncWidthFailure()
        var resources = Pele.HostResources(); resources.textWidth = { _ in throw sentinel }
        do {
            _ = try await engine.renderAsync(source, limits: exact, outputLimit: .undefined, options: options, resources: resources)
            XCTFail("Throwing host callback succeeded")
        } catch { XCTAssertTrue((error as? AsyncWidthFailure) === sentinel) }
        let recovered = try await engine.renderAsync("pie\n\"A\":1", outputLimit: .undefined)
        XCTAssertEqual(recovered.type, "pie")
        try await engine.close()
    }

    func testDefaultPoliciesUsePeleComparisonsAndClassCallbacksBeforeOutputRejection() async throws {
        let engine = try Pele.AsyncEngine()
        let source = "classDiagram\nclass Account"
        let sentinel = AsyncWidthFailure()
        var resources = Pele.HostResources(); resources.textWidth = { _ in throw sentinel }
        for threshold in [0, -1] {
            var options = Pele.Engine.Options(); options.outputLimit = threshold
            do {
                _ = try await engine.renderAsync(source, options: options, resources: resources)
                XCTFail("Class output policy ran before its host callback")
            } catch { XCTAssertTrue((error as? AsyncWidthFailure) === sentinel) }
            do {
                _ = try await engine.renderAsync(source, options: options)
                XCTFail("Default facade output policy did not reject")
            } catch {
                XCTAssertEqual((error as? CompatibilityError)?.message, "Diagram output is longer than the limit of \(threshold) characters.")
            }
            let recovered = try await engine.renderAsync(source, outputLimit: .undefined, options: options)
            XCTAssertEqual(recovered.type, "class")
        }
        try await engine.close()
    }

    func testIntegerParseFallbacksAndExplicitOverridesResolveIndependently() async throws {
        let engine = try Pele.AsyncEngine(), source = "flowchart TD\nA --> B"
        var options = Pele.Engine.Options()
        options.limit = -1; options.maxEdges = -1; options.outputLimit = -1
        let sentinel = AsyncWidthFailure()
        var resources = Pele.HostResources(); resources.textWidth = { _ in throw sentinel }
        do {
            _ = try await engine.renderAsync(source, options: options, resources: resources)
            XCTFail("Negative source threshold did not reject")
        } catch {
            XCTAssertEqual((error as? CompatibilityError)?.message, "Diagram source is longer than the limit of -1 characters.")
        }
        let selected = try await engine.registered("flowchart")
        XCTAssertTrue(selected, "Selection survives a source threshold failure")
        do {
            _ = try await engine.renderAsync(source, limits: .init(source: .undefined), options: options, resources: resources)
            XCTFail("Overriding source also overrode the edge threshold")
        } catch {
            XCTAssertEqual((error as? CompatibilityError)?.message, "Edge limit exceeded. 0 edges found, but the limit is -1.")
        }
        do {
            _ = try await engine.renderAsync(source, limits: .init(maxEdges: .undefined), options: options, resources: resources)
            XCTFail("Overriding edges also overrode the source threshold")
        } catch {
            XCTAssertEqual((error as? CompatibilityError)?.message, "Diagram source is longer than the limit of -1 characters.")
        }
        let limits = Pele.ParseLimits(source: .undefined, maxEdges: .undefined)
        do {
            _ = try await engine.renderAsync(source, limits: limits, options: options)
            XCTFail("Overriding parse thresholds also overrode the output threshold")
        } catch {
            XCTAssertEqual((error as? CompatibilityError)?.message, "Diagram output is longer than the limit of -1 characters.")
        }
        let recovered = try await engine.renderAsync(source, limits: limits, outputLimit: .undefined, options: options)
        XCTAssertEqual(recovered.type, "flowchart")
        try await engine.close()
    }

    func testPreDispatchCancellationDoesNotSelectAndCleanupStillCloses() async throws {
        let engine = try Pele.AsyncEngine(), gate = AsyncStartGate()
        let pending = Task {
            await gate.wait()
            return try await engine.renderAsync("pie\n\"A\":1")
        }
        pending.cancel(); await gate.open()
        do { _ = try await pending.value; XCTFail("Cancelled request dispatched") }
        catch { XCTAssertTrue(error is CancellationError) }
        let stillEmpty = try await engine.capabilities()
        XCTAssertTrue(stillEmpty.allSatisfy { !$0.registered })
        let cleanupGate = AsyncStartGate()
        let cleanup = Task { await cleanupGate.wait(); try await engine.close() }
        cleanup.cancel(); await cleanupGate.open(); try await cleanup.value
        do { _ = try await engine.capabilities(); XCTFail("Cancelled cleanup failed to close") }
        catch { XCTAssertEqual((error as? PeleError)?.message, "Registry is closed") }
    }
}
