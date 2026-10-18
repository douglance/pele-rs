#!/bin/bash
set -euo pipefail
# Allocation attribution only. Never measures elapsed benchmarks or edits Root files.
task_repo=${1:?Usage: reproduce.sh /absolute/path/to/pele-rs [--prepare-only]}
task_evidence=$(cd "$(dirname "$0")" && pwd)
task_checkout=$(mktemp -d "${TMPDIR:-/tmp}/pele-property-reproduce.XXXXXX")
git -C "$task_repo" worktree add --detach "$task_checkout" a34ca6c91040fb8a9d7915d7c03a4eda10396c17
git -C "$task_checkout" apply "$task_evidence/baseline-overlay.patch"
git -C "$task_checkout" apply "$task_evidence/harness.patch"
cp "$task_evidence/compare-property.mjs" "$task_checkout/scripts/perf/compare-property.mjs"
if [[ ! -d "$task_repo/node_modules/pele" ]]; then
  printf '%s\n' 'Pinned development Pele dependency missing; no install performed.' >&2
  exit 1
fi
ln -s "$task_repo/node_modules" "$task_checkout/node_modules"
cd "$task_checkout"
node --input-type=module - "$task_evidence/evidence.json" <<'JS'
import {readFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
const evidence=JSON.parse(await readFile(process.argv[2]));
const bytes=await readFile('node_modules/pele/dist/pele.js');
assert.equal(createHash('sha256').update(bytes).digest('hex'),evidence.oracleSha256,'Pinned Pele artifact changed');
JS
git apply --check "$task_evidence/runtime.patch"
if [[ ${2:-} == --prepare-only ]]; then
  printf 'Prepared isolated checkout: %s\n' "$task_checkout"
  exit 0
fi
node scripts/perf/build.mjs
node --expose-gc scripts/perf/run.mjs --allocation-only \
  --stages parse-public,parse-public-fresh-registry,property-repeated,property-unique \
  --output dist/evidence/property-baseline-allocations.json
git apply "$task_evidence/runtime.patch"
node scripts/perf/build.mjs
node --expose-gc scripts/perf/run.mjs --allocation-only \
  --stages parse-public,parse-public-fresh-registry,property-repeated,property-unique \
  --output dist/evidence/property-prototype-allocations.json
node scripts/perf/compare-property.mjs \
  dist/evidence/property-baseline-allocations.json \
  dist/evidence/property-prototype-allocations.json \
  dist/evidence/property-allocation-comparison.json
printf 'Isolated reports retained: %s/dist/evidence\n' "$task_checkout"
