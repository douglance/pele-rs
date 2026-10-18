// Development-only bounded Pele oracle. No JS algorithm ships in native clients.
import {readFile, writeFile, readdir} from 'node:fs/promises';
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {gzipSync} from 'node:zlib';
import {fileURLToPath, pathToFileURL} from 'node:url';
import {resolve} from 'node:path';
import assert from 'node:assert/strict';
const option=(name,fallback)=>{const at=process.argv.indexOf(name);return at<0?fallback:process.argv[at+1]};
const script=fileURLToPath(import.meta.url), repo=resolve(option('--repo',fileURLToPath(new URL('../../',import.meta.url))));
const upstream=resolve(option('--upstream','/tmp/pele-upstream-458322a5'));
const output=resolve(option('--out',repo+'/fixtures/pele/all-render-numbers.json.gz'));
const sourcesPath=resolve(option('--sources',fileURLToPath(new URL('./all-render-numbers-sources.json',import.meta.url))));
const sources=JSON.parse(await readFile(sourcesPath,'utf8'));
const revision='458322a5b8abb1ae57ddf127c32017ab9fb14a92', NOW=1704067200000;
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
const bits=number=>{const b=Buffer.alloc(8);b.writeDoubleBE(number);return b.toString('hex')};
const tag=value=>({kind:'number',bits:bits(value)});
const number=value=>Buffer.from(value.bits,'hex').readDoubleBE();
const baseline={fontSize:13,maxWidth:220,directionBreakpoint:80,padding:3,now:NOW+.5};
const budget={workerTimeoutMs:2000,heapMiB:192,outputBytes:256*1024,callbackWork:2000};
const env={PATH:'/usr/bin:/bin',TZ:'UTC',LANG:'C',LC_ALL:'C'};
function graph(root){
  const builtinPrototypes=new Set([Object.prototype,Array.prototype,Map.prototype,Set.prototype,Date.prototype,null]),prototypes=new Map(),ids=new Map(),nodes=[],functions=[];
  if(root&&typeof root==='object'&&!builtinPrototypes.has(Object.getPrototypeOf(root)))prototypes.set(Object.getPrototypeOf(root),1);
  const atom=value=>{
    if(value===undefined)return ['undefined'];if(value===null)return ['null'];
    if(typeof value==='boolean')return ['bool',value];
    if(typeof value==='string')return ['string',Array.from({length:value.length},(_,i)=>value.charCodeAt(i))];
    if(typeof value==='number')return ['number',bits(value)];
    if(typeof value==='function'){
      let index=functions.findIndex(entry=>entry.value===value);
      if(index<0){index=functions.length;functions.push({value,shape:{arity:value.length,name:value.name,ownKeys:Reflect.ownKeys(value)}})}
      return ['functionObservation',index];
    }
    if(!ids.has(value)){ids.set(value,ids.size);nodes.push(null)}return ['ref',ids.get(value)];
  };
  const r=atom(root);
  for(const [value,id] of ids){
    const prototype=Object.getPrototypeOf(value);
    const kind=Array.isArray(value)?'array':value instanceof Map?'map':value instanceof Set?'set':value instanceof Date?'date':!builtinPrototypes.has(prototype)?'instance':'object';
    if(kind==='instance'&&!prototypes.has(prototype))prototypes.set(prototype,prototypes.size+1);
    const node={kind};if(kind==='instance')node.prototype=prototypes.get(prototype);if(kind==='array')node.length=value.length;
    if(kind==='map')node.entries=[...value].map(([key,value])=>[atom(key),atom(value)]);
    if(kind==='set')node.values=[...value].map(atom);if(kind==='date')node.milliseconds=bits(+value);
    node.properties=Object.getOwnPropertyNames(value).filter(key=>!(kind==='array'&&key==='length')).map(key=>{
      const d=Object.getOwnPropertyDescriptor(value,key);assert.ok('value'in d);
      return [Array.from({length:key.length},(_,i)=>key.charCodeAt(i)),atom(d.value),(d.enumerable?1:0)|(d.writable?2:0)|(d.configurable?4:0)];
    });nodes[id]=node;
  }
  return {root:r,nodes,...(functions.length?{functionObservations:functions.map(entry=>entry.shape),functionScope:'Observation identities only; no fabricated native schema method IDs and no function-source/name parity promise.'}:{})};
}
function schedule(model){if(model.type!=='gantt')return undefined;return {
  nowBits:bits(model.now),workBits:bits(model.work),excludedDays:model.excludedDays,datedTokens:model.datedTokens,
  tasks:model.tasks.map((task,i)=>({id:task.id,processed:task.processed,start:task.startTime===undefined?{kind:'undefined'}:tag(+task.startTime),
    end:task.endTime===undefined?{kind:'undefined'}:tag(+task.endTime),renderEnd:task.renderEndTime===null?{kind:'null'}:tag(+task.renderEndTime),
    byIdSame:model.byId.get(task.id)===task,previousEndSame:i>0&&task.startTime===model.tasks[i-1].endTime})),
}}
if(option('--worker',false)) {
  const c=JSON.parse(option('--case','{}'));let calls=[];
  const {parse,render}=await import(pathToFileURL(repo+'/node_modules/pele/dist/pele.js').href);
  const options={...baseline};
  for(const [key,value] of Object.entries(c.numeric))options[key]=number(value);
  Date.now=()=>{calls.push(['clock',bits(NOW)]);return NOW};
  const measurer={width(text,size,style){
    if(calls.length>=budget.callbackWork)throw Error('all-render-numbers callback harness budget exceeded');
    const width=text.length*size*(style&1?.71:.6);
    calls.push(['width',Array.from({length:text.length},(_,i)=>text.charCodeAt(i)),bits(size),style,bits(width)]);
    return width;
  }};
  const run=input=>{try {
    const result=render(input,{...options,measurer});
    assert.ok(result.svg.isWellFormed(),'SVG contains lone UTF16 surrogate');
    const bytes=Buffer.byteLength(result.svg);
    return bytes>budget.outputBytes?{harness:{kind:'returned-output-budget',bytes,sha256:hash(result.svg)}}:
      {result,widthBits:bits(result.width),heightBits:bits(result.height)};
  } catch(error) {
    return error.message==='all-render-numbers callback harness budget exceeded'?
      {harness:{kind:'callback-budget',calls:calls.length}}:
      {error:{name:error.name,message:error.message,...(error.name==='PeleError'?{
        code:error.code,type:error.type,line:error.line,column:error.column,snippet:error.snippet,
      }:{})}};
  }};
  const expected=run(c.source),sourceCalls=calls;calls=[];
  const model=parse(c.source),before=graph(model),tasks=model.tasks;
  const modelExpected=run(model),modelCalls=calls;
  const equivalent=JSON.stringify(expected)===JSON.stringify(modelExpected)&&JSON.stringify(sourceCalls)===JSON.stringify(modelCalls);
  process.stdout.write(JSON.stringify({...c,legacyOptions:baseline,expected:{...expected,calls:sourceCalls,counts:{
    clock:sourceCalls.filter(call=>call[0]==='clock').length,
    measurement:sourceCalls.filter(call=>call[0]==='width').length,
  },graphBefore:before,graphAfter:graph(model),sourceModelEquivalent:equivalent,
    ...(!equivalent?{modelObservation:{...modelExpected,calls:modelCalls}}:{}),
    ...(model.type==='gantt'?{schedule:schedule(model),tasksIdentityRetained:tasks===model.tasks}:{})}}));
} else {
  assert.equal(execFileSync('git',['rev-parse','HEAD'],{cwd:upstream,env,encoding:'utf8'}).trim(),revision);
  execFileSync('git',['diff','--quiet',revision,'--','src'],{cwd:upstream,env});
  const oracleSha256=hash(await readFile(repo+'/node_modules/pele/dist/pele.js'));
  assert.equal(oracleSha256,'d1ebc1889f3069d7759fe5db34c4ad4b4823285edbee2b537d0ef3d505a76d8d');
  const provenance={};
  async function visit(dir){for(const entry of (await readdir(upstream+'/'+dir,{withFileTypes:true})).sort((a,b)=>a.name.localeCompare(b.name))){
    const path=dir+'/'+entry.name;
    if(entry.isDirectory())await visit(path);else if(entry.name.endsWith('.ts'))provenance[path]=hash(await readFile(upstream+'/'+path));
  }}
  await visit('src');
  assert.equal(Object.keys(sources).length,33);
  const fields=['fontSize','maxWidth','directionBreakpoint','padding','now'];
  const matrix=[NaN,Infinity,-Infinity,-0,.5,13.25].map(tag), planned=[], cases=[];
  for(const [family,source] of Object.entries(sources)){
    planned.push({id:`baseline/${family}`,family,source,numeric:{}});
    for(const field of fields)for(const [index,value] of matrix.entries())planned.push({
      id:`scalar/${family}/${field}/${index}`,family,source,numeric:{[field]:value},
    });
  }
  for(const [index,c] of planned.entries()){
    try {
      const raw=execFileSync(process.execPath,['--max-old-space-size='+budget.heapMiB,script,'--worker','1','--case',JSON.stringify(c),'--repo',repo,'--sources',sourcesPath],{
        env,encoding:'utf8',timeout:budget.workerTimeoutMs,maxBuffer:2*1024*1024,stdio:['ignore','pipe','pipe'],killSignal:'SIGKILL',
      });
      cases.push(JSON.parse(raw));
    } catch(error){cases.push({...c,legacyOptions:baseline,expected:{harness:{
      kind:error.code==='ETIMEDOUT'?'owned-worker-timeout':'owned-worker-failure',code:error.code??null,
      signal:error.signal??null,status:error.status??null,stderrBytes:error.stderr?.length??0,stderrSha256:hash(error.stderr??''),
    }}})}
    if((index+1)%31===0)process.stderr.write(`oracle families ${(index+1)/31}/33\n`);
  }
  const fixture={revision,oracleSha256,provenance,baseline,sources,sourceCatalogSha256:hash(await readFile(sourcesPath)),
    host:{timezone:'UTC',now:tag(NOW),measure:'UTF16 length * size * (style & 1 ? .71 : .6)'},
    harness:{...budget,childEnvironment:Object.keys(env),noElapsedMeasurement:true},
    scope:{families:33,casesPerFamily:31,fields,matrix,lanes:['source','parsed-model'],
      numberEquality:'Supplied numbers retain exact raw bits. Computed NaNs compare as NaNs, other computed numbers use exact bits. Whole SVG strings and callback order are exact.',
      exclusions:['arbitrary JS coercion','malformed source/SVG UTF16','nonfinite host clock fallback','function source and minifier-generated names'],
      harnessOutcomes:'Timeout/heap/callback/output guards are harness observations and never asserted as Pele errors or parity successes.'},cases};
  const json=JSON.stringify(fixture)+'\n',compressed=gzipSync(json,{level:9});
  await writeFile(output,compressed);
  const summary={families:33,cases:cases.length,svgs:cases.filter(c=>c.expected.result).length,errors:cases.filter(c=>c.expected.error).length,
    harness:cases.filter(c=>c.expected.harness).map(c=>({id:c.id,...c.expected.harness})),
    baselineErrors:cases.filter(c=>c.id.startsWith('baseline/')&&!c.expected.result).map(c=>({id:c.id,expected:c.expected})),
    bytes:Buffer.byteLength(json),gzipBytes:compressed.length,jsonSha256:hash(json),gzipSha256:hash(compressed),output};
  await writeFile(output+'.summary.json',JSON.stringify(summary,null,2)+'\n');
  process.stdout.write(JSON.stringify(summary)+'\n');
}
