// Development-only pinned Pele numeric edge-limit oracle; no JS runtime ships.
import {parse,render} from 'pele';
import {readFile,writeFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import assert from 'node:assert/strict';
process.env.TZ='UTC';
const root=process.argv[2]??'/tmp/pele-upstream-458322a5';
const revision='458322a5b8abb1ae57ddf127c32017ab9fb14a92';
assert.equal(execFileSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).trim(),revision);
execFileSync('git',['diff','--quiet',revision,'--','src'],{cwd:root});
const sha=b=>createHash('sha256').update(b).digest('hex');
const provenance={};
for(const path of ['src/core.ts','src/errors.ts','src/diagrams/flowchart/db.ts','src/diagrams/flowchart/index.ts','src/diagrams/agentflow/db.ts','src/diagrams/agentflow/index.ts','src/diagrams/swimlane/index.ts'])provenance[path]=sha(await readFile(root+'/'+path));
const oracleSha256=sha(await readFile('node_modules/pele/dist/pele.js'));
assert.equal(oracleSha256,'d1ebc1889f3069d7759fe5db34c4ad4b4823285edbee2b537d0ef3d505a76d8d');
const now=1704067200000;Date.now=()=>now;
const host={measure:'fixed',now,timezone:'UTC'};
const options={limit:0,maxEdges:0,config:{retained:{flag:'kept'},flowchart:{inheritDir:true}}};
const bits=n=>{const b=Buffer.alloc(8);b.writeDoubleBE(n);return b.toString('hex')};
const number=n=>({kind:'number',bits:bits(n)}),undefinedValue={kind:'undefined'},nullValue={kind:'null'};
const decode=v=>v.kind==='undefined'?undefined:v.kind==='null'?null:Buffer.from(v.bits,'hex').readDoubleBE();
const tagged=v=>v===undefined?undefinedValue:v===null?nullValue:number(v);
const privateFields=new Set(['subgraphLookup','globalNodes','members','edgeById','pairCount','subCount','bareMappings','firstMapping','reported','options']);
const typed=model=>JSON.parse(JSON.stringify(model,(k,v)=>privateFields.has(k)?undefined:v instanceof Map?[...v]:v));
const error=e=>{assert.equal(e.name,'PeleError');return{name:e.name,code:e.code,message:e.message,type:e.type,line:e.line,column:e.column,snippet:e.snippet}};
function graph(root){
  const proto=root&&typeof root==='object'?Object.getPrototypeOf(root):null;
  const ids=new Map(),nodes=[];
  const atom=v=>{
    if(v===undefined)return['undefined'];if(v===null)return['null'];
    if(typeof v==='boolean')return['bool',v];if(typeof v==='string')return['string',Array.from({length:v.length},(_,i)=>v.charCodeAt(i))];
    if(typeof v==='number')return['number',bits(v)];
    assert.notEqual(typeof v,'function','no function-valued own fields in this feature corpus');
    if(!ids.has(v)){ids.set(v,ids.size);nodes.push(null)}return['ref',ids.get(v)];
  };
  const r=atom(root);
  for(const[v,id]of ids){
    const kind=Array.isArray(v)?'array':v instanceof Map?'map':v instanceof Set?'set':proto!==Object.prototype&&Object.getPrototypeOf(v)===proto?'instance':'object';
    const n={kind};if(kind==='instance')n.prototype=1;if(kind==='array')n.length=v.length;
    if(kind==='map')n.entries=[...v].map(([k,v])=>[atom(k),atom(v)]);
    if(kind==='set')n.values=[...v].map(atom);
    n.properties=Object.getOwnPropertyNames(v).filter(k=>!(kind==='array'&&k==='length')).map(k=>{const d=Object.getOwnPropertyDescriptor(v,k);assert.ok('value'in d);return[Array.from({length:k.length},(_,i)=>k.charCodeAt(i)),atom(d.value),(d.enumerable?1:0)|(d.writable?2:0)|(d.configurable?4:0)]});
    nodes[id]=n;
  }
  return{root:r,nodes};
}
function parseOptions(limits,base=options){return{...base,...('source'in limits?{limit:decode(limits.source)}:{}),...('maxEdges'in limits?{maxEdges:decode(limits.maxEdges)}:{})}}
function drawing(model){
  const calls=[];const measurer={width(text,size,style){calls.push([Array.from({length:text.length},(_,i)=>text.charCodeAt(i)),bits(size),style]);return text.length*size*(style&1?.71:.6)}};
  const result=render(model,{now,measurer});assert.ok(result.svg.isWellFormed());
  return{result,widthBits:bits(result.width),heightBits:bits(result.height),calls};
}
const cases=[];
function add(id,source,limits,{parseOnly=false,base=options}={}){
  let expected;
  try{
    const model=parse(source,parseOptions(limits,base));
    expected={type:source.trimStart().startsWith('swimlane')?'swimlane':model.type,model:typed(model),...(!parseOnly?{graph:graph(model),...drawing(model)}:{edgeCount:model.edges.length}),...(model.options&&Object.hasOwn(model.options,'maxEdges')?{maxEdges:tagged(model.options.maxEdges)}:{})};
  }catch(e){expected={error:error(e)}}
  cases.push({id,source,limits,...(parseOnly?{parseOnly:true}:{}),...(base!==options?{options:base}:{}),expected});
}
const values=[undefinedValue,nullValue,...[NaN,Infinity,-Infinity,0,-0,-1,-.5,.5,1,1.5,2,2.5,3,3.5,4,Number.MIN_VALUE,-Number.MIN_VALUE,-1e21,Number.MAX_VALUE].map(number),{kind:'number',bits:'fff8000000000042'}];
const families=[['flowchart','flowchart TB'],['agentflow','agentflow-beta TB'],['swimlane','swimlane-beta TB']];
for(const[family,head]of families){
  for(const[name,body]of[['node','A["😀"]'],['chain','A-->B-->C-->D'],['cartesian','A & B --> C & D'],['malformed','A-->B\n@']])for(const[index,maxEdges]of values.entries())add(`${family}/${name}/${index}`,head+'\n'+body,{source:undefinedValue,maxEdges});
  const source=head+'\nA-->B';
  for(const[name,limits]of[
    ['both-legacy',{}],['edge-only',{maxEdges:number(Infinity)}],['source-only',{source:undefinedValue}],
    ['source-null',{source:nullValue,maxEdges:number(Infinity)}],['source-fraction',{source:number(source.length-.5),maxEdges:nullValue}],
    ['source-nan-edge-null',{source:number(NaN),maxEdges:nullValue}],['source-exact-edge-one',{source:number(source.length),maxEdges:number(1)}],
  ])add(`${family}/composition/${name}`,source,limits);
  for(const count of[5000,5001])add(`${family}/default-boundary/${count}`,head+'\n'+'A-->B\n'.repeat(count),{source:undefinedValue,maxEdges:undefinedValue},{parseOnly:true,base:{...options,config:{flowchart:{maxEdges:9000},maxEdges:9000}}});
}
for(const[index,maxEdges]of values.entries())add(`info/ignored/${index}`,'info title Edge policy',{source:undefinedValue,maxEdges});
const methods=[];
for(const[family,head]of families){
  const source=head+'\nA\nB\nC\nD',limits={source:undefinedValue,maxEdges:number(2.5)};
  const m=parse(source,parseOptions(limits)),initial=graph(m),edges=m.edges,steps=[];
  const operations=[
    {method:'addLink',args:[['A','B'],['C','D'],{type:'arrow_point',stroke:'normal',length:1,id:'pair@'}]},
    {setMaxEdges:number(Infinity)},
    {method:'addSingleLink',args:['B','D']},
    {method:'addSingleLink',args:['A','C']},
    {setMaxEdges:nullValue},
    {method:'addLink',args:[['A'],['B']]},
    {setMaxEdges:undefinedValue},
    {method:'addLink',args:[['A'],['B']]},
  ];
  for(const op of operations){
    let outcome;
    try{if(op.method){const returned=m[op.method](...op.args);assert.equal(returned,undefined)}else m.options.maxEdges=decode(op.setMaxEdges);outcome={returned:['undefined']}}
    catch(e){outcome={error:error(e)}}
    steps.push({op,...outcome,graph:graph(m),edgesIdentityRetained:m.edges===edges,edges:m.edges.map(e=>({id:e.id,start:e.start,end:e.end,edgeByIdSame:m.edgeById.get(e.id)===e}))});
  }
  methods.push({id:family,source,limits,initial,steps,expected:drawing(m)});
}
await writeFile('fixtures/pele/edge-limit.json',JSON.stringify({revision,oracleSha256,provenance,options,host,scope:'Exact numeric edge thresholds and complete reachable own-property graphs; bounded UTF8/well-formed inputs. Independent native one-million-edge ceiling is an extension tested separately, never a claimed Pele error. Typed projection retains the published model slice; graph retains every private own field/alias/IEEE value.',cases,methods},null,2)+'\n');
console.log(`${cases.length} edge-limit cases: ${cases.filter(c=>c.expected.result).length} SVGs, ${cases.filter(c=>c.expected.error).length} errors, ${cases.filter(c=>c.parseOnly).length} parse-only boundaries; ${methods.length} method sequences/${methods.reduce((n,m)=>n+m.steps.length,0)} steps`);
