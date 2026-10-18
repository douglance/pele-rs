#!/usr/bin/env bash
set -euo pipefail
repo_dir="$(cd "$(dirname "$0")/.." && pwd)"
# The shell forwards only ordinary home/tool settings, never the parent secrets.
exec /usr/bin/env -i PATH=/usr/bin:/bin:/usr/sbin:/sbin HOME="${HOME:?}" LANG=C LC_ALL=C TZ=UTC \
    MACOSX_DEPLOYMENT_TARGET=14.0 /usr/bin/python3 - "$repo_dir" "$@" <<'PYTHON'
import argparse
import hashlib
import json
import os
from pathlib import Path
import selectors
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import plistlib

root = Path(sys.argv[1])
parser = argparse.ArgumentParser(description="Run synchronous native UI features in an actual signed AppKit QA application")
parser.add_argument("--scratch-path", type=Path, default=root / "target/swift-ui")
parser.add_argument("--timeout", type=int, default=120, help="GUI completion deadline in seconds (1–300)")
args = parser.parse_args(sys.argv[2:])
if sys.platform != "darwin" or not 1 <= args.timeout <= 300:
    parser.error("Native UI verification requires macOS and a timeout from 1 to 300 seconds")
evidence = root / "dist/evidence/native-ui"
evidence.mkdir(parents=True, exist_ok=True)
run = Path(tempfile.mkdtemp(prefix="run-", dir=evidence))
# Keep the caller's path spelling: Xcode treats /tmp and /private/tmp as
# different allowed roots and may discard cached generated C module maps.
scratch = Path(os.path.abspath(args.scratch_path.expanduser()))
safe = {"PATH": "/usr/bin:/bin:/usr/sbin:/sbin", "HOME": str(Path.home()), "LANG": "C", "LC_ALL": "C", "TZ": "UTC", "MACOSX_DEPLOYMENT_TARGET": "14.0"}
commands = []
qa_pid = None
executable = None

def digest(path):
    with path.open("rb") as stream:
        result = hashlib.sha256()
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            result.update(block)
        return result.hexdigest()

def write_json(path, value):
    encoded = json.dumps(value, indent=2).encode()
    if len(encoded) > 1024 * 1024:
        raise RuntimeError("UI provenance exceeds its 1 MiB budget")
    path.write_bytes(encoded)

def inputs():
    files = list((root / "bindings/swift/Sources").rglob("*.swift"))
    files += list((root / "bindings/swift/Tests").rglob("*.swift"))
    files += [root / "bindings/swift/Package.swift", root / "scripts/test-swift-ui.sh", root / "scripts/native-ui-host.m"]
    artifact = root / "bindings/swift/Artifacts/CPele.xcframework"
    files += list(artifact.rglob("*.a")) + list(artifact.rglob("*.h")) + list(artifact.rglob("module.modulemap"))
    files.append(artifact / "Info.plist")
    return {str(path.relative_to(root)): digest(path) for path in sorted(set(files))}

def terminate_group(process):
    if process.poll() is None:
        try:
            os.killpg(process.pid, signal.SIGTERM)
        except ProcessLookupError:
            return
        try:
            process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            process.wait(timeout=3)

def command(arguments, name, timeout=600):
    path = run / (name + ".log")
    process = subprocess.Popen([str(value) for value in arguments], cwd=root, env=safe,
        stdout=subprocess.PIPE, stderr=subprocess.STDOUT, start_new_session=True)
    selector = selectors.DefaultSelector()
    selector.register(process.stdout, selectors.EVENT_READ)
    deadline, written, failure = time.monotonic() + timeout, 0, None
    try:
        with path.open("wb") as log:
            while selector.get_map():
                if time.monotonic() >= deadline:
                    failure = name + " exceeded its deadline"
                    break
                for key, _ in selector.select(timeout=0.1):
                    block = os.read(key.fd, 65536)
                    if not block:
                        selector.unregister(key.fileobj)
                        continue
                    available = max(0, 8 * 1024 * 1024 - written)
                    log.write(block[:available]); written += len(block[:available])
                    if len(block) > available:
                        failure = name + " exceeded its 8 MiB log budget"
                        break
                if failure:
                    break
        if failure:
            terminate_group(process)
        status = process.wait(timeout=3)
    finally:
        selector.close()
        process.stdout.close()
        terminate_group(process)
    commands.append({"arguments": [str(value) for value in arguments], "exit": status, "log": path.name})
    write_json(run / "commands.json", commands)
    if failure or status:
        raise RuntimeError(failure or name + " failed; see " + path.name)
    return path.read_text(errors="replace").strip()

def owns_process(pid):
    if not executable or pid <= 1:
        return False
    process = subprocess.run(["/bin/ps", "-p", str(pid), "-o", "comm="], env=safe, capture_output=True, text=True, timeout=3)
    return bool(process.stdout.strip()) and Path(process.stdout.strip()).resolve() == executable.resolve()

def stop_qa():
    if qa_pid and owns_process(qa_pid):
        try:
            os.kill(qa_pid, signal.SIGTERM)
        except ProcessLookupError:
            return
        deadline = time.monotonic() + 3
        while owns_process(qa_pid) and time.monotonic() < deadline:
            time.sleep(0.05)
        if owns_process(qa_pid):
            try:
                os.kill(qa_pid, signal.SIGKILL)
            except ProcessLookupError:
                pass

try:
    write_json(evidence / "latest.json", {"run": str(run.relative_to(root)), "succeeded": False, "status": "running"})
    before = inputs()
    revision = command(["/usr/bin/git", "rev-parse", "HEAD"], "revision", timeout=10)
    write_json(run / "provenance.json", {"revision": revision, "inputs": before, "scratch": str(scratch)})
    swift = ["/usr/bin/swift", "build", "-c", "release", "--package-path", root / "bindings/swift", "--scratch-path", scratch]
    command(swift + ["--build-tests"], "swift-build")
    binary_path = Path(command(swift + ["--show-bin-path"], "swift-bin-path", timeout=60).splitlines()[-1])
    candidates = [binary_path / name for name in ["PeleTests.xctest", "PelePackageTests.xctest"] if (binary_path / name).is_dir()]
    if len(candidates) != 1:
        raise RuntimeError("Cannot identify the matching current Swift test bundle")
    if before != inputs():
        raise RuntimeError("Swift source or native SDK changed during the UI build")
    shutil.copytree(candidates[0], run / "PeleTests.xctest")
    with (run / "PeleTests.xctest/Contents/Info.plist").open("rb") as stream:
        test_name = plistlib.load(stream)["CFBundleExecutable"]
    original = candidates[0] / "Contents/MacOS" / test_name
    copied = run / "PeleTests.xctest/Contents/MacOS" / test_name
    if digest(original) != digest(copied):
        raise RuntimeError("Copied test bundle differs from the current build")
    developer = Path(command(["/usr/bin/xcode-select", "-p"], "developer-path", timeout=10))
    framework = developer / "Platforms/MacOSX.platform/Developer/Library/Frameworks"
    library = developer / "Platforms/MacOSX.platform/Developer/usr/lib"
    app = run / "Pele Native UI QA.app"
    (app / "Contents/MacOS").mkdir(parents=True)
    executable = app / "Contents/MacOS/PeleNativeUIQA"
    (app / "Contents/Info.plist").write_bytes(plistlib.dumps({
        "CFBundleIdentifier": "com.unlocalhosted.pele.native-ui-qa", "CFBundleName": "Pele Native UI QA",
        "CFBundleExecutable": "PeleNativeUIQA", "CFBundlePackageType": "APPL", "CFBundleVersion": "1",
        "CFBundleShortVersionString": "1", "LSMinimumSystemVersion": "14.0", "NSPrincipalClass": "NSApplication", "LSUIElement": False}))
    shutil.copy2(root / "scripts/native-ui-host.m", run / "native-ui-host.m")
    command(["/usr/bin/clang", "-fobjc-arc", "-fblocks", "-mmacosx-version-min=14.0", "-F", framework,
        "-framework", "AppKit", "-framework", "XCTest", "-Wl,-rpath," + str(framework), "-Wl,-rpath," + str(library),
        run / "native-ui-host.m", "-o", executable], "host-build", timeout=60)
    command(["/usr/bin/codesign", "--force", "--sign", "-", app], "host-sign", timeout=30)
    command(["/usr/bin/codesign", "--verify", "--deep", "--strict", app], "host-sign-verify", timeout=30)
    write_json(run / "provenance.json", {"revision": revision, "inputs": before, "scratch": str(scratch),
        "testBundleSHA256": digest(copied), "hostSHA256": digest(executable), "host": "com.unlocalhosted.pele.native-ui-qa"})
    command(["/usr/bin/open", "-n", app, "--args", run, Path.home()], "host-launch", timeout=30)
    deadline = time.monotonic() + args.timeout
    result_file = run / "result.json"
    while not result_file.exists():
        if (run / "pid.txt").exists():
            qa_pid = int((run / "pid.txt").read_text())
            if not owns_process(qa_pid):
                raise RuntimeError("Native UI host exited without a complete result; see runtime.log")
        if time.monotonic() >= deadline:
            raise RuntimeError("Native UI GUI deadline exceeded; an active desktop is required (no headless fallback)")
        time.sleep(0.05)
    if result_file.stat().st_size > 65536:
        raise RuntimeError("Native UI result exceeds its 64 KiB budget")
    result = json.loads(result_file.read_text())
    qa_pid = result["pid"]
    if result.get("version") != 1 or result.get("host") != "com.unlocalhosted.pele.native-ui-qa" or not result.get("active") or not result.get("succeeded") or result.get("failures") != 0 or result.get("executed") != result.get("expected") or set(result.get("suites", {})) != {"MountTests", "ZoomTests"}:
        raise RuntimeError("Native UI feature assertions failed; see runtime.log and result.json")
    deadline = time.monotonic() + 5
    while owns_process(qa_pid) and time.monotonic() < deadline:
        time.sleep(0.05)
    if owns_process(qa_pid):
        raise RuntimeError("Completed native UI host did not exit")
    if before != inputs():
        raise RuntimeError("Swift source or native SDK changed during the UI run")
    write_json(run / "runner-result.json", {"succeeded": True, "executed": result["executed"], "pid": qa_pid})
    write_json(evidence / "latest.json", {"run": str(run.relative_to(root)), "succeeded": True})
    print(f"Native GUI Mount + Zoom: {result['executed']} passed. Evidence: {run}")
except Exception as error:
    write_json(run / "runner-result.json", {"succeeded": False, "error": str(error), "pid": qa_pid})
    write_json(evidence / "latest.json", {"run": str(run.relative_to(root)), "succeeded": False, "status": "failed"})
    print(f"Native GUI verification failed: {error}. Evidence: {run}", file=sys.stderr)
    sys.exit(1)
finally:
    stop_qa()
PYTHON
