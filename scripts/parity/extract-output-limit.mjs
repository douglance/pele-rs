// Development-only pinned Pele output-limit oracle. Consumer runtime is Rust.
import {parse, render} from 'pele';
import {readFile, writeFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import assert from 'node:assert/strict';
process.env.TZ = 'UTC';
const revision = '458322a5b8abb1ae57ddf127c32017ab9fb14a92';
const root = process.argv[2] ?? '/tmp/pele-upstream-458322a5';
assert.equal(execFileSync('git', ['rev-parse', 'HEAD'], {cwd: root, encoding: 'utf8'}).trim(), revision);
const paths = ['src/core.ts', 'src/errors.ts', 'src/types.ts', 'src/diagrams/class/render.ts', 'src/diagrams/gantt/db.ts', 'src/diagrams/gantt/render.ts', 'src/text/measurer.ts'];
execFileSync('git', ['diff', '--quiet', revision, '--', ...paths], {cwd: root});
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const provenance = {};
for (const path of paths) provenance[path] = sha(await readFile(root + '/' + path));
const oracleSha256 = sha(await readFile('node_modules/pele/dist/pele.js'));
assert.equal(oracleSha256, 'd1ebc1889f3069d7759fe5db34c4ad4b4823285edbee2b537d0ef3d505a76d8d');
const now = 1704067200000;
Date.now = () => now;
const options = {outputLimit: 0, now};
const host = {measure: 'fixed', now, timezone: 'UTC'};
const bits = n => { const b = Buffer.alloc(8); b.writeDoubleBE(n); return b.toString('hex'); };
const number = n => ({kind: 'number', bits: bits(n)});
const decode = limit => limit.kind === 'undefined' ? undefined : limit.kind === 'null' ? null : Buffer.from(limit.bits, 'hex').readDoubleBE();
const normalize = v => JSON.parse(JSON.stringify(v, (_, x) => x instanceof Map ? [...x] : x instanceof Set ? [...x] : x));
// Existing immutable transport replaces Maps but intentionally retains JSON's
// ordinary Set projection ({}). The public graph projection above expands Sets.
const typedProjection = v => JSON.parse(JSON.stringify(v, (_, x) => x instanceof Map ? [...x] : x));
function schedule(model) {
  if (model.type !== 'gantt') return undefined;
  return {
    now: model.now, work: model.work, excludedDays: model.excludedDays, datedTokens: model.datedTokens,
    tasks: model.tasks.map((t, i) => ({id: t.id, startTime: t.startTime === undefined ? 'undefined' : +t.startTime, endTime: t.endTime === undefined ? 'undefined' : +t.endTime, renderEndTime: t.renderEndTime === null ? null : +t.renderEndTime, processed: t.processed, manualEndTime: t.manualEndTime, byIdSame: model.byId.get(t.id) === t, previousEndSame: i > 0 && t.startTime === model.tasks[i - 1].endTime})),
  };
}
function run(source, limit, fail = false) {
  const model = parse(source), before = normalize(model), typedProjectionBefore = typedProjection(model), calls = [];
  const tasks = model.tasks;
  const measurer = {width(text, size, style) { calls.push([Array.from({length:text.length}, (_, i) => text.charCodeAt(i)), bits(size), style]); if (fail) throw new Error('output-limit host stopped'); return text.length * size * (style & 1 ? .71 : .6); }};
  let expected;
  try {
    const result = render(model, {...options, outputLimit: decode(limit), measurer});
    assert.ok(result.svg.isWellFormed(), 'This fixture covers well-formed SVG only');
    expected = {type: result.type, result, widthBits: bits(result.width), heightBits: bits(result.height)};
  } catch (error) {
    if (fail) {
      assert.equal(error.name, 'Error');
      assert.equal(error.message, 'output-limit host stopped');
      expected = {hostError:{name:error.name,message:error.message}};
    } else {
      assert.equal(error.name, 'PeleError');
      expected = {error: {name:error.name, code:error.code, message:error.message, type:error.type, line:error.line, column:error.column, snippet:error.snippet}};
    }
  }
  return {...expected, typedProjectionBefore, modelBefore: before, modelAfter: normalize(model), calls, ...(model.type === 'gantt' ? {schedule: schedule(model), tasksIdentityRetained: model.tasks === tasks} : {})};
}
const seeds = [
  ['pie-ascii', 'pie title Output\n"A":1\n"B":2'],
  ['pie-astral', 'pie title 😀\n"é😀":1'],
  ['pie-crlf', '\ufeffpie\r\n"😀":1\r\n'],
  ['info', 'info title 😀'],
  ['class-ascii', 'classDiagram\nclass LongIdentifier\nLongIdentifier : +String field'],
  ['class-astral', 'classDiagram\nclass A["😀"]'],
  ['gantt', 'gantt\ndateFormat YYYY-MM-DD\nsection S\nA:a,2024-01-01,1d\nB:b,after a,2d'],
  ['gantt-astral', 'gantt\ntitle 😀\ndateFormat YYYY-MM-DD\nsection S\n😀:a,2024-01-01,1d'],
];
const values = [{kind:'undefined'}, {kind:'null'}, ...[NaN,Infinity,-Infinity,0,-0,-1,-.5,.5,1,Number.MIN_VALUE,-Number.MIN_VALUE,-1e-7,-1e21,Number.MAX_VALUE,Number.MAX_SAFE_INTEGER,3999999.5,4000000,4000000.5].map(number), {kind:'number',bits:'fff8000000000042'}];
const cases = [];
for (const [name, source] of seeds) {
  const baseline = run(source, number(Infinity));
  assert.ok(baseline.result, name);
  const length = baseline.result.svg.length;
  assert.ok(length < 100000, 'No large rendering is needed to exercise numeric semantics');
  for (const [index, limit] of [...values, ...[-.5,0,.5].map(delta => number(length + delta))].entries()) {
    const expected = run(source, limit);
    assert.deepEqual(expected.calls, baseline.calls, 'Output rejection follows all measurement callbacks');
    if (expected.schedule) assert.deepEqual(expected.schedule, baseline.schedule);
    cases.push({id:`${name}/${index}`, source, limit, expected:{...expected, outputUTF16Length:length}});
  }
}
assert.equal(cases.length,192);
const ordering=seeds.filter(([id])=>id==='class-ascii'||id==='gantt').map(([id,source])=>({id,source,limit:number(0),expected:run(source,number(0),true)}));
await writeFile('fixtures/pele/output-limit.json', JSON.stringify({revision, oracleSha256, provenance, options, host, scope:'Well-formed returned SVG; exact output-limit values only. Source and edge APIs, malformed UTF16, arbitrary coercion and incremental allocation bounds are outside this fixture. Ordering records preserve the pinned JS Error; native host error identity/category is asserted through its existing host contract separately.', cases, ordering}, null, 2)+'\n');
console.log(`${cases.length} pinned output-limit cases; ${cases.filter(c=>c.expected.result).length} full model/SVG, ${cases.filter(c=>c.expected.error).length} complete errors`);
