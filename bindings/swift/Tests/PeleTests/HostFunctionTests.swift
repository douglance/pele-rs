import Foundation
import XCTest
import Pele

private final class Marker: Error {}
@MainActor private final class State {
    var mode = 0, calls = 0, clocks = 0, measurements = 0
    var view: Pele.ActiveModel?
    var original: UInt32 = 19
    var first: UInt32?
    var caught = false
}
@MainActor final class HostFunctionTests: XCTestCase {
    let source = "gantt\ndateFormat YYYY-MM-DD\nsection S\nA:a,2024-01-01,1d\nB:b,after a,2d"
    let now = 1_704_067_200_000.0
    func reference(_ atom: Pele.PublicGraph.Atom) throws -> UInt32 {
        guard case .ref(let id) = atom else { throw PeleError(code: "test", message: "Reference required") }; return id
    }
    func install(_ function: Pele.HostFunction, on document: Pele.MutableDocument) throws {
        let value = try document.bindHostFunction(function)
        var graph = try document.snapshot().graph
        try graph.setField("getTasks", value: value)
        _ = try document.apply(graph, expectedRevision: document.revision)
    }
    private func resources(_ state: State) -> Pele.HostResources {
        var h = Pele.HostResources(); h.timeZone = TimeZone(secondsFromGMT: 0)!
        h.nowMillis = { MainActor.assumeIsolated { state.clocks += 1; return 1_704_067_200_000 } }
        h.textWidth = { request in MainActor.assumeIsolated { state.measurements += 1; return Double(request.text.utf16.count) * request.size * 0.6 } }
        return h
    }
    func testOriginalDelegationAliasesAlternateResultsAndLiveClosure() throws {
        let state = State(), document = try Pele.Engine().parseMutable(source)
        let function = Pele.HostFunction { view, receiver, arguments in
            state.view = view; state.calls += 1
            XCTAssertThrowsError(try document.snapshot())
            XCTAssertThrowsError(try document.close())
            XCTAssertEqual(arguments.count, 1)
            guard case .number(let now) = arguments[0] else { throw Marker() }; XCTAssertEqual(now, self.now)
            let tasks = try view.invokeBuiltin(method: state.original, receiver: receiver, arguments: arguments)
            let a = try view.read("0", on: tasks), b = try view.read("1", on: tasks)
            let aid = try self.reference(a)
            if let previous = state.first { XCTAssertEqual(aid, previous) } else { state.first = aid }
            let end = try view.read("endTime", on: a), start = try view.read("startTime", on: b)
            XCTAssertEqual(try self.reference(end), try self.reference(start))
            XCTAssertEqual(try view.dateMilliseconds(end), self.now + 86_400_000)
            try view.set("task", on: a, to: view.intern("Changed live"))
            if state.mode == 1 { let array = try view.array(); try view.append(a, to: array); return array }
            return tasks
        }
        try install(function, on: document)
        let full = try document.render(outputLimit: .undefined, resources: resources(state))
        XCTAssertTrue(full.svg.contains("Changed live")); XCTAssertEqual(state.clocks, 1)
        XCTAssertThrowsError(try state.view!.root())
        state.mode = 1
        let short = try document.render(outputLimit: .undefined, resources: resources(state))
        XCTAssertLessThan(short.height, full.height)
        function.call = { view, receiver, args in
            let tasks = try view.invokeBuiltin(method: 19, receiver: receiver, arguments: args)
            let a = try view.read("0", on: tasks)
            try view.set("task", on: a, to: view.intern("Live replacement"))
            return tasks
        }
        let changed = try document.renderPresentation(resources: resources(state))
        XCTAssertTrue(changed.svg.contains("Live replacement")); XCTAssertNotNil(changed.presentation)
        let snapshot = try document.snapshot()
        XCTAssertEqual(snapshot.revision, 4)
        try document.close(); XCTAssertThrowsError(try document.render())
    }
    func testOriginalErrorPartialMutationsAndLateLimitRecovery() throws {
        let state = State(), error = Marker(), document = try Pele.Engine().parseMutable(source)
        let function = Pele.HostFunction { view, receiver, arguments in
            state.view = view
            let tasks = try view.invokeBuiltin(method: 19, receiver: receiver, arguments: arguments)
            let task = try view.read("0", on: tasks)
            try view.set("task", on: task, to: view.intern("Mutation survives"))
            if state.mode == 0 { throw error }
            return tasks
        }
        try install(function, on: document)
        XCTAssertThrowsError(try document.renderOperation(outputLimit: .undefined, resources: resources(state))) { XCTAssertTrue(($0 as AnyObject) === error) }
        XCTAssertEqual(document.revision, 2); XCTAssertEqual(state.measurements, 0)
        let graph = try document.snapshot().graph
        let tasks = try reference(try XCTUnwrap(graph.field("tasks")))
        let task = try reference(try XCTUnwrap(graph.field("0", on: tasks)))
        guard case .string(let id) = try graph.field("task", on: task) else { return XCTFail() }
        XCTAssertEqual(graph.strings[Int(id)].stringIfWellFormed, "Mutation survives")
        state.mode = 1
        XCTAssertThrowsError(try document.render(outputLimit: .number(0), resources: resources(state)))
        XCTAssertEqual(document.revision, 3); XCTAssertGreaterThan(state.measurements, 0)
        XCTAssertTrue(try document.render(outputLimit: .undefined, resources: resources(state)).svg.contains("Mutation survives"))
        XCTAssertThrowsError(try state.view!.array())
    }
    func testCaughtCalendarErrorReentryAndLosslessArenaValues() throws {
        let state = State(), error = Marker(), document = try Pele.Engine().parseMutable(source)
        let function = Pele.HostFunction { view, receiver, arguments in
            state.view = view
            do { _ = try view.invokeBuiltin(method: 19, receiver: receiver, arguments: arguments); XCTFail() }
            catch let caught { XCTAssertTrue((caught as AnyObject) === error); state.caught = true }
            let tasks = try view.invokeBuiltin(method: 19, receiver: receiver, arguments: arguments)
            let raw = Pele.JSString(codeUnits: [0xd800, 65])
            let value = try view.intern(raw); XCTAssertEqual(try view.string(value), raw)
            try view.set("rawAlias", on: receiver, to: value)
            let date = try view.date(milliseconds: -0.5)
            XCTAssertEqual(try view.dateMilliseconds(date).bitPattern, Double(0).bitPattern)
            try view.set("createdDate", on: receiver, to: date)
            return tasks
        }
        try install(function, on: document)
        var h = resources(state)
        h.calendarInstant = { _ in try MainActor.assumeIsolated {
            XCTAssertThrowsError(try state.view!.root())
            if !state.caught { throw error }; return 1_704_067_200_000
        }}
        _ = try document.render(outputLimit: .undefined, resources: h)
        XCTAssertTrue(state.caught)
        let graph = try document.snapshot().graph
        guard case .string(let id) = try graph.field("rawAlias") else { return XCTFail() }
        XCTAssertEqual(graph.strings[Int(id)].codeUnits, [0xd800, 65])
    }
    func testOwnerBindingsAndCloseReleaseWithoutEscapedLeaseRetention() throws {
        let first = try Pele.Engine().parseMutable(source), second = try Pele.Engine().parseMutable(source)
        var function: Pele.HostFunction? = Pele.HostFunction { view, receiver, arguments in try view.invokeBuiltin(method: 19, receiver: receiver, arguments: arguments) }
        weak var weakFunction = function
        let atom = try first.bindHostFunction(function!)
        function = nil; XCTAssertNotNil(weakFunction)
        var graph = try second.snapshot().graph; try graph.setField("getTasks", value: atom)
        _ = try second.apply(graph, expectedRevision: 0)
        var options = Pele.Engine.Options(); options.now = now
        XCTAssertThrowsError(try second.render(outputLimit: .undefined, options: options))
        try first.close(); XCTAssertNil(weakFunction)
        try second.close()
    }
}
