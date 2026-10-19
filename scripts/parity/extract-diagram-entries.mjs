// Development-only pinned Pele low-level entry oracle. No JS parser/renderer ships.
import {readFile,writeFile} from 'node:fs/promises';
import {gunzipSync} from 'node:zlib';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {resolve} from 'node:path';
import assert from 'node:assert/strict';
process.env.TZ='UTC';
const args=process.argv.slice(2),arg=(key,fallback)=>{const i=args.indexOf(key);return i<0?fallback:args[i+1];};
const repo=resolve(arg('--repo',fileURLToPath(new URL('../../',import.meta.url)))),upstream=resolve(arg('--upstream','/tmp/pele-upstream-458322a5'));
const output=resolve(arg('--out',repo+'/fixtures/pele/diagram-entries.json'));
const revision='458322a5b8abb1ae57ddf127c32017ab9fb14a92',sha=b=>createHash('sha256').update(b).digest('hex');
const env={PATH:process.env.PATH??'/usr/bin:/bin',TZ:'UTC',LANG:'C',LC_ALL:'C'};
assert.equal(execFileSync('git',['rev-parse','HEAD'],{cwd:upstream,env,encoding:'utf8'}).trim(),revision);
execFileSync('git',['diff','--quiet',revision,'--','src'],{cwd:upstream,env});
const dist=await readFile(repo+'/node_modules/pele/dist/pele.js');assert.equal(sha(dist),'d1ebc1889f3069d7759fe5db34c4ad4b4823285edbee2b537d0ef3d505a76d8d');
const numericPath=repo+'/fixtures/pele/all-render-numbers.json';let numeric;try{numeric=await readFile(numericPath);}catch{numeric=gunzipSync(await readFile(numericPath+'.gz'));}
const baselines=JSON.parse(numeric).cases.filter(c=>c.id.startsWith('baseline/'));
const bits=n=>{const b=Buffer.alloc(8);b.writeDoubleBE(n);return b.toString('hex');},units=s=>Array.from({length:s.length},(_,i)=>s.charCodeAt(i));
function graph(root){const ids=new Map(),queue=[],nodes=[],functions=[],prototypes=new Map(),builtin=new Set([Object.prototype,Array.prototype,Map.prototype,Set.prototype,Date.prototype,null]);
 const atom=v=>{if(v===undefined)return['undefined'];if(v===null)return['null'];if(typeof v==='boolean')return['bool',v];if(typeof v==='number')return['number',bits(v)];if(typeof v==='string')return['string',units(v)];if(typeof v==='function'){if(Object.hasOwn(v,'prototype'))void v.prototype;let i=functions.findIndex(f=>f.value===v);if(i<0){i=functions.length;functions.push({value:v,arity:v.length,ownKeys:Reflect.ownKeys(v)});}return['functionObservation',i];}if(!ids.has(v)){ids.set(v,queue.length);queue.push(v);}return['ref',ids.get(v)];};
 const r=atom(root);for(let id=0;id<queue.length;id++){const value=queue[id],proto=Object.getPrototypeOf(value),kind=Array.isArray(value)?'array':value instanceof Map?'map':value instanceof Set?'set':value instanceof Date?'date':builtin.has(proto)?'object':'instance',node={kind};if(kind==='instance'){if(!prototypes.has(proto))prototypes.set(proto,prototypes.size+1);node.prototype=prototypes.get(proto);}if(kind==='array')node.length=value.length;if(kind==='map')node.entries=[...value].map(([k,v])=>[atom(k),atom(v)]);if(kind==='set')node.values=[...value].map(atom);if(kind==='date')node.milliseconds=bits(+value);node.properties=Object.getOwnPropertyNames(value).filter(k=>!(kind==='array'&&k==='length')).map(k=>{const d=Object.getOwnPropertyDescriptor(value,k);assert.ok('value'in d);return[units(k),atom(d.value),(d.enumerable?1:0)|(d.writable?2:0)|(d.configurable?4:0)];});nodes.push(node);}return{root:r,nodes,...(functions.length?{functionObservations:functions.map(({arity,ownKeys})=>({arity,ownKeys}))}:{})};}
const now=1704067200000;Date.now=()=>now;const cases=[],entries=[],provenance={};
for(const p of ['src/core.ts','src/types.ts','src/text/icons.ts','src/preprocess.ts'])provenance[p]=sha(await readFile(upstream+'/'+p));
const undefinedTag={kind:'undefined'},number=n=>({kind:'number',bits:bits(n)}),decode=v=>v.kind==='undefined'?undefined:v.kind==='null'?null:Buffer.from(v.bits,'hex').readDoubleBE();
for(const base of baselines){const family=base.family,path=repo+'/node_modules/pele/dist/esm/diagrams/'+family+'.js',entry=(await import(pathToFileURL(path).href)).default;
 provenance['dist/esm/diagrams/'+family+'.js']=sha(await readFile(path));
 const metadata={type:entry.type,...(Object.hasOwn(entry,'section')?{section:entry.section}:{}),...(Object.hasOwn(entry,'keepComments')?{keepComments:entry.keepComments}:{}),ownKeys:Object.keys(entry),parseArity:entry.parse.length,renderArity:entry.render.length};entries.push(metadata);
 const source=base.source.replace(/^---\n[\s\S]*?\n---\n/,'')+'\n';
 for(const variant of ['baseline','prepared-context','edited']){
  const config=variant==='baseline'?{}:{wrap:true,flowchart:{inheritDir:true},[entry.section??family]:{useMaxWidth:false}};
  const title=variant==='baseline'?undefined:'Prepared '+family,lineOffset=variant==='baseline'?0:7,maxEdges=undefinedTag;
  const c={id:family+'/'+variant,family,source,config,title:title??null,lineOffset,maxEdges,edit:variant==='edited'?{title:'Edited '+family}:null,options:{now,...(variant==='baseline'?{}:{outputLimit:1,limit:-1,maxEdges:-1,maxWidth:320})}};
  const model=entry.parse(source,config,title,lineOffset,{maxEdges:decode(maxEdges)});const before=graph(model);if(c.edit)Object.assign(model,c.edit);const calls=[];
  const options={...c.options,measurer:{width(text,size,style){if(calls.length>=2000)throw Error('Oracle callback harness budget');const width=text.length*size*(style&1?.71:.6);calls.push(['width',units(text),bits(size),style,bits(width)]);return width;}},icons:name=>{calls.push(['icon',units(name)]);return'<path d="M0,0"/>';}};
  let expected;try{const result=entry.render(model,config,options);assert.ok(result.svg.length<1_000_000);expected={before,after:graph(model),result,widthBits:bits(result.width),heightBits:bits(result.height),calls};}catch(e){expected={before,after:graph(model),error:{name:e.name,message:e.message,...(e.name==='PeleError'?{code:e.code,type:e.type,line:e.line,column:e.column,snippet:e.snippet}:{})},calls};}
  cases.push({...c,expected});
 }
}
const core=await import(pathToFileURL(repo+'/node_modules/pele/dist/esm/core.js').href);
const coreIconCases=[];
for(const [family,source] of [
 ['architecture','architecture-beta\nservice app(server)[App]\nservice db(server)[Data]\napp:R --> L:db\n'],
 ['flowchart','flowchart LR\nA@{icon: fa:fa-user, label: \"fas:fa-user First\"} --> B@{icon: fa:fa-user, label: \"fas:fa-user Second\"}\n'],
 ['usecase','usecase-beta\nactor Customer(\"fas:fa-user Customer\")\nCheckout(\"fas:fa-user Work\")\nCustomer --> Checkout\n']
]){
 const entry=(await import(pathToFileURL(repo+'/node_modules/pele/dist/esm/diagrams/'+family+'.js').href)).default;core.register(entry);const calls=[];
 const options={now,measurer:{width(text,size,style){const width=text.length*size*(style&1?.71:.6);calls.push(['width',units(text),bits(size),style,bits(width)]);return width;}},icons(name){calls.push(['icon',units(name)]);return'<path d=\"M0,0\"/>';}};
 const result=core.render(source,options);coreIconCases.push({id:family+'/direct-and-inline-icons',family,source,options:{now},expected:{result,calls}});
}
assert.equal(entries.length,33);const fixture={revision,oracleSha256:sha(dist),provenance,host:{measure:'fixedUTF16',now,timezone:'UTC'},scope:'Prepared source passed verbatim; finite nonnegative integer lineOffset. Real returned mutable models; arbitrary plain Model construction remains a separate capability.',entries,cases,coreIconCases};await writeFile(output,JSON.stringify(fixture,null,2)+'\n');console.log(JSON.stringify({entries:entries.length,cases:cases.length,errors:cases.filter(c=>c.expected.error).map(c=>({id:c.id,error:c.expected.error})),out:output}));
