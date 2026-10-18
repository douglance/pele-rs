// Synchronous AppKit feature-test host. Diagram work stays in the linked SDK.
#import <AppKit/AppKit.h>
#import <XCTest/XCTest.h>
#import <sys/resource.h>
#import <unistd.h>

static NSString *const QAIdentifier = @"com.unlocalhosted.pele.native-ui-qa";

@interface PeleUIObserver : NSObject <XCTestObservation>
@end
@implementation PeleUIObserver
- (void)testCaseDidFinish:(XCTestCase *)testCase {
    fprintf(stdout, "CASE %s failures=%lu\n", testCase.name.UTF8String,
            (unsigned long)testCase.testRun.failureCount);
}
- (void)testCase:(XCTestCase *)testCase didFailWithDescription:(NSString *)description
          inFile:(NSString *)filePath atLine:(NSUInteger)line {
    fprintf(stdout, "FAIL %s:%lu %s\n", filePath.UTF8String,
            (unsigned long)line, description.UTF8String);
}
@end

@interface PeleUIDelegate : NSObject <NSApplicationDelegate>
@property(nonatomic, strong) PeleUIObserver *observer;
@property(nonatomic, strong) NSWindow *readyWindow;
@property(nonatomic) BOOL started;
@end
@implementation PeleUIDelegate
- (void)applicationDidFinishLaunching:(NSNotification *)notification {
    self.readyWindow = [[NSWindow alloc] initWithContentRect:NSMakeRect(100, 100, 420, 160)
        styleMask:NSWindowStyleMaskTitled backing:NSBackingStoreBuffered defer:NO];
    self.readyWindow.title = @"Pele Native UI QA";
    NSTextField *label = [NSTextField labelWithString:@"Activate this QA window to run native Mount and Zoom feature tests."];
    label.frame = NSMakeRect(20, 60, 380, 50);
    label.lineBreakMode = NSLineBreakByWordWrapping;
    [self.readyWindow.contentView addSubview:label];
    [self.readyWindow makeKeyAndOrderFront:nil];
    [NSApp activateIgnoringOtherApps:YES];
    fprintf(stdout, "READY pid=%d bundle=%s active=%d\n", getpid(),
            NSRunningApplication.currentApplication.bundleIdentifier.UTF8String, NSApp.isActive);
    if (NSApp.isActive) [self beginTests];
}
- (void)applicationDidBecomeActive:(NSNotification *)notification {
    if (self.readyWindow) [self beginTests];
}
- (void)beginTests {
    if (self.started) return;
    self.started = YES;
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, (int64_t)(0.2 * NSEC_PER_SEC)),
                   dispatch_get_main_queue(), ^{
        if (!NSApp.isActive) { self.started = NO; return; }
        // Keep one real host window visible between test windows. Hiding the
        // last window can asynchronously deactivate the application mid-test.
        [self executeTests];
    });
}
- (void)executeTests {
    NSString *directory = NSProcessInfo.processInfo.arguments[1];
    NSString *bundlePath = [directory stringByAppendingPathComponent:@"PeleTests.xctest"];
    NSError *error = nil;
    if (![[NSBundle bundleWithPath:bundlePath] loadAndReturnError:&error]) {
        fprintf(stdout, "Cannot load copied test bundle: %s\n", error.description.UTF8String);
        exit(2);
    }
    self.observer = [PeleUIObserver new];
    [[XCTestObservationCenter sharedTestObservationCenter] addTestObserver:self.observer];
    XCTestSuite *suite = [[XCTestSuite alloc] initWithName:@"Pele native GUI integration"];
    NSMutableDictionary *counts = [NSMutableDictionary dictionary];
    // Async tests use SwiftPM's XCTest driver, never this synchronous GUI host.
    for (NSString *name in @[@"MountTests", @"ZoomTests"]) {
        Class testClass = NSClassFromString([@"PeleTests." stringByAppendingString:name]);
        if (!testClass) { fprintf(stdout, "Missing test class: %s\n", name.UTF8String); exit(3); }
        XCTestSuite *child = [XCTestSuite testSuiteForTestCaseClass:testClass];
        if (child.testCaseCount == 0 || child.testCaseCount > 100) {
            fprintf(stdout, "Unexpected test count: %s\n", name.UTF8String); exit(3);
        }
        counts[name] = @(child.testCaseCount);
        [suite addTest:child];
    }
    NSUInteger expected = suite.testCaseCount;
    BOOL active = NSApp.isActive;
    fprintf(stdout, "RUN pid=%d active=%d expected=%lu\n", getpid(), active, (unsigned long)expected);
    [suite runTest];
    BOOL succeeded = active && suite.testRun.hasSucceeded && suite.testRun.executionCount == expected;
    NSDictionary *result = @{
        @"version": @1, @"host": QAIdentifier, @"pid": @(getpid()), @"active": @(active),
        @"suites": counts, @"expected": @(expected), @"executed": @(suite.testRun.executionCount),
        @"failures": @(suite.testRun.failureCount), @"succeeded": @(succeeded)
    };
    NSData *json = [NSJSONSerialization dataWithJSONObject:result options:NSJSONWritingPrettyPrinted error:&error];
    if (!json || json.length > 65536 || ![json writeToFile:[directory stringByAppendingPathComponent:@"result.json"]
        options:NSDataWritingAtomic error:&error]) {
        fprintf(stdout, "Cannot write bounded result\n"); exit(4);
    }
    fprintf(stdout, "RESULT %s\n", [[NSString alloc] initWithData:json encoding:NSUTF8StringEncoding].UTF8String);
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, NSEC_PER_SEC), dispatch_get_main_queue(), ^{
        exit(succeeded ? 0 : 1);
    });
}
@end

int main(int argc, const char *argv[]) {
    if (argc != 3) return 4;
    // LaunchServices starts the actual .app; preserve its PID while constraining
    // all test-process environment values. Never inspect inherited credentials.
    if (getenv("PELE_NATIVE_UI_MINENV") == NULL) {
        char *home = NULL;
        if (asprintf(&home, "HOME=%s", argv[2]) < 0) return 4;
        char *minimal[] = {"PATH=/usr/bin:/bin:/usr/sbin:/sbin", home, "LANG=C", "LC_ALL=C",
                           "TZ=UTC", "PELE_NATIVE_UI_MINENV=1", NULL};
        execve(argv[0], (char *const *)argv, minimal);
        return 5;
    }
    @autoreleasepool {
        NSString *directory = [NSString stringWithUTF8String:argv[1]];
        // The OS enforces a hard bound even if a failing assertion emits a large value.
        struct rlimit limit = {8 * 1024 * 1024, 8 * 1024 * 1024};
        if (setrlimit(RLIMIT_FSIZE, &limit) != 0) return 6;
        if (!freopen([[directory stringByAppendingPathComponent:@"runtime.log"] fileSystemRepresentation], "w", stdout)) return 6;
        if (dup2(fileno(stdout), fileno(stderr)) < 0) return 6;
        setbuf(stdout, NULL);
        if (![[NSString stringWithFormat:@"%d", getpid()] writeToFile:[directory stringByAppendingPathComponent:@"pid.txt"]
            atomically:YES encoding:NSUTF8StringEncoding error:nil]) return 6;
        NSApplication *app = [NSApplication sharedApplication];
        PeleUIDelegate *delegate = [PeleUIDelegate new];
        app.delegate = delegate;
        [app setActivationPolicy:NSApplicationActivationPolicyRegular];
        [app run];
    }
    return 0;
}
