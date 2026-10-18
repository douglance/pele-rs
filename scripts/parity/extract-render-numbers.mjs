// Development-only pinned numeric render oracle; owned bounded worker per input.
import{readFile,writeFile}from'node:fs/promises';
import{createHash}from'node:crypto';
import{execFileSync}from'node:child_process';
import{fileURLToPath,pathToFileURL}from'node:url';
import{resolve}from'node:path';
import assert from'node:assert/strict';
const args=process.argv.slice(2),option=(key,fallback)=>{const i=args.indexOf(key);return i<0?fallback:args[i+1]};
const script=fileURLToPath(import.meta.url),repo=resolve(option('--repo',fileURLToPath(new URL('../../',import.meta.url))));
const upstream=resolve(option('--upstream','/tmp/pele-upstream-458322a5')),output=resolve(option('--out',repo+'/fixtures/pele/render-numbers.json'));
const revision='458322a5b8abb1ae57ddf127c32017ab9fb14a92',NOW=1704067200000;
const sha=b=>createHash('sha256').update(b).digest('hex');
const bits=n=>{const b=Buffer.alloc(8);b.writeDoubleBE(n);return b.toString('hex')};
const number=n=>({kind:'number',bits:bits(n)}),date=n=>({kind:'date',bits:bits(n)}),undefinedValue={kind:'undefined'},nullValue={kind:'null'};
const decode=v=>v.kind==='undefined'?undefined:v.kind==='null'?null:v.kind==='date'?new Date(Buffer.from(v.bits,'hex').readDoubleBE()):Buffer.from(v.bits,'hex').readDoubleBE();
const sources={pie:'pie title Output\n"A":1\n"B":2',flowchart:'flowchart LR\nA[Alpha]-->B[Beta]-->C[Gamma]-->D[Delta]',gantt:'gantt\ndateFormat YYYY-MM-DD\nsection S\nActive:active,1d\nNext:after task1,2d'};
const baseline={fontSize:13,maxWidth:220,directionBreakpoint:80,padding:3,now:NOW+.5};
const fields=['fontSize','maxWidth','directionBreakpoint','padding','now'];
const contracts={numericPacket:'Five independent presence bits and IEEE values. Unmasked fields retain legacy JSON; masked values override only corresponding fields.',date:'now convenience follows actual new Date(input).getTime() before number transport.',boundary:'Upstream now:null coerces0; this runtime-null value is recorded outside selected declared number|Date packet scope.',allocation:'Worker timeout, heap and returned-output guards are oracle harness budgets, never claimed Pele errors.'};
function graph(root){
 const prototype=root&&typeof root==='object'?Object.getPrototypeOf(root):null,ids=new Map(),nodes=[];
 const atom=v=>{if(v===undefined)return['undefined'];if(v===null)return['null'];if(typeof v==='boolean')return['bool',v];if(typeof v==='string')return['string',Array.from({length:v.length},(_,i)=>v.charCodeAt(i))];if(typeof v==='number')return['number',bits(v)];if(typeof v==='function')throw Error('unexpected own function in bounded fixture');if(!ids.has(v)){ids.set(v,ids.size);nodes.push(null)}return['ref',ids.get(v)]};
 const r=atom(root);
 for(const[v,id]of ids){const kind=Array.isArray(v)?'array':v instanceof Map?'map':v instanceof Set?'set':v instanceof Date?'date':prototype!==Object.prototype&&Object.getPrototypeOf(v)===prototype?'instance':'object';const n={kind};if(kind==='instance')n.prototype=1;if(kind==='array')n.length=v.length;if(kind==='map')n.entries=[...v].map(([k,v])=>[atom(k),atom(v)]);if(kind==='set')n.values=[...v].map(atom);if(kind==='date')n.milliseconds=bits(+v);n.properties=Object.getOwnPropertyNames(v).filter(k=>!(kind==='array'&&k==='length')).map(k=>{const d=Object.getOwnPropertyDescriptor(v,k);assert.ok('value'in d);return[Array.from({length:k.length},(_,i)=>k.charCodeAt(i)),atom(d.value),(d.enumerable?1:0)|(d.writable?2:0)|(d.configurable?4:0)]});nodes[id]=n}
 return{root:r,nodes};
}
function schedule(model){if(model.type!=='gantt')return undefined;return{nowBits:bits(model.now),workBits:bits(model.work),excludedDays:model.excludedDays,datedTokens:model.datedTokens,tasks:model.tasks.map((task,i)=>({id:task.id,processed:task.processed,start:task.startTime===undefined?{kind:'undefined'}:number(+task.startTime),end:task.endTime===undefined?{kind:'undefined'}:number(+task.endTime),renderEnd:task.renderEndTime===null?{kind:'null'}:number(+task.renderEndTime),byIdSame:model.byId.get(task.id)===task,previousEndSame:i>0&&task.startTime===model.tasks[i-1].endTime}))}}
if(option('--worker',false)){
 process.env.TZ='UTC';const c=JSON.parse(option('--case','{}'));const{parse,render}=await import(pathToFileURL(repo+'/node_modules/pele/dist/pele.js').href);
 const model=parse(sources[c.family]),before=graph(model),tasks=model.tasks,calls=[];
 const hostValue=c.hostNow?decode(c.hostNow):NOW;
 Date.now=()=>{calls.push(['clock',c.hostThrows?'throw':bits(hostValue)]);if(c.hostThrows)throw Error('render-numbers host clock stopped');return hostValue};
 const options={...baseline};const numeric={};
 for(const[field,tag]of Object.entries(c.values??{})){
  const value=decode(tag);options[field]=value;
  if(tag.kind==='number'||tag.kind==='date')numeric[field]=number(tag.kind==='date'?+value:value);
 }
 const legacy={...baseline};for(const[field,tag]of Object.entries(c.values??{})){if(tag.kind==='undefined')delete legacy[field];else if(tag.kind==='null')legacy[field]=null}
 const measurer={width(text,size,style){const width=text.length*size*(style&1?.71:.6);calls.push(['width',Array.from({length:text.length},(_,i)=>text.charCodeAt(i)),bits(size),style,bits(width)]);if(calls.length>2000)throw Error('oracle callback work budget exceeded');return width}};
 let expected;
 try{const result=render(model,{...options,measurer});assert.ok(result.svg.isWellFormed());if(Buffer.byteLength(result.svg)>256*1024)expected={unsupported:{kind:'returned-output-harness-budget',bytes:Buffer.byteLength(result.svg),sha256:sha(result.svg)}};else expected={result,widthBits:bits(result.width),heightBits:bits(result.height)}}
 catch(error){expected={error:{name:error.name,message:error.message,...(error.name==='PeleError'?{code:error.code,type:error.type,line:error.line,column:error.column,snippet:error.snippet}:{})}}}
 if(expected.error?.message==='oracle callback work budget exceeded')expected={unsupported:{kind:'callback-harness-budget',calls:calls.length}};
 const result={...c,source:sources[c.family],legacyOptions:legacy,numeric,...(c.values?.now?.kind==='null'?{boundary:'runtime-null-now-outside-declared-packet'}:{}),expected:{...expected,calls,counts:{clock:calls.filter(c=>c[0]==='clock').length,measurement:calls.filter(c=>c[0]==='width').length},graphBefore:before,graphAfter:graph(model),...(model.type==='gantt'?{schedule:schedule(model),tasksIdentityRetained:model.tasks===tasks}:{})}};
 process.stdout.write(JSON.stringify(result));
}else{
 assert.equal(execFileSync('git',['rev-parse','HEAD'],{cwd:upstream,encoding:'utf8'}).trim(),revision);execFileSync('git',['diff','--quiet',revision,'--','src'],{cwd:upstream});
 const provenance={};for(const p of['src/core.ts','src/types.ts','src/diagrams/common/fit-width.ts','src/diagrams/pie/render.ts','src/diagrams/flowchart/render.ts','src/diagrams/gantt/render.ts','src/diagrams/gantt/db.ts','src/text/label.ts'])provenance[p]=sha(await readFile(upstream+'/'+p));
 const oracleSha256=sha(await readFile(repo+'/node_modules/pele/dist/pele.js'));assert.equal(oracleSha256,'d1ebc1889f3069d7759fe5db34c4ad4b4823285edbee2b537d0ef3d505a76d8d');
 const matrix=[undefinedValue,nullValue,...[NaN,Infinity,-Infinity,0,-0,-1,-.5,.5,13.25,240.5].map(number),{kind:'number',bits:'fff8000000000042'}],planned=[];
 for(const family of Object.keys(sources))planned.push({id:`legacy/${family}`,family,values:{}});
 for(const family of Object.keys(sources))for(const field of fields)for(const[index,value]of matrix.entries())planned.push({id:`matrix/${family}/${field}/${index}`,family,values:{[field]:value}});
 for(const family of Object.keys(sources)){
  planned.push({id:`joint-finite/${family}`,family,values:{fontSize:number(13.25),maxWidth:number(140.5),directionBreakpoint:number(240.25),padding:number(.5),now:number(NOW+.875)}});
  planned.push({id:`joint-nonfinite/${family}`,family,values:{fontSize:number(NaN),maxWidth:number(Infinity),directionBreakpoint:number(-Infinity),padding:number(-0),now:number(NaN)}});
 }
 for(const value of[80,220,220.5,640])planned.push({id:`flow-direction/${value}`,family:'flowchart',values:{directionBreakpoint:number(value)}});
 for(const family of Object.keys(sources))for(const value of[80,140,220,640])planned.push({id:`fit-width/${family}/${value}`,family,values:{maxWidth:number(value)}});
 for(const family of Object.keys(sources))for(const[index,value]of[NOW+.875,NOW-.875,-.875,.875,NaN,Infinity,-Infinity,8640000000000001,-8640000000000001].entries()){
  const clipped=+new Date(value);planned.push({id:`date/${family}/${index}`,family,values:{now:date(value)},equivalent:`date-number/${family}/${index}`});planned.push({id:`date-number/${family}/${index}`,family,values:{now:number(clipped)},equivalent:`date/${family}/${index}`});
 }
 for(const[index,value]of[undefinedValue,number(NaN),number(Infinity),date(NaN)].entries()){
  planned.push({id:`fractional-host-clock/${index}`,family:'gantt',values:{now:value},hostNow:number(NOW+.625)});
  planned.push({id:`throwing-host-clock/${index}`,family:'gantt',values:{now:value},hostThrows:true});
 }
 planned.push({id:'finite-now-no-host-clock',family:'gantt',values:{now:number(NOW+.875)},hostThrows:true},{id:'finite-date-no-host-clock',family:'gantt',values:{now:date(NOW+.875)},hostThrows:true});
 for(const family of['pie','flowchart'])for(const[index,value]of[undefinedValue,number(NaN),date(NaN)].entries())planned.push({id:`ignored-clock/${family}/${index}`,family,values:{now:value},hostThrows:true});
 const env={PATH:process.env.PATH??'/usr/bin:/bin',TZ:'UTC',LANG:'C',LC_ALL:'C'},cases=[];
 for(const c of planned){
  try{const raw=execFileSync(process.execPath,['--max-old-space-size=192',script,'--worker','1','--case',JSON.stringify(c),'--repo',repo],{env,encoding:'utf8',timeout:2000,maxBuffer:2*1024*1024,stdio:['ignore','pipe','pipe']});cases.push(JSON.parse(raw))}
  catch(error){cases.push({...c,source:sources[c.family],unsupported:{kind:error.code==='ETIMEDOUT'?'owned-worker-timeout':'owned-worker-failure',code:error.code??null,signal:error.signal??null,status:error.status??null,stderrBytes:error.stderr?.length??0,stderrSha256:sha(error.stderr??'')}})}
 }
 const pairs=[];for(const c of cases.filter(c=>c.id.startsWith('date/'))){const other=cases.find(v=>v.id===c.equivalent);if(!c.expected||!other?.expected){pairs.push({left:c.id,right:c.equivalent,compared:false});continue}assert.deepEqual(c.expected,other.expected,`Date vs TimeClip numeric ${c.id}`);pairs.push({left:c.id,right:c.equivalent,compared:true})}
 const fixture={revision,oracleSha256,provenance,contracts,baseline,host:{measure:'fixedUTF16',timezone:'UTC',now:number(NOW)},harness:{workerTimeoutMs:2000,heapMiB:192,outputBytes:256*1024,callbackWork:2000,childEnvironment:['PATH','TZ','LANG','LC_ALL'],noElapsedMeasurement:true},scope:'Bounded well-formed source/SVG cases. No arbitrary object coercion; numbers outside Date range are tested only through Date→NaN, never as giant finite raw now. Nonfinite geometry may intentionally produce SVG coordinates unsuitable for display; exact returned strings remain the oracle.',pairs,cases};
 await writeFile(output,JSON.stringify(fixture,null,2)+'\n');console.log(JSON.stringify({cases:cases.length,svg:cases.filter(c=>c.expected?.result).length,errors:cases.filter(c=>c.expected?.error).length,unsupported:cases.filter(c=>c.unsupported||c.expected?.unsupported).map(c=>({id:c.id,reason:c.unsupported??c.expected.unsupported})),boundaries:cases.filter(c=>c.boundary).length,datePairs:pairs.length,output,sha256:sha(JSON.stringify(fixture,null,2)+'\n')}));
}
