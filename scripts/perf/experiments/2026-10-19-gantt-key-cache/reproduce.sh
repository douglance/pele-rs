#!/bin/bash
set -euo pipefail
# Isolated allocation/correctness capture only. No elapsed benchmark commands.
task_repo=${1:?Usage: reproduce.sh /absolute/path/to/pele-rs [--prepare-only]}
task_evidence=$(cd "$(dirname "$0")" && pwd)
task_checkout=$(mktemp -d "${TMPDIR:-/tmp}/pele-gantt-key-reproduce.XXXXXX")
git -C "$task_repo" worktree add --detach "$task_checkout" 32e845d4b0e63476dcaba8ebe4dc8730bdc7fdd0
git -C "$task_checkout" apply "$task_evidence/harness.patch"
if [[ ! -d "$task_repo/node_modules/pele" ]]; then
  printf '%s\n' 'Pinned development Pele artifact missing; no install performed.' >&2
  exit 1
fi
ln -s "$task_repo/node_modules" "$task_checkout/node_modules"
node --input-type=module - "$task_checkout" <<'JS'
import {readFile} from 'node:fs/promises';import {createHash} from 'node:crypto';import assert from 'node:assert/strict';const bytes=await readFile(process.argv[2]+'/node_modules/pele/dist/pele.js');assert.equal(createHash('sha256').update(bytes).digest('hex'),'d1ebc1889f3069d7759fe5db34c4ad4b4823285edbee2b537d0ef3d505a76d8d');
JS
git -C "$task_checkout" apply --check "$task_evidence/runtime.patch"
if [[ ${2:-} == --prepare-only ]]; then printf 'Prepared isolated checkout: %s\n' "$task_checkout"; exit 0; fi
node "$task_evidence/build-allocation.mjs" "$task_checkout"
node "$task_evidence/capture.mjs" "$task_checkout" "$task_checkout/baseline-allocations.json"
git -C "$task_checkout" apply "$task_evidence/runtime.patch"
node "$task_evidence/build-allocation.mjs" "$task_checkout"
node "$task_evidence/capture.mjs" "$task_checkout" "$task_checkout/candidate-allocations.json"
(cd "$task_checkout"; source scripts/env.sh; CARGO_TARGET_DIR="$task_checkout/scripts/perf/native/target" env -u GANTT_FILTER -u GANTT_SKIP_STRESS cargo test --locked --release -p pele-rs --test public_gantt --test gantt_compat)
node "$task_evidence/compare.mjs" "$task_checkout/baseline-allocations.json" "$task_checkout/candidate-allocations.json" "$task_checkout/comparison.json"
printf 'Isolated reports retained: %s\n' "$task_checkout"
