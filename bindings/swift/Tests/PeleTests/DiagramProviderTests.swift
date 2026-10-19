import Foundation
import XCTest
import Pele

private final class CustomModel {
    var width: Double = 40
    var renders = 0
    var onDeinit: () -> Void = {}
    deinit { onDeinit() }
}
private final class WrongModel {}
private final class ReleaseProbe {
    let body: () -> Void
    init(_ body: @escaping () -> Void) { self.body = body }
    deinit { body() }
}
private final class CustomError: Error {}

// External consumer feature tests: only public Pele API, original class models,
// actual Rust/C dispatch and caller-authored SVG; no @testable or native internals.
final class DiagramProviderTests: XCTestCase {
    private let source = "---\ntitle: Custom\nconfig:\n  pie:\n    useMaxWidth: false\n---\npie\n%% retained comment\n\"A\":1"
    private func drawing(_ model: CustomModel, generation: String = "A") -> Pele.ProviderRendered {
        Pele.ProviderRendered(svg: "<svg xmlns=\"http://www.w3.org/2000/svg\"><rect data-provider=\"\(generation)\" width=\"\(model.width)\" height=\"20\"/></svg>", width: model.width, height: 20,
            links: [DiagramLink(id: "safe", href: "https://example.com")])
    }
    func testPublicProviderIdentityReplacementLivePropertiesAndLifetime() throws {
        let registry = try Pele.Registry.empty()
        var seen: Pele.ProviderParseContext?, renderContext: Pele.ProviderRenderContext?
        var a: Pele.Diagram<CustomModel>? = Pele.Diagram(type: "pie", section: "pie", keepComments: true,
            parse: { seen = $0; return CustomModel() }, render: { model, context in
                renderContext = context; model.renders += 1; return self.drawing(model)
            })
        weak var weakA = a
        try registry.register(a!)
        XCTAssertTrue(try registry.registered("pie")); XCTAssertTrue(try registry.supports(source))
        let first = try registry.parseProvider(source, as: CustomModel.self)
        let model = try first.model
        XCTAssertTrue(model === (try first.model)); XCTAssertEqual(seen?.title, "Custom")
        XCTAssertTrue(seen?.source.contains("%% retained comment") == true); XCTAssertGreaterThan(seen!.lineOffset, 0)
        if case .number(let limit) = seen!.maxEdges { XCTAssertEqual(limit, 5000) } else { XCTFail("Effective default") }
        model.width = 91
        XCTAssertEqual(try first.render().width, 91); XCTAssertEqual(renderContext?.options.responsive, false)
        XCTAssertThrowsError(try renderContext!.textWidth("expired", size: 13))
        let b = Pele.Diagram(type: "pie", parse: { _ in CustomModel() }, render: { model, _ in self.drawing(model, generation: "B") })
        try registry.register(b)
        let second = try registry.parseProvider(source, as: CustomModel.self)
        a!.type = "flowchart"; a!.section = "other"
        a!.render = { model, context in renderContext = context; model.width += 1; return self.drawing(model, generation: "A-live") }
        XCTAssertEqual(first.type, "pie"); XCTAssertTrue(try first.render().svg.contains("A-live"))
        XCTAssertEqual(renderContext?.options.responsive, true); XCTAssertTrue(try second.render().svg.contains("B"))
        a = nil; try registry.close(); XCTAssertNotNil(weakA)
        _ = try first.render(); try first.close(); XCTAssertNil(weakA)
        XCTAssertEqual(model.width, 93); XCTAssertThrowsError(try first.model)
        _ = try second.render(); try second.close()
    }
    func testPublicErrorsMutationRecoveryResourcesAndReentry() throws {
        let registry = try Pele.Registry.empty(), marker = CustomError()
        var document: Pele.ProviderDocument<CustomModel>?
        let provider = Pele.Diagram(type: "pie", parse: { _ in CustomModel() }, render: { model, _ in
            model.renders += 1; throw marker
        })
        try registry.register(provider); document = try registry.parseProvider(source, as: CustomModel.self)
        let model = try document!.model
        do { _ = try document!.render(outputLimit: .number(0)); XCTFail("Original error must precede limit") }
        catch { XCTAssertTrue(error as AnyObject === marker) }
        XCTAssertEqual(model.renders, 1); XCTAssertEqual(document!.revision, 1)
        provider.render = { model, _ in
            XCTAssertThrowsError(try document!.close()); XCTAssertThrowsError(try document!.model)
            XCTAssertThrowsError(try document!.render()); model.renders += 1; return self.drawing(model)
        }
        XCTAssertThrowsError(try document!.render(outputLimit: .number(0)))
        XCTAssertEqual(model.renders, 2); XCTAssertEqual(document!.revision, 2)
        var resources = Pele.HostResources(); resources.textWidth = { _ in throw marker }
        provider.render = { model, context in _ = try context.textWidth("host", size: 13); return self.drawing(model) }
        do { _ = try document!.render(resources: resources); XCTFail("Original resource error") }
        catch { XCTAssertTrue(error as AnyObject === marker) }
        provider.render = { model, context in
            do { _ = try context.textWidth("host", size: 13); XCTFail("Expected resource failure") }
            catch { XCTAssertTrue(error as AnyObject === marker) }
            return self.drawing(model, generation: "recovered")
        }
        XCTAssertTrue(try document!.render(resources: resources).svg.contains("recovered"))
        provider.render = { model, context in model.width = try context.textWidth("host", size: 13); return self.drawing(model) }
        resources.textWidth = { request in request.size + Double(request.text.count) }
        XCTAssertEqual(try document!.render(resources: resources).width, 17)
        provider.parse = { _ in
            XCTAssertThrowsError(try registry.close()); XCTAssertThrowsError(try registry.registerBuiltin("pie")); throw marker
        }
        do { _ = try registry.parseProvider(source, as: CustomModel.self); XCTFail("Original parse error") }
        catch { XCTAssertTrue(error as AnyObject === marker) }
        provider.render = { model, _ in self.drawing(model) }; provider.parse = { _ in CustomModel() }
        document = nil; try registry.close()
    }
    func testPublicExactNumbersPassiveOptionsSelectionAndModelMismatch() throws {
        let registry = try Pele.Registry.empty()
        var parseCount = 0, modelReleases = 0, context: Pele.ProviderRenderContext?
        let provider = Pele.Diagram(type: "pie", parse: { request in
            parseCount += 1
            if case .number(let edges) = request.maxEdges { XCTAssertTrue(edges.isNaN) } else { XCTFail("Raw exact edges") }
            let model = CustomModel(); model.onDeinit = { modelReleases += 1 }; return model
        }, render: { model, request in context = request; return self.drawing(model) })
        try registry.register(provider)
        var parse = Pele.Engine.ParseOptions(); parse.limit = -1; parse.maxEdges = -1
        XCTAssertThrowsError(try registry.parseProvider(source, as: CustomModel.self, options: parse)); XCTAssertEqual(parseCount, 0)
        let limits = Pele.ParseLimits(source: .undefined, maxEdges: .number(.nan))
        XCTAssertThrowsError(try registry.parseProvider(source, as: WrongModel.self, limits: limits, options: parse))
        XCTAssertEqual(modelReleases, 1)
        let document = try registry.parseProvider(source, as: CustomModel.self, limits: limits, options: parse)
        var options = Pele.Engine.Options(); options.fontSize = .nan; options.maxWidth = -.infinity
        options.directionBreakpoint = -0.0; options.padding = .infinity; options.now = 1.25
        options.limit = -1; options.maxEdges = -1; options.config = ["passive": .bool(true)]; options.outputLimit = -1
        XCTAssertThrowsError(try document.render(options: options)) // Late exact negative limit.
        XCTAssertEqual(context!.options.limit, -1); XCTAssertEqual(context!.options.maxEdges, -1)
        XCTAssertEqual(context!.options.config["passive"], .bool(true))
        XCTAssertTrue(context!.options.fontSize!.isNaN); XCTAssertEqual(context!.options.maxWidth, -.infinity)
        XCTAssertEqual(context!.options.directionBreakpoint!.bitPattern, (-0.0).bitPattern)
        XCTAssertEqual(context!.options.padding, .infinity); XCTAssertEqual(context!.options.now, 1.25)
        _ = try document.render(outputLimit: .number(.infinity), options: options)
        if case .number(let threshold) = context!.outputLimit { XCTAssertEqual(threshold, .infinity) } else { XCTFail("Exact policy") }
        XCTAssertNil(context!.options.outputLimit)
        try registry.registerBuiltin("pie")
        XCTAssertThrowsError(try registry.parseProvider(source, as: CustomModel.self))
        XCTAssertTrue(try registry.parseMutable("pie\n\"A\":1").render().svg.contains("<svg"))
        _ = try document.render(); try document.close(); XCTAssertEqual(modelReleases, 2)
        try registry.close()
    }
    func testPublicReleaseCallbacksCannotReenterDestruction() throws {
        let registry = try Pele.Registry.empty()
        var registryReleases = 0, modelReleases = 0
        func makeProvider() -> Pele.Diagram<CustomModel> {
            let probe = ReleaseProbe { [weak registry] in
                registryReleases += 1
                do { try registry!.close(); XCTFail("Registry destruction reentered") } catch {}
                do { _ = try registry!.capabilities(); XCTFail("Registry accessed during destruction") } catch {}
            }
            return Pele.Diagram(type: "pie", parse: { _ in
                withExtendedLifetime(probe) { CustomModel() }
            }, render: { model, _ in self.drawing(model) })
        }
        var provider: Pele.Diagram<CustomModel>? = makeProvider()
        try registry.register(provider!)
        let document = try registry.parseProvider(source, as: CustomModel.self)
        weak var weakDocument = document
        try document.model.onDeinit = {
            modelReleases += 1
            do { _ = try weakDocument!.model; XCTFail("Document accessed during destruction") } catch {}
            do { try weakDocument!.close(); XCTFail("Document destruction reentered") } catch {}
        }
        try document.close(); XCTAssertEqual(modelReleases, 1)
        provider = nil; try registry.close(); XCTAssertEqual(registryReleases, 1)
        try document.close(); try registry.close()
    }

    func testNestedIdleDocumentAndLiveCommentPolicyKeepIndependentCalls() throws {
        let registry = try Pele.Registry.empty(), marker = CustomError()
        var next = 0, comments: [Bool] = []
        let provider = Pele.Diagram(type: "pie", keepComments: true, parse: { request in
            comments.append(request.source.contains("%% retained comment")); next += 1
            let model = CustomModel(); model.width = Double(next); return model
        }, render: { model, _ in self.drawing(model) })
        try registry.register(provider)
        let first = try registry.parseProvider(source, as: CustomModel.self)
        provider.keepComments = false
        let second = try registry.parseProvider(source, as: CustomModel.self)
        XCTAssertEqual(comments, [true, false])
        provider.render = { model, context in
            model.renders += 1
            if model.width == 2 { throw marker }
            do { _ = try second.render(); XCTFail("Nested original failure") }
            catch { XCTAssertTrue(error as AnyObject === marker) }
            // The outer callback/resources remain live after a complete nested call.
            XCTAssertGreaterThan(try context.textWidth("outer", size: 13), 0)
            return self.drawing(model)
        }
        _ = try first.render()
        XCTAssertEqual(first.revision, 1); XCTAssertEqual(second.revision, 1)
        XCTAssertEqual(try first.model.renders, 1); XCTAssertEqual(try second.model.renders, 1)
        provider.render = { model, _ in self.drawing(model) }
        try first.close(); try second.close(); try registry.close()
    }

}
