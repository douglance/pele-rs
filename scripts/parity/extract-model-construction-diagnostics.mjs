// Pinned development oracle for fresh AgentFlow diagnostic helper failures.
import {readFile,writeFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {resolve} from 'node:path';
import assert from 'node:assert/strict';
const args=process.argv.slice(2),arg=(key,fallback)=>{const i=args.indexOf(key);return i<0?fallback:args[i+1];};
const repo=resolve(arg('--repo',fileURLToPath(new URL('../../',import.meta.url)))),upstream=resolve(arg('--upstream','/tmp/pele-upstream-458322a5'));
const output=resolve(arg('--out',repo+'/fixtures/pele/model-construction-diagnostics.json'));
const revision='458322a5b8abb1ae57ddf127c32017ab9fb14a92',sha=b=>createHash('sha256').update(b).digest('hex'),env={PATH:process.env.PATH??'/usr/bin:/bin',TZ:'UTC',LANG:'C',LC_ALL:'C'};
assert.equal(execFileSync('git',['rev-parse','HEAD'],{cwd:upstream,env,encoding:'utf8'}).trim(),revision);execFileSync('git',['diff','--quiet',revision,'--','src'],{cwd:upstream,env});
const dist=await readFile(repo+'/node_modules/pele/dist/pele.js');assert.equal(sha(dist),'d1ebc1889f3069d7759fe5db34c4ad4b4823285edbee2b537d0ef3d505a76d8d');
const bits=n=>{const b=Buffer.alloc(8);b.writeDoubleBE(n);return b.toString('hex');},units=s=>Array.from({length:s.length},(_,i)=>s.charCodeAt(i));
function graph(root){const ids=new Map(),queue=[],nodes=[],functions=[],prototypes=new Map(),builtin=new Set([Object.prototype,Array.prototype,Map.prototype,Set.prototype,Date.prototype,null]);
 const atom=v=>{if(v===undefined)return['undefined'];if(v===null)return['null'];if(typeof v==='boolean')return['bool',v];if(typeof v==='number')return['number',bits(v)];if(typeof v==='string')return['string',units(v)];if(typeof v==='function'){if(Object.hasOwn(v,'prototype'))void v.prototype;let i=functions.findIndex(f=>f.value===v);if(i<0){i=functions.length;functions.push({value:v,arity:v.length,ownKeys:Reflect.ownKeys(v)});}return['functionObservation',i];}if(!ids.has(v)){ids.set(v,queue.length);queue.push(v);}return['ref',ids.get(v)];};
 const r=atom(root);for(let id=0;id<queue.length;id++){const value=queue[id],proto=Object.getPrototypeOf(value),kind=Array.isArray(value)?'array':value instanceof Map?'map':value instanceof Set?'set':value instanceof Date?'date':builtin.has(proto)?'object':'instance',node={kind};if(kind==='instance'){if(!prototypes.has(proto))prototypes.set(proto,prototypes.size+1);node.prototype=prototypes.get(proto);}if(kind==='array')node.length=value.length;if(kind==='map')node.entries=[...value].map(([k,v])=>[atom(k),atom(v)]);if(kind==='set')node.values=[...value].map(atom);if(kind==='date')node.milliseconds=bits(+value);node.properties=Object.getOwnPropertyNames(value).filter(k=>!(kind==='array'&&k==='length')).map(k=>{const d=Object.getOwnPropertyDescriptor(value,k);assert.ok('value'in d);return[units(k),atom(d.value),(d.enumerable?1:0)|(d.writable?2:0)|(d.configurable?4:0)];});nodes.push(node);}return{root:r,nodes,...(functions.length?{functionObservations:functions.map(({arity,ownKeys})=>({arity,ownKeys}))}:{})};}
function hydrate(input){
 const number=hex=>Buffer.from(hex,'hex').readDoubleBE(),objects=input.nodes.map(n=>n.kind==='array'?new Array(n.length):n.kind==='map'?new Map():n.kind==='set'?new Set():n.kind==='date'?new Date(number(n.milliseconds)):{}),functions=(input.functionObservations??[]).map(()=>function semanticModel(){});
 const atom=a=>a[0]==='undefined'?undefined:a[0]==='null'?null:a[0]==='bool'?a[1]:a[0]==='number'?number(a[1]):a[0]==='string'?String.fromCharCode(...a[1]):a[0]==='ref'?objects[a[1]]:functions[a[1]];
 input.nodes.forEach((n,i)=>{const o=objects[i];if(n.kind==='map')for(const[k,v]of n.entries)o.set(atom(k),atom(v));if(n.kind==='set')for(const v of n.values)o.add(atom(v));for(const[k,v,f]of n.properties)Object.defineProperty(o,String.fromCharCode(...k),{value:atom(v),enumerable:!!(f&1),writable:!!(f&2),configurable:!!(f&4)});});return atom(input.root);
}
const input=await readFile(repo+'/fixtures/pele/diagram-model-construction.json'),base=JSON.parse(input).cases.find(c=>c.id==='agentflow/literal'),entryPath=repo+'/node_modules/pele/dist/esm/diagrams/agentflow.js',entry=(await import(pathToFileURL(entryPath).href)).default,cases=[];
for(const shape of ['circle','madeup','collapsed','containment']){
 const model=hydrate(base.expected.before);
 if(shape==='circle'||shape==='madeup')model.nodes.get('brief').type=shape;
 else model.subgraphs=[{id:'G',type:'flow',nodes:shape==='collapsed'?['brief']:['G'],title:'Group',classes:[],labelType:'text',...(shape==='collapsed'?{metadata:{view:'collapsed'}}:{})}];
 const before=graph(model),calls=[];
 let error;try{entry.render(model,{}, {now:1704067200000,measurer:{width(text,size,style){const width=text.length*size*(style&1?.71:.6);calls.push(['width',units(text),bits(size),style,bits(width)]);return width;}}});}catch(e){error={name:e.name,message:e.message};}
 assert.deepEqual(error,{name:'TypeError',message:({circle:'t.emitError is not a function',madeup:'t.emitWarning is not a function',collapsed:'n.subgraph is not a function',containment:'n.emitWarning is not a function'})[shape]});assert.equal(calls.length,0);
 cases.push({id:'agentflow/diagnostic/'+shape,family:'agentflow',shape,config:{},options:{now:1704067200000},expected:{before,after:graph(model),error,calls}});
}
const provenance={};for(const p of ['src/diagrams/agentflow/graph.ts','src/diagrams/agentflow/shapes.ts'])provenance[p]=sha(await readFile(upstream+'/'+p));provenance['dist/esm/diagrams/agentflow.js']=sha(await readFile(entryPath));
await writeFile(output,JSON.stringify({revision,oracleSha256:sha(dist),inputSha256:sha(input),provenance,scope:'Four fresh ordinary AgentflowModel shape/containment helper reads. Canonical npm TypeError messages and complete unmodified partial graphs; no DB helper injection.',cases},null,2)+'\n');console.log(JSON.stringify({out:output,cases:cases.map(c=>({id:c.id,error:c.expected.error}))}));
