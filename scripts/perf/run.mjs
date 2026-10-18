// Matched pinned Pele/retained native Rust. No timings without --quiet-run.
process.env.TZ='UTC';
import{readFile,writeFile,mkdir}from'node:fs/promises';
import{createHash}from'node:crypto';
import{spawn,execFileSync}from'node:child_process';
import{createInterface}from'node:readline';
import{cpus,loadavg,platform,arch,totalmem,freemem}from'node:os';
import{resolve,dirname}from'node:path';
import{fileURLToPath}from'node:url';
import assert from'node:assert/strict';
import{cases,revision,now,pairedStages,componentStages}from'./cases.mjs';
import*as upstream from'pele';
Date.now=()=>now;
const root=resolve(dirname(fileURLToPath(import.meta.url)),'../..');
process.chdir(root);
const args=process.argv.slice(2),arg=(name,fallback)=>{const i=args.indexOf(name);return i<0?fallback:args[i+1]};
const timed=args.includes('--quiet-run'),allocation=args.includes('--allocations');
if(allocation&&!timed)throw Error('Allocation collection also requires a coordinated --quiet-run');
const trials=Number(arg('--trials',9)),targetMs=Number(arg('--batch-ms',12)),warmupMs=Number(arg('--warmup-ms',100));
assert(trials>=3&&trials<=51&&targetMs>=2&&targetMs<=200&&warmupMs>=0&&warmupMs<=1000);
const caseIDs=arg('--cases','').split(',').filter(Boolean);
const selected=cases().filter(c=>(!arg('--filter','')||c.id.includes(arg('--filter','')))&&(!caseIDs.length||caseIDs.includes(c.id)));
assert(selected.length,'No cases selected');
const selectedStages=arg('--stages','').split(',').filter(Boolean);
const stages=selectedStages.length?selectedStages:[...pairedStages,...componentStages];
assert(stages.every(s=>[...pairedStages,...componentStages].includes(s)));
const binary=resolve(arg('--native','scripts/perf/native/target/release/pele-perf'));
const allocBinary=resolve(arg('--allocation-native','scripts/perf/native/target/release/pele-perf-alloc'));
const sha=data=>createHash('sha256').update(data).digest('hex');
const bits=n=>{const b=Buffer.alloc(8);b.writeDoubleBE(n);return b.toString('hex')};
function host(){return{measurer:{width:(t,z,s)=>t.length*z*((s&1)?0.71:0.6)}}}
function summary(r){return{type:r.type,widthBits:bits(r.width),heightBits:bits(r.height),svgSha256:sha(Buffer.from(r.svg)),svgBytes:Buffer.byteLength(r.svg)}}
function system(){const processList=execFileSync('ps',['-Ao','pid,pcpu,comm'],{encoding:'utf8'}).trim().split('\n').slice(1).map(s=>{const m=s.trim().match(/^(\d+)\s+([\d.]+)\s+(.*)$/);return m?{pid:+m[1],cpuPercent:+m[2],name:m[3].split('/').pop()}:null}).filter(Boolean);return{loadAverage:loadavg(),freeMemory:freemem(),processes:processList.filter(p=>p.cpuPercent>=10||/^(cargo|rustc|swiftc|swift-frontend|xcodebuild|xctest|ld)$/.test(p.name)).sort((a,b)=>b.cpuPercent-a.cpuPercent).slice(0,20)}}
function buildGuard(){const s=system();const busy=s.processes.filter(p=>/^(cargo|rustc|swiftc|swift-frontend|xcodebuild|ld)$/.test(p.name));if(busy.length)throw Error('Build process active; defer timed run: '+JSON.stringify(busy));return s}
class Native{
 constructor(path){this.child=spawn(path,[],{env:{...process.env,TZ:'UTC',PELE_PERF_ALLOW_TIMING:timed?'1':'0'},stdio:['pipe','pipe','inherit']});this.pending=[];this.dead=false;createInterface({input:this.child.stdout}).on('line',line=>{const p=this.pending.shift();if(!p)return;clearTimeout(p.timeout);try{const r=JSON.parse(line);r.ok?p.resolve(r.value):p.reject(Error(r.error))}catch(e){p.reject(e)}});this.child.on('error',e=>this.fail(e));this.child.on('exit',(code,signal)=>this.fail(Error(`Native benchmark exited ${code??signal}`)))}
 fail(e){this.dead=true;for(const p of this.pending){clearTimeout(p.timeout);p.reject(e)}this.pending=[]}
 send(request){if(this.dead)return Promise.reject(Error('Native process closed'));return new Promise((resolve,reject)=>{const timeout=setTimeout(()=>{this.child.kill();reject(Error('Native command exceeded 60s'))},60_000);this.pending.push({resolve,reject,timeout});this.child.stdin.write(JSON.stringify(request)+'\n')})}
 close(){this.child.stdin.end();this.child.kill()}
}
const quantile=(v,p)=>{const a=[...v].sort((a,b)=>a-b);return a[Math.max(0,Math.ceil(a.length*p)-1)]};
function statistics(v){const median=quantile(v,.5);return{medianMs:median,sampleP95Ms:quantile(v,.95),minMs:Math.min(...v),maxMs:Math.max(...v),madMs:quantile(v.map(n=>Math.abs(n-median)),.5)}}
function ratioInterval(js,native){let seed=19;const values=[];for(let b=0;b<2000;b++){const x=[],y=[];for(let i=0;i<js.length;i++){seed=(Math.imul(seed,1664525)+1013904223)>>>0;const n=seed%js.length;x.push(js[n]);y.push(native[n])}values.push(quantile(y,.5)/quantile(x,.5))}return{medianRatio:quantile(native,.5)/quantile(js,.5),bootstrap95:[quantile(values,.025),quantile(values,.975)]}}
function jsWork(c,stage){const renderOptions={...c.options,...host()},model=upstream.parse(c.source,c.options);if(stage.startsWith('render-'))upstream.render(model,renderOptions);return()=>{if(stage.startsWith('parse'))return upstream.parse(c.source,c.options).type.length;const r=upstream.render(stage==='render-source'?c.source:model,renderOptions);return r.svg.length+r.width+r.height}}
function jsBatch(c,stage,count){global.gc?.();const work=jsWork(c,stage);let checksum=0;const start=process.hrtime.bigint();for(let i=0;i<count;i++)checksum+=work();return{elapsedNs:Number(process.hrtime.bigint()-start),checksum,count}}
function maxBatch(c,stage){return stage.includes('mutable')||stage==='graph-apply'?Math.max(1,Math.min(128,Math.floor(25_000/c.units))):1024}
async function buildInputs(){const files=execFileSync('rg',['--files','crates/pele/src','scripts/perf/native/src'],{encoding:'utf8'}).trim().split('\n').concat(['crates/pele/Cargo.toml','Cargo.toml','scripts/perf/native/Cargo.toml','scripts/perf/native/Cargo.lock']).sort();const h=createHash('sha256');for(const p of files){h.update(p+'\0');h.update(await readFile(p))}return h.digest('hex')}
const buildManifest=JSON.parse(await readFile(resolve(dirname(binary),'build-manifest.json'),'utf8'));
assert.equal(await buildInputs(),buildManifest.sourceInputSha256,'Native source changed since benchmark build');
assert.equal(sha(await readFile(binary)),buildManifest.binaries['pele-perf'],'Native benchmark binary differs from build manifest');
if(allocation)assert.equal(sha(await readFile(allocBinary)),buildManifest.binaries['pele-perf-alloc'],'Allocation binary differs from build manifest');
async function provenance(){const list=execFileSync('rg',['--files','crates/pele/src'],{encoding:'utf8'}).trim().split('\n').sort();const tree=createHash('sha256');for(const p of list){tree.update(p+'\0');tree.update(await readFile(p))}return{revision:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim(),worktree:execFileSync('git',['status','--porcelain'],{encoding:'utf8'}).trim().split('\n'),rustSourceSha256:tree.digest('hex'),buildManifest,nativeSha256:sha(await readFile(binary)),nativeLockSha256:sha(await readFile('scripts/perf/native/Cargo.lock')),harnessSha256:Object.fromEntries(await Promise.all(['run.mjs','cases.mjs','build.mjs'].map(async p=>[p,sha(await readFile('scripts/perf/'+p))]))),upstream:revision,oracleSha256:sha(await readFile('node_modules/pele/dist/pele.js')),node:process.version,rustc:execFileSync('/bin/bash',['-c','source scripts/env.sh && rustc --version'],{encoding:'utf8'}).trim(),platform:platform(),arch:arch(),cpu:cpus()[0]?.model,logicalCPUs:cpus().length,totalMemory:totalmem()}}
const report={schemaVersion:1,mode:timed?'local-timing':'correctness-only',timestamp:new Date().toISOString(),...await provenance(),settings:{trials,targetBatchMs:targetMs,warmupMs,explicitNodeGC:!!global.gc,fixedClock:now,timeZone:'UTC',resource:'fixed UTF16 length × size × (bold?.71:.6)',freshRetainedStatePerBatch:true,warmRenderBeforeBatch:true},scope:{paired:'Same source, public options, fixed measurement/clock and exact SVG+dimension verification. JS raw model render compared with Rust typed retained and mutable publication APIs; publication returns an additional cloned graph.',nativeComponents:'Separate native API components, not matched JS operations or end-to-end Swift costs.',excluded:'Startup, module loading, process setup, NDJSON control pipe, SVG DOM insertion, layout/paint and GPU. JS GC during a batch remains part of observed runtime.',externalLoad:'Host is not assumed globally idle. External xctest may consume one core; unrelated processes are never stopped.'},systemBefore:timed?buildGuard():system(),cases:[],rows:[],allocationRows:[]};
let native,alloc;
try{
 native=new Native(binary);if(allocation)alloc=new Native(allocBinary);
 for(const c of selected){
  const expected=summary(upstream.render(c.source,{...c.options,...host()}));
  const prepared=await native.send({op:'prepare',case:c});assert.deepEqual(prepared.output,expected,c.id+' initial output');
  if(alloc){const p=await alloc.send({op:'prepare',case:c});assert.deepEqual(p.output,expected,c.id+' allocation output')}
  const verification=await native.send({op:'verify',case:c.id});for(const output of verification.outputs)assert.deepEqual(output,expected,c.id+' retained output');
  report.cases.push({...c,sourceSha256:sha(Buffer.from(c.source)),sourceBytes:Buffer.byteLength(c.source),output:expected,native:prepared});
  if(!timed){console.log('verified '+c.id);continue}
  for(const stage of stages){
   const paired=pairedStages.includes(stage),counts={native:1,js:1};
   const warmups={native:{operations:0,elapsedNs:0},js:{operations:0,elapsedNs:0}};
   for(let i=0;i<6||warmups.native.elapsedNs<warmupMs*1e6;i++){const count=i<6?1:Math.min(32,maxBatch(c,stage)),t=await native.send({op:'measure',case:c.id,stage,count});warmups.native.operations+=count;warmups.native.elapsedNs+=t.elapsedNs}
   while(counts.native<maxBatch(c,stage)){const t=await native.send({op:'measure',case:c.id,stage,count:counts.native});if(t.elapsedNs>=targetMs*1e6)break;counts.native*=2}
   counts.native=Math.min(counts.native,maxBatch(c,stage));
   if(paired){for(let i=0;i<6||warmups.js.elapsedNs<warmupMs*1e6;i++){const count=i<6?1:Math.min(32,maxBatch(c,stage)),t=jsBatch(c,stage,count);warmups.js.operations+=count;warmups.js.elapsedNs+=t.elapsedNs}while(counts.js<maxBatch(c,stage)&&jsBatch(c,stage,counts.js).elapsedNs<targetMs*1e6)counts.js*=2;counts.js=Math.min(counts.js,maxBatch(c,stage))}
   const nativeSamples=[],jsSamples=[],details=[];
   for(let trial=0;trial<trials;trial++)for(const runtime of trial%2?['native','js']:['js','native']){
    if(runtime==='js'){if(paired){const t=jsBatch(c,stage,counts.js);jsSamples.push(t.elapsedNs/t.count/1e6)}}
    else{const t=await native.send({op:'measure',case:c.id,stage,count:counts.native});nativeSamples.push(t.elapsedNs/t.count/1e6);details.push(t)}
   }
   report.rows.push({case:c.id,stage,pairedJS:paired,counts,warmups,native:statistics(nativeSamples),...(paired?{js:statistics(jsSamples),nativeOverJS:ratioInterval(jsSamples,nativeSamples)}:{}),nativeSamplesMs:nativeSamples,jsSamplesMs:jsSamples,nativeDetails:details});
   if(alloc){report.allocationRows.push({case:c.id,stage,count:1,...await alloc.send({op:'allocations',case:c.id,stage,count:1})})}
  }
  for(const output of(await native.send({op:'verify',case:c.id})).outputs)assert.deepEqual(output,expected,c.id+' post-run output');
  console.log('measured '+c.id);
 }
 assert.equal(await buildInputs(),buildManifest.sourceInputSha256,'Native source changed during benchmark');
 report.systemAfter=system();report.completedAt=new Date().toISOString();
 const output=arg('--output',timed?'dist/evidence/perf-native-js.json':'dist/evidence/perf-verified.json');await mkdir(dirname(output),{recursive:true});await writeFile(output,JSON.stringify(report,null,2)+'\n');console.log(output);
}finally{native?.close();alloc?.close()}
