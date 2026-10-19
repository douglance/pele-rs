// Compare isolated property-cache attribution runs; no runtime or timing work.
import {readFile,writeFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
const [beforePath,afterPath,outPath]=process.argv.slice(2);
assert(beforePath&&afterPath&&outPath,'Usage: compare-property.mjs before.json after.json output.json');
const beforeBytes=await readFile(beforePath),afterBytes=await readFile(afterPath);
const a=JSON.parse(beforeBytes),b=JSON.parse(afterBytes);
assert.equal(a.mode,'allocation-only');assert.equal(b.mode,'allocation-only');
assert.equal(a.cases.length,b.cases.length);
for(let i=0;i<a.cases.length;i++){
 const x=a.cases[i],y=b.cases[i];
 assert.equal(x.id,y.id);assert.equal(x.sourceSha256,y.sourceSha256);
 assert.deepEqual(x.output,y.output,x.id+' whole SVG/dimension/type');
 assert.equal(x.native.graphSha256,y.native.graphSha256,x.id+' complete graph identity/order');
 assert.deepEqual(x.native.propertyGraphs,y.native.propertyGraphs,x.id+' repeated/unique rawUTF16 graph');
}
const deltas=[];
for(const x of a.allocationRows){
 const y=b.allocationRows.find(v=>v.case===x.case&&v.stage===x.stage);assert(y);
 const delta={case:x.case,stage:x.stage};
 for(const key of ['allocations','reallocations','requestedBytes','liveBytesDelta','peakAdditionalRequestedBytes'])delta[key]={before:x[key],after:y[key],change:y[key]-x[key],percent:x[key]?(y[key]-x[key])/x[key]*100:null};
 deltas.push(delta);
}
const result={schemaVersion:1,scope:'Isolated GraphWriter positive property-name cache allocation attribution; no elapsed timings or speed claim.',before:{path:beforePath,sha256:createHash('sha256').update(beforeBytes).digest('hex'),build:a.buildManifest,graphWriterBytes:a.cases[0].native.graphWriterBytes},after:{path:afterPath,sha256:createHash('sha256').update(afterBytes).digest('hex'),build:b.buildManifest,graphWriterBytes:b.cases[0].native.graphWriterBytes},cases:a.cases.length,allSvgOutputsEqual:true,allGraphIdentitiesEqual:true,rawUtf16AndUniquePropertyGraphsEqual:true,deltas};
await writeFile(outPath,JSON.stringify(result,null,2)+'\n');
console.table(deltas.filter(x=>x.case.endsWith('/medium')).map(x=>({case:x.case,stage:x.stage,allocationChange:x.allocations.change,allocationPercent:x.allocations.percent?.toFixed(1),bytesChange:x.requestedBytes.change,peakChange:x.peakAdditionalRequestedBytes.change})));
console.log(outPath);
