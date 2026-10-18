// Compare public results; parser/layout internals are deliberately not test seams.
import { readFile } from 'node:fs/promises';
import assert from 'node:assert/strict';
import { parse } from 'pele';
const fixtures = JSON.parse(await readFile('fixtures/manifest.json', 'utf8'));
let passed = 0;
for (const fixture of fixtures.filter(f => !f.error)) {
  const source = fixture.source ?? await readFile(`fixtures/${fixture.file}`, 'utf8');
  const rust = JSON.parse(await readFile(`dist/evidence/rust/${fixture.name}.json`, 'utf8')).result;
  const swift = JSON.parse(await readFile(`dist/evidence/swift/${fixture.name}.json`, 'utf8'));
  assert.deepEqual(swift, rust, `${fixture.name}: Swift/C and Rust geometry/SVG differ`);
  const upstream = parse(source);
  assert.equal(upstream.nodes.size, rust.geometry.nodes.length, fixture.name);
  assert.equal(upstream.edges.length, rust.geometry.edges.length, fixture.name);
  for (const node of rust.geometry.nodes) {
    const original = upstream.nodes.get(node.id);
    assert.ok(original, `${fixture.name}: missing upstream node ${node.id}`);
    if (fixture.name !== 'injection' && fixture.name !== 'combining') {
      // Pele stores entity-encoded markup; the Rust contract stores literal label text.
      assert.equal((original.text ?? node.id).replaceAll('\\n', '\n').replaceAll('\r\n', '\n'), node.label, `${fixture.name}: node ${node.id} label`);
    }
  }
  for (const [index, edge] of rust.geometry.edges.entries()) {
    assert.equal(upstream.edges[index].start, edge.source, fixture.name);
    assert.equal(upstream.edges[index].end, edge.target, fixture.name);
  }
  passed++;
}
console.log(`${passed} fixtures: Swift/C = Rust exact geometry/SVG; Pele node/edge semantics agree`);
