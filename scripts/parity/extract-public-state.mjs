// Development-only pinned oracle. No JS diagram logic ships in consumers.
// Regenerates public_state.rs literal full-graph/method/semantic/cached SVG vectors.
import {parse,render} from 'pele';
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';import{writeFileSync,readFileSync}from'node:fs';
const root=process.argv[2]??'/tmp/pele-upstream-458322a5';
const revision='458322a5b8abb1ae57ddf127c32017ab9fb14a92';
assert.equal(execFileSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).trim(),revision);
execFileSync('git',['diff','--quiet',revision,'--','src'],{cwd:root});
const sha=data=>createHash('sha256').update(data).digest('hex');
for(const [path,expected]of [['src/diagrams/state/index.ts', '83616f8ef2ce5ddf706440a3e536730c618d15e015b20f6e92ff82272b9f6d54'], ['src/diagrams/state/db.ts', '19479aa5df9a37df4439e917103962bec4e79c4cf0b909ecf2dc5a44dd626a10'], ['src/diagrams/state/graph.ts', 'f1fc3b848b7b1aef11797da8b01318343d38e1e998f9982e490c553848f36d28'], ['src/diagrams/state/parser.ts', 'f03eac6f13a60a98316a58a304f1332cef13c7a01de58ee861e0f91baab472c6'], ['src/diagrams/state/render.ts', '2980d6c6cbefcb9bd6a84d01d6049efdd2a987236a4806d31e79c38e0ab704a1']])assert.equal(sha(readFileSync(`${root}/${path}`)),expected,path);
const bundle=fileURLToPath(new URL('../../node_modules/pele/dist/pele.js',import.meta.url));
assert.equal(sha(readFileSync(bundle)),'d1ebc1889f3069d7759fe5db34c4ad4b4823285edbee2b537d0ef3d505a76d8d');
// Constructor minifier names and engine-generated TypeError wording are not claimed.
// Arrays carry implicit length separately; object descriptors and every private
// cache/Map/Set reference are canonicalized without losing UTF-16 or IEEE numbers.
const flowProto=Object.getPrototypeOf(parse('stateDiagram-v2'));
const source='stateDiagram-v2\naccTitle: Access\naccDescr: Details\ndirection LR\nstate Group {\n[*] --> A\nA --> B: Transition\n}\nA --> C\nclassDef blue fill:#247acd,color:#ffffff\nclass A blue\nstyle B stroke:#ff0000\nnote right of C: Note\nclick A \"https://example.com\" \"Tooltip\"\n';
function graph(root){const ids=new Map(),nodes=[];const atom=v=>{if(v===undefined)return['undefined'];if(v===null)return['null'];if(typeof v==='boolean')return['bool',v];if(typeof v==='string')return['string',Array.from({length:v.length},(_,i)=>v.charCodeAt(i))];if(typeof v==='number'){const d=new DataView(new ArrayBuffer(8));d.setFloat64(0,v);return['number',d.getBigUint64(0).toString(16).padStart(16,'0')]}if(typeof v==='function')throw Error('unaccounted function');if(!ids.has(v)){ids.set(v,ids.size);nodes.push(null)}return['ref',ids.get(v)]};const r=atom(root);for(const [v,id]of ids){const kind=Array.isArray(v)?'array':v instanceof Map?'map':v instanceof Set?'set':Object.getPrototypeOf(v)===flowProto?'instance':'object';const n={kind};if(kind==='instance')n.prototype=1;if(kind==='array')n.length=v.length;if(kind==='map')n.entries=[...v].map(([k,v])=>[atom(k),atom(v)]);if(kind==='set')n.values=[...v].map(atom);n.properties=Object.getOwnPropertyNames(v).filter(k=>!(kind==='array'&&k==='length'&&Object.getOwnPropertyDescriptor(v,k).writable)).map(k=>{const d=Object.getOwnPropertyDescriptor(v,k);return[Array.from({length:k.length},(_,i)=>k.charCodeAt(i)),atom(d.value),(d.enumerable?1:0)|(d.writable?2:0)|(d.configurable?4:0)]});nodes[id]=n;}return{root:r,nodes}}
function at(m,path){return path.reduce((v,key)=>typeof key==='object'?v.get(key.map):v[key],m)}
function value(m,v){if(v&&typeof v==='object'){if('record'in v)return Object.fromEntries(v.record.map(([k,v])=>[k,value(m,v)]));if('ref'in v)return at(m,v.ref);if('undefined'in v)return undefined;if('utf16'in v)return String.fromCharCode(...v.utf16);if('number'in v)return({nan:NaN,inf:Infinity,'-inf':-Infinity,'-0':-0})[v.number];if(Array.isArray(v))return v.map(x=>value(m,x));return Object.fromEntries(Object.entries(v).map(([k,v])=>[k,value(m,v)]))}return v}
const cases=[];function add(id,{src=source,options={},ops=[],svg=true}={}){const m=parse(src,options),steps=[];const initial=graph(m);for(const op of ops){let result,error;try{if(op.method){result=op.method==='get direction'?Object.getOwnPropertyDescriptor(flowProto,'direction').get.call(m):m[op.method](...(op.args??[]).map(v=>value(m,v)));if(op.store)m[op.store]=result;}else if(op.set){const target=at(m,op.set.slice(0,-1));target[op.set.at(-1)]=value(m,op.value)}else if(op.reverse)at(m,op.reverse).reverse();else if(op.delete){const target=at(m,op.delete.slice(0,-1));delete target[op.delete.at(-1)]}else if(op.mapDelete){const path=op.mapDelete.slice(0,-1);at(m,path).delete(op.mapDelete.at(-1))}else if(op.mapSet){at(m,op.mapSet).set(value(m,op.key),value(m,op.value))}else if(op.readonly){Object.defineProperty(at(m,op.readonly.slice(0,-1)),op.readonly.at(-1),{writable:false})}else throw Error('op');}catch(e){error={name:e.name,...(e.name==='PeleError'?{message:e.message,code:e.code,type:e.type,line:e.line,column:e.column}:{})}}steps.push({op,graph:graph(m),...(error?{error}:{result:graph(result)})})}const outputs=[];if(svg)for(const maxWidth of [undefined,240]){try{const r=render(m,{maxWidth});outputs.push({maxWidth:maxWidth??null,width:r.width,height:r.height,svgSha256:createHash('sha256').update(r.svg).digest('hex'),links:r.links})}catch(e){outputs.push({maxWidth:maxWidth??null,error:{name:e.name,...(e.name==='PeleError'?{message:e.message,code:e.code,type:e.type,line:e.line,column:e.column}:{})}})}}cases.push({id,source:src,options,initial,steps,outputs})}
const S=(id,type='default',description='')=>({record:[['stmt','state'],['id',id],['type',type],['description',description]]});
const R=(one,two,text)=>({record:[['stmt','relation'],['state1',one],['state2',two],...text===undefined?[]:[['description',text]]]});
const D=value=>({record:[['stmt','dir'],['value',value]]});
add('initial');
for(const src of ['stateDiagram','stateDiagram-v2\nA','stateDiagram-v2\nstate "Display" as A\nA: second\nA --> B: move','stateDiagram-v2\nstate F <<fork>>\nF --> C\nstate C <<choice>>','stateDiagram-v2\nstate Composite {\ndirection RL\n[*] --> A\n--\nB --> [*]\n}\n','stateDiagram-v2\nstate Composite {\nA\n--\n}\n','stateDiagram-v2\nA:::blue --> B\nclassDef blue color:red,fill:blue;\nclass A blue\nstyle B fill:green,stroke:black;','stateDiagram-v2\nA\nnote left of A: One\nnote right of A: Two','stateDiagram-v2\nstate A {\nstate B {\nC\n}\n}\nA --> B','---\ntitle: State title\n---\nstateDiagram-v2\ndirection LR\nA --> B','stateDiagram-v2\nhide empty description\nscale 20 width\nstate bare\naccTitle: A title\naccDescr: A description'])add('source-'+cases.length,{src});
add('direction',{src:'stateDiagram-v2\ndirection LR\nA --> B\ndirection RL',ops:[{method:'get direction'},{method:'getDirection'},{method:'setDirection',args:['TB']},{method:'get direction'},{method:'getDirection'}]});
add('new-direction',{src:'stateDiagram-v2\nA',ops:[{method:'setDirection',args:['LR']},{method:'get direction'}]});
add('direction-nullish',{src:'stateDiagram-v2\ndirection LR\nA',ops:[{set:['rootDoc',0,'value'],value:{undefined:true}},{method:'get direction'},{method:'getDirection'}]});
add('root-edit-stale',{src:'stateDiagram-v2\nA: Before\nA --> B',ops:[{set:['rootDoc',0,'description'],value:'After'},{set:['rootDoc',1,'description'],value:'Changed edge'}]});
add('root-edit-extract',{src:'stateDiagram-v2\nA: Before\nA --> B',ops:[{set:['rootDoc',0,'description'],value:'After'},{set:['rootDoc',1,'description'],value:'Changed edge'},{method:'extract',args:[{ref:['rootDoc']}]}]});
add('set-doc',{ops:[{set:['newDoc'],value:[D('RL'),S('X'),R(S('[*]'),S('X')),R(S('X'),S('[*]'),'Finish')]},{method:'setRootDoc',args:[{ref:['newDoc']}]}]});
add('extract-other-doc',{ops:[{set:['other'],value:[S('X'),R(S('X'),S('Y'),'Next')]},{method:'extract',args:[{record:[['doc',{ref:['other']}]]}]},{method:'get direction'}]});
add('translate-only',{ops:[{set:['rootDoc'],value:[R(S('[*]'),S('New'))]},{method:'translate'},{method:'getState',args:['New']}]});
add('translate-top-dividers',{ops:[{set:['rootDoc'],value:[S('A'),S('divider-id-custom','divider'),S('B')]},{method:'translate'},{method:'getDividerId'},{method:'setRootDoc',args:[{ref:['rootDoc']}]}]});
add('divider-counter',{src:'stateDiagram-v2\nstate X {\nA\n--\nB\n--\nC\n}\n',ops:[{method:'getDividerId'},{method:'getDividerId'}]});
add('add-state',{ops:[{method:'addState',args:[' New ','choice',[S('Inside')],[' First ',': second'],{position:'left of',text:'Note'},[' blue ','red']]},{method:'getState',args:['New'],store:'returned'},{set:['returned','styles',0],value:'fill:#ff0000'}]});
add('existing-state',{ops:[{method:'addState',args:['A','join',[S('Nested')],' Appended ',{position:'right of',text:'New note'},['blue']]},{method:'getState',args:['A']}]});
add('add-relation',{ops:[{method:'addRelation',args:[S(' Left '),S(' Right '),' New edge ']},{method:'getState',args:['Left']}]});
add('links',{ops:[{method:'addLink',args:['A','javascript:alert(1)','Unsafe']},{method:'addLink',args:['B','https://example.com/b','New']}]});
add('description',{ops:[{method:'addDescription',args:['A',': Added']},{method:'addDescription',args:['missing',': Ignored']}]});
add('style-class',{ops:[{method:'addStyleClass',args:['red','fill:blue,color:red;,,stroke:black;']},{method:'addStyleClass',args:['red','fill:green']},{method:'setCssClass',args:['A, B,missing','red']},{method:'named',args:['A, A,unknown'],store:'namedStates'}]});
add('accessibility',{ops:[{method:'setAccTitle',args:[' \uFEFFNative title  ']},{method:'setAccDescription',args:['First\n \tSecond\n\n Third']},{method:'trimColon',args:[' :Trimmed ']},{method:'trimColon',args:[':Trimmed ']}]});
add('style-cache-live',{src:'stateDiagram-v2\nA --> B\nstyle A fill:#247acd',ops:[{set:['states',{map:'A'},'styles',0],value:'fill:#ff0000'}]});
add('style-cache-replaced',{src:'stateDiagram-v2\nA --> B\nstyle A fill:#247acd',ops:[{set:['oldStyles'],value:{ref:['states',{map:'A'},'styles']}},{set:['states',{map:'A'},'styles'],value:['fill:#00ff00']},{set:['oldStyles',0],value:'fill:#ff0000'}]});
add('cached-classes',{ops:[{method:'addStyleClass',args:['blue','fill:#ff0000']},{method:'setCssClass',args:['C','blue']}]});
add('graph-edit',{ops:[{set:['graph','nodes',0,'label'],value:'Direct graph label'},{set:['graph','edges',0,'label'],value:'Direct edge'}]});
add('doc-note-alias',{src:'stateDiagram-v2\nA\nnote right of A: Before',ops:[{set:['rootDoc',1,'note','text'],value:'After'},{method:'getState',args:['A']}]});
add('nested-doc-alias',{src:'stateDiagram-v2\nstate Group {\nA: Before\n}',ops:[{set:['states',{map:'Group'},'doc',0,'description'],value:'After'},{method:'extract',args:[{ref:['rootDoc']}]}]});
add('graph-deleted-source',{src:'stateDiagram-v2\nA --> B\nstyle A fill:#247acd',ops:[{set:['saved'],value:{ref:['states',{map:'A'},'styles']}},{mapDelete:['states','A']},{set:['saved',0],value:'fill:#ff0000'}]});
add('readonly-state-partial',{ops:[{readonly:['states',{map:'A'},'note']},{method:'addState',args:['A','default',{undefined:true},'Added',{position:'right of',text:'Cannot assign'}]}]});
add('readonly-root-partial',{ops:[{readonly:['graph']},{method:'extract',args:[[S('New')]]}]});
add('invalid-desc-partial',{ops:[{method:'addState',args:['New','default',{undefined:true},['First',17]]}]});
add('invalid-doc-partial',{ops:[{method:'setRootDoc',args:[17]}],svg:false});
add('raw-utf16',{ops:[{method:'trimColon',args:[{utf16:[58,32,55296,65]}]},{method:'setAccTitle',args:[{utf16:[32,55296,65]}]}],svg:false});
add('unused-cycles',{ops:[{set:['privateExtra'],value:{ref:[]}},{set:['states',{map:'C'},'extra'],value:{utf16:[55296]}}]});
add('unused-link-cycles',{ops:[{mapSet:['links'],key:'ghost',value:{ref:[]}},{mapSet:['links'],key:17,value:{ref:[]}},{mapSet:['links'],key:'C',value:null}]});
add('readonly-doc-length',{src:'stateDiagram-v2\nA',ops:[{readonly:['rootDoc','length']},{method:'setDirection',args:['RL']},{method:'get direction'}]});
add('readonly-description-length',{ops:[{readonly:['states',{map:'A'},'descriptions','length']},{method:'addDescription',args:['A','Blocked']}]});
add('readonly-class-length',{ops:[{readonly:['classes',{map:'blue'},'styles','length']},{method:'addStyleClass',args:['blue','color:red']}]});
const testPath=fileURLToPath(new URL('../../crates/pele/tests/public_state.rs',import.meta.url));
const test=readFileSync(testPath,'utf8');const marker='const FLOW_VECTORS:';const start=test.indexOf(marker);assert(start>=0,'missing fixture literal');
const vectors=JSON.stringify(cases);
writeFileSync(testPath,test.slice(0,start)+'const FLOW_VECTORS: &str = r####"'+vectors+'"####;\n');
console.log(JSON.stringify({revision,source:'src/diagrams/state/db.ts',artifact:'pele/dist/pele.js',node:process.version,cases:cases.length,operations:cases.reduce((n,c)=>n+c.steps.length,0),methods:[...new Set(cases.flatMap(c=>c.steps.map(s=>s.op.method).filter(Boolean)))].sort(),svgs:cases.reduce((n,c)=>n+c.outputs.filter(o=>!o.error).length,0),sha256:sha(vectors)}));
