// Development-only pinned oracle. No JS diagram logic ships in consumers.
// Regenerates public_usecase.rs literal full-graph/method/semantic/cached SVG vectors.
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
for(const [path,expected]of [["src/diagrams/cynefin/index.ts", "cc54929acd34b9d71e6a22bdf9a48317451ef01c0132ba0fd0fce1256cac4afe"], ["src/diagrams/cynefin/model.ts", "11288261e40e4e07dab2e5f7fb12db202962969c88a469e0b224e13685c36b64"], ["src/diagrams/cynefin/parser.ts", "618a8db8a9761f5429df76064f5a25eaee891fcb96cf549468c36e1d10fd3226"], ["src/diagrams/cynefin/render.ts", "03d04b0e5a27637a468d6a63064ba169ab26f70cea3683fb38da3446bbc32048"], ["src/diagrams/cynefin/boundaries.ts", "20ae7ebce68400bfca1a7946a768575e6411089a48ccef14ec46ae95741c59c8"]])assert.equal(sha(readFileSync(`${root}/${path}`)),expected,path);
const bundle=fileURLToPath(new URL('../../node_modules/pele/dist/pele.js',import.meta.url));
assert.equal(sha(readFileSync(bundle)),'d1ebc1889f3069d7759fe5db34c4ad4b4823285edbee2b537d0ef3d505a76d8d');
// Constructor minifier names and engine-generated TypeError wording are not claimed.
// Arrays carry implicit length separately; object descriptors and every private
// cache/Map/Set reference are canonicalized without losing UTF-16 or IEEE numbers.
assert.equal(Object.getPrototypeOf(parse('cynefin-beta')),Object.prototype);
const flowProto=null;
const source='cynefin-beta\ntitle Decisions\naccTitle: Accessible\naccDescr: Details\ncomplex\n"Investigate"\ncomplicated\n"Consult expert"\nclear\n"Use checklist"\nchaotic\n"Stabilize"\nconfusion\n"Triage"\n"One"\n"Two"\n"Three"\n"Four"\ncomplex --> clear : "Learned"\nclear --> chaotic\nconfusion --> complicated : "Escalate"';
function graph(root){const ids=new Map(),nodes=[];const atom=v=>{if(v===undefined)return['undefined'];if(v===null)return['null'];if(typeof v==='boolean')return['bool',v];if(typeof v==='string')return['string',Array.from({length:v.length},(_,i)=>v.charCodeAt(i))];if(typeof v==='number'){const d=new DataView(new ArrayBuffer(8));d.setFloat64(0,v);return['number',d.getBigUint64(0).toString(16).padStart(16,'0')]}if(typeof v==='function')throw Error('unaccounted function');if(!ids.has(v)){ids.set(v,ids.size);nodes.push(null)}return['ref',ids.get(v)]};const r=atom(root);for(const [v,id]of ids){const kind=Array.isArray(v)?'array':v instanceof Map?'map':v instanceof Set?'set':Object.getPrototypeOf(v)===flowProto?'instance':'object';const n={kind};if(kind==='instance')n.prototype=1;if(kind==='array')n.length=v.length;if(kind==='map')n.entries=[...v].map(([k,v])=>[atom(k),atom(v)]);if(kind==='set')n.values=[...v].map(atom);n.properties=Object.getOwnPropertyNames(v).filter(k=>!(kind==='array'&&k==='length'&&Object.getOwnPropertyDescriptor(v,k).writable)).map(k=>{const d=Object.getOwnPropertyDescriptor(v,k);return[Array.from({length:k.length},(_,i)=>k.charCodeAt(i)),atom(d.value),(d.enumerable?1:0)|(d.writable?2:0)|(d.configurable?4:0)]});nodes[id]=n;}return{root:r,nodes}}
function at(m,path){return path.reduce((v,key)=>typeof key==='object'?v.get(key.map):v[key],m)}
function value(m,v){if(v&&typeof v==='object'){if('record'in v)return Object.fromEntries(v.record.map(([k,v])=>[k,value(m,v)]));if('ref'in v)return at(m,v.ref);if('undefined'in v)return undefined;if('utf16'in v)return String.fromCharCode(...v.utf16);if('number'in v)return({nan:NaN,inf:Infinity,'-inf':-Infinity,'-0':-0})[v.number];if(Array.isArray(v))return v.map(x=>value(m,x));return Object.fromEntries(Object.entries(v).map(([k,v])=>[k,value(m,v)]))}return v}
const cases=[];function add(id,{src=source,options={},ops=[],svg=true}={}){const m=parse(src,options),steps=[];const initial=graph(m);for(const op of ops){let result,error;try{if(op.method){result=m[op.method](...(op.args??[]).map(v=>value(m,v)));if(op.store)m[op.store]=result;}else if(op.set){const target=at(m,op.set.slice(0,-1));target[op.set.at(-1)]=value(m,op.value)}else if(op.reverse)at(m,op.reverse).reverse();else if(op.delete){const target=at(m,op.delete.slice(0,-1));delete target[op.delete.at(-1)]}else if(op.mapDelete){const path=op.mapDelete.slice(0,-1);at(m,path).delete(op.mapDelete.at(-1))}else if(op.mapSet){at(m,op.mapSet).set(value(m,op.key),value(m,op.value))}else if(op.readonly){Object.defineProperty(at(m,op.readonly.slice(0,-1)),op.readonly.at(-1),{writable:false})}else throw Error('op');}catch(e){error={name:e.name,...(e.name==='PeleError'?{message:e.message,code:e.code,type:e.type,line:e.line,column:e.column}:{})}}steps.push({op,graph:graph(m),...(error?{error}:{result:graph(result)})})}const outputs=[];if(svg)for(const maxWidth of [undefined,240]){try{const r=render(m,{maxWidth});outputs.push({maxWidth:maxWidth??null,width:r.width,height:r.height,svgSha256:createHash('sha256').update(r.svg).digest('hex'),links:r.links})}catch(e){outputs.push({maxWidth:maxWidth??null,error:{name:e.name,...(e.name==='PeleError'?{message:e.message,code:e.code,type:e.type,line:e.line,column:e.column}:{})}})}}cases.push({id,source:src,options,initial,steps,outputs})}
add('initial');add('empty',{src:'cynefin-beta'});
const corpus=JSON.parse(readFileSync(new URL('../../fixtures/pele/cynefin.json',import.meta.url)));
for(const c of corpus.cases.filter(c=>c.expected.model&&c.source.length<2500).slice(0,50))add('corpus/'+c.id,{src:c.source,options:c.options});
for(const domain of ['complex','complicated','clear','chaotic','confusion'])add('domain-'+domain,{ops:[{set:['domains',{map:domain},'name'],value:'wrong display name'},{set:['domains',{map:domain},'items',0,'label'],value:'Native '+domain}]});
add('remove-domain',{ops:[{mapDelete:['domains','complex']},{mapDelete:['domains','clear']}]});
add('replace-domain',{ops:[{mapSet:['domains'],key:'complex',value:{record:[['items',[{label:'Replacement'}]],['name','clear']]}},{mapSet:['domains'],key:'unknown',value:{ref:[]}}]});
add('share-domain',{ops:[{mapSet:['domains'],key:'clear',value:{ref:['domains',{map:'complex'}]}},{set:['domains',{map:'complex'},'items',0,'label'],value:'Shared live label'}]});
add('share-item',{ops:[{set:['domains',{map:'complex'},'items',1],value:{ref:['domains',{map:'complex'},'items',0]}},{set:['domains',{map:'complex'},'items',0,'label'],value:'Shared item'}]});
add('reverse-confusion',{ops:[{reverse:['domains',{map:'confusion'},'items']}]});
add('confusion-tail-hole',{ops:[{delete:['domains',{map:'confusion'},'items',3]},{set:['domains',{map:'confusion'},'items',4,'label'],value:{utf16:[55296]}},{set:['domains',{map:'confusion'},'items',50],value:{ref:[]}}]});
add('confusion-overflow-truncate',{ops:[{set:['domains',{map:'confusion'},'items'],value:[{label:'Only'}]}]});
add('unknown-transition',{ops:[{set:['transitions',0,'from'],value:'unknown'},{set:['transitions',0,'label'],value:{utf16:[55296]}}]});
add('self-transition',{ops:[{set:['transitions',0,'to'],value:'complex'},{set:['transitions',0,'label'],value:{ref:[]}}]});
add('transition-edits',{ops:[{reverse:['transitions']},{set:['transitions',0,'from'],value:'chaotic'},{set:['transitions',0,'to'],value:'clear'},{set:['transitions',0,'label'],value:'Changed transition'}]});
add('transition-alias',{ops:[{set:['transitions',3],value:{ref:['transitions',0]}},{set:['transitions',4],value:{ref:['transitions',0]}},{set:['transitions',0,'label'],value:'Repeated alias'}]});
add('metadata',{ops:[{set:['type'],value:'pie'},{set:['title'],value:'Native title'},{set:['accTitle'],value:'Native accessible'},{set:['accDescr'],value:'Native details'}]});
add('metadata-falsy',{ops:[{set:['accTitle'],value:null},{set:['accDescr'],value:false}]});
add('unknown-map-keys',{ops:[{mapSet:['domains'],key:{number:'nan'},value:{ref:[]}},{mapSet:['domains'],key:{utf16:[55296]},value:{ref:[]}},{mapSet:['domains'],key:'not-a-domain',value:{ref:[]}}]});
add('unused-extra',{ops:[{set:['extra'],value:{ref:[]}},{set:['rawNumber'],value:{number:'-0'}},{set:['rawUnits'],value:{utf16:[55296,65]}}]});
for(const field of ['title','domains','transitions'])add('invalid-'+field,{ops:[{set:[field],value:null}]});
add('sparse-active-item',{ops:[{delete:['domains',{map:'complex'},'items',0]}]});
add('sparse-active-transition',{ops:[{delete:['transitions',0]}]});
add('nullable-domain-values',{ops:[{mapSet:['domains'],key:'complex',value:null},{mapSet:['domains'],key:'clear',value:17},{set:['domains',{map:'confusion'},'items'],value:null}]});
const testPath=fileURLToPath(new URL('../../crates/pele/tests/public_cynefin.rs',import.meta.url)),test=readFileSync(testPath,'utf8'),marker='const FLOW_VECTORS:',start=test.indexOf(marker);assert(start>=0);const vectors=JSON.stringify(cases);writeFileSync(testPath,test.slice(0,start)+'const FLOW_VECTORS: &str = r####"'+vectors+'"####;\n');console.log(JSON.stringify({revision,source:'src/diagrams/cynefin/model.ts',artifact:'pele/dist/pele.js',node:process.version,cases:cases.length,operations:cases.reduce((n,c)=>n+c.steps.length,0),methods:[],svgs:cases.reduce((n,c)=>n+c.outputs.filter(o=>!o.error).length,0),sha256:sha(vectors)}));
