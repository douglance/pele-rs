// Development-only pinned oracle. No JS diagram logic ships in consumers.
// Regenerates public_wardley.rs literal full-graph/method/semantic/cached SVG vectors.
import {parse,render} from 'pele';
import {gunzipSync} from 'node:zlib';
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';import{writeFileSync,readFileSync}from'node:fs';
const root=process.argv[2]??'/tmp/pele-upstream-458322a5';
const revision='458322a5b8abb1ae57ddf127c32017ab9fb14a92';
assert.equal(execFileSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).trim(),revision);
execFileSync('git',['diff','--quiet',revision,'--','src'],{cwd:root});
const sha=data=>createHash('sha256').update(data).digest('hex');
for(const [path,expected]of [["src/diagrams/wardley/index.ts", "6c508e446a3a34ad99e6f5fe145f68cff7c7cc0438fcac402c0894fbebc19c44"], ["src/diagrams/wardley/model.ts", "2ceee67579e2cf8f1ab812d638cce80999ab335522175b6b0eb3c0da1330cb50"], ["src/diagrams/wardley/parser.ts", "4819be3eaffda54267b01415ceb427b267f31e3c78df6e726afb9730709e60a2"], ["src/diagrams/wardley/render.ts", "346213563769a6be540aaddf6916f498ca223f7af68c389afe7e758e40a6f0f4"], ["src/diagrams/wardley/builder.ts", "9ad6e7705fa1e552973c0428724a48d3ec321986c17b8bf2da8346191ea44fee"]])assert.equal(sha(readFileSync(`${root}/${path}`)),expected,path);
const bundle=fileURLToPath(new URL('../../node_modules/pele/dist/pele.js',import.meta.url));
assert.equal(sha(readFileSync(bundle)),'d1ebc1889f3069d7759fe5db34c4ad4b4823285edbee2b537d0ef3d505a76d8d');
// Constructor minifier names and engine-generated TypeError wording are not claimed.
// Arrays carry implicit length separately; object descriptors and every private
// cache/Map/Set reference are canonicalized without losing UTF-16 or IEEE numbers.
assert.equal(Object.getPrototypeOf(parse('wardley-beta')),Object.prototype);
const flowProto=null;
const source="wardley-beta\ntitle Strategy\naccTitle: Accessible\naccDescr: Details\nsize [1000,700]\nevolution Genesis@0.25 -> Custom@0.5 -> Product@0.75 -> Commodity@1.0\nanchor User [0.9,0.1]\ncomponent Service [0.7,0.3] (build)\ncomponent Data [0.4,0.6] (buy) inertia\npipeline Data {\ncomponent SQL [0.3]\ncomponent Cloud [0.8]\n}\nUser -> Service\nService +> Data\nService -.-> SQL\nevolve Service 0.8\nannotations [0.1,0.2]\nannotation 1,[0.6,0.4] \"First annotation\"\nannotation 2,[0.5,0.8] \"Second annotation\"\nnote \"Build carefully\" [0.8,0.6]\naccelerator Cloud [0.3,0.8]\ndeaccelerator Legacy [0.4,0.2]";
function graph(root){const ids=new Map(),nodes=[];const atom=v=>{if(v===undefined)return['undefined'];if(v===null)return['null'];if(typeof v==='boolean')return['bool',v];if(typeof v==='string')return['string',Array.from({length:v.length},(_,i)=>v.charCodeAt(i))];if(typeof v==='number'){const d=new DataView(new ArrayBuffer(8));d.setFloat64(0,v);return['number',d.getBigUint64(0).toString(16).padStart(16,'0')]}if(typeof v==='function')throw Error('unaccounted function');if(!ids.has(v)){ids.set(v,ids.size);nodes.push(null)}return['ref',ids.get(v)]};const r=atom(root);for(const [v,id]of ids){const kind=Array.isArray(v)?'array':v instanceof Map?'map':v instanceof Set?'set':Object.getPrototypeOf(v)===flowProto?'instance':'object';const n={kind};if(kind==='instance')n.prototype=1;if(kind==='array')n.length=v.length;if(kind==='map')n.entries=[...v].map(([k,v])=>[atom(k),atom(v)]);if(kind==='set')n.values=[...v].map(atom);n.properties=Object.getOwnPropertyNames(v).filter(k=>!(kind==='array'&&k==='length'&&Object.getOwnPropertyDescriptor(v,k).writable)).map(k=>{const d=Object.getOwnPropertyDescriptor(v,k);return[Array.from({length:k.length},(_,i)=>k.charCodeAt(i)),atom(d.value),(d.enumerable?1:0)|(d.writable?2:0)|(d.configurable?4:0)]});nodes[id]=n;}return{root:r,nodes}}
function at(m,path){return path.reduce((v,key)=>typeof key==='object'?v.get(key.map):v[key],m)}
function value(m,v){if(v&&typeof v==='object'){if('record'in v)return Object.fromEntries(v.record.map(([k,v])=>[k,value(m,v)]));if('ref'in v)return at(m,v.ref);if('undefined'in v)return undefined;if('utf16'in v)return String.fromCharCode(...v.utf16);if('number'in v)return({nan:NaN,inf:Infinity,'-inf':-Infinity,'-0':-0})[v.number];if(Array.isArray(v))return v.map(x=>value(m,x));return Object.fromEntries(Object.entries(v).map(([k,v])=>[k,value(m,v)]))}return v}
const cases=[];function add(id,{src=source,options={},ops=[],svg=true}={}){const m=parse(src,options),steps=[];const initial=graph(m);for(const op of ops){let result,error;try{if(op.method){result=m[op.method](...(op.args??[]).map(v=>value(m,v)));if(op.store)m[op.store]=result;}else if(op.set){const target=at(m,op.set.slice(0,-1));target[op.set.at(-1)]=value(m,op.value)}else if(op.reverse)at(m,op.reverse).reverse();else if(op.delete){const target=at(m,op.delete.slice(0,-1));delete target[op.delete.at(-1)]}else if(op.mapDelete){const path=op.mapDelete.slice(0,-1);at(m,path).delete(op.mapDelete.at(-1))}else if(op.mapSet){at(m,op.mapSet).set(value(m,op.key),value(m,op.value))}else if(op.readonly){Object.defineProperty(at(m,op.readonly.slice(0,-1)),op.readonly.at(-1),{writable:false})}else throw Error('op');}catch(e){error={name:e.name,...(e.name==='PeleError'?{message:e.message,code:e.code,type:e.type,line:e.line,column:e.column}:{})}}steps.push({op,graph:graph(m),...(error?{error}:{result:graph(result)})})}const outputs=[];if(svg)for(const maxWidth of [undefined,240]){try{const r=render(m,{maxWidth});outputs.push({maxWidth:maxWidth??null,width:r.width,height:r.height,...(!Number.isFinite(r.width)?{widthNonfinite:Number.isNaN(r.width)?'nan':r.width>0?'inf':'-inf'}:{}),...(!Number.isFinite(r.height)?{heightNonfinite:Number.isNaN(r.height)?'nan':r.height>0?'inf':'-inf'}:{}),svgSha256:createHash('sha256').update(r.svg).digest('hex'),links:r.links})}catch(e){outputs.push({maxWidth:maxWidth??null,error:{name:e.name,...(e.name==='PeleError'?{message:e.message,code:e.code,type:e.type,line:e.line,column:e.column}:{})}})}}cases.push({id,source:src,options,initial,steps,outputs})}
add('initial');add('empty',{src:'wardley-beta'});
const corpus=JSON.parse(gunzipSync(readFileSync(new URL('../../fixtures/pele/wardley.json.gz',import.meta.url))));
for(const c of corpus.cases.filter(c=>c.expected.model&&c.source.length<2500).slice(0,55))add('corpus/'+c.id,{src:c.source,options:c.options});
add('duplicate-anchor-component',{src:'wardley-beta\nanchor A [0.9,0.1]\ncomponent A [0.6,0.4]\ncomponent A [0.7,0.8] label [4,5]'});
add('pipeline-node-replacement',{src:'wardley-beta\ncomponent A [0.6,0.4]\ncomponent A_B [0.4,0.5]\npipeline A {\ncomponent B [0.2]\n}\npipeline A_B {\ncomponent X [0.3]\n}\npipeline A {\ncomponent B [0.8]\n}'});
for(const [field,value]of [['label','Native label'],['x',20],['y',80],['className','anchor'],['labelOffsetX',-40],['labelOffsetY',30],['inertia',true],['inPipeline',true],['isPipelineParent',true],['sourceStrategy','market']])add('node-'+field,{ops:[{set:['nodes',1,field],value}]});
add('node-id-links',{ops:[{set:['nodes',1,'id'],value:'renamed'},{set:['links',0,'target'],value:'renamed'},{set:['trends',0,'nodeId'],value:'renamed'}]});
add('reorder-all',{ops:['nodes','links','trends','pipelines','annotations','notes','accelerators','deaccelerators'].map(field=>({reverse:[field]}))});
add('node-alias',{ops:[{set:['nodes',5],value:{ref:['nodes',1]}},{set:['nodes',1,'label'],value:'Shared node'}]});
add('link-edit',{ops:[{set:['links',0,'label'],value:'Visible caption'},{set:['links',0,'dashed'],value:true},{set:['links',0,'flow'],value:'bidirectional'}]});
add('link-ignore',{ops:[{set:['links',0,'target'],value:'missing'},{set:['links',0,'label'],value:{utf16:[55296]}}]});
add('link-self',{ops:[{set:['links',0,'source'],value:'Service'},{set:['links',0,'label'],value:{ref:[]}}]});
add('unused-flow-class',{ops:[{set:['links',0,'flow'],value:{ref:[]}},{set:['nodes',1,'className'],value:{utf16:[55296]}}]});
add('link-alias',{ops:[{set:['links',4],value:{ref:['links',0]}},{set:['links',0,'flow'],value:'backward'}]});
add('trend-edit',{ops:[{set:['trends',0,'targetX'],value:92},{set:['trends',0,'targetY'],value:40}]});
add('trend-ignore',{ops:[{set:['trends',0,'nodeId'],value:'unknown'},{set:['trends',0,'targetX'],value:{ref:[]}}]});
add('pipeline-edit',{ops:[{reverse:['pipelines',0,'componentIds']},{set:['pipelines',0,'componentIds',2],value:'Service'},{set:['pipelines',0,'componentIds',3],value:'missing'}]});
add('annotation-edit',{ops:[{set:['annotations',0,'number'],value:7},{set:['annotations',0,'text'],value:'Native annotation'},{set:['annotations',0,'coordinates',1],value:{ref:['annotations',0,'coordinates',0]}},{set:['annotations',0,'coordinates',0,'x'],value:25},{set:['annotationsBox'],value:{x:80,y:40}}]});
add('notes-forces',{ops:[{set:['notes',0,'text'],value:'Native note'},{set:['notes',0,'x'],value:15},{set:['accelerators',0,'name'],value:'Accelerate'},{set:['accelerators',0,'y'],value:90},{set:['deaccelerators',0,'name'],value:'Slow down'}]});
add('custom-axes',{ops:[{set:['axes','xLabel'],value:'Maturity'},{set:['axes','yLabel'],value:'Need'},{set:['axes','stages'],value:['Start','Middle','End']},{set:['axes','stageBoundaries'],value:[0.2,0.6,1]}]});
add('axes-empty-mismatch',{ops:[{set:['axes','stages'],value:[]},{set:['axes','stageBoundaries'],value:[0.5]}]});
add('axes-null-label',{ops:[{set:['axes','xLabel'],value:null},{set:['axes','yLabel'],value:null}]});
add('size',{ops:[{set:['size'],value:{record:[['width',1200],['height',700]]}}]});
add('size-fallback',{ops:[{set:['size'],value:{record:[['width',{number:'inf'}],['height','not numeric']]}}]});
add('metadata',{ops:[{set:['type'],value:'pie'},{set:['title'],value:'Native title'},{set:['accTitle'],value:'Accessible'},{set:['accDescr'],value:'Detailed'}]});
add('metadata-falsy',{ops:[{set:['accTitle'],value:null},{set:['accDescr'],value:false}]});
add('unused-cyclic-extra',{ops:[{set:['extra'],value:{ref:[]}},{set:['nan'],value:{number:'nan'}},{set:['raw'],value:{utf16:[55296,65]}}]});
add('numeric-bool-null',{ops:[{set:['nodes',1,'x'],value:null},{set:['nodes',1,'y'],value:true},{set:['nodes',1,'labelOffsetX'],value:null}]});
add('numeric-nonfinite',{ops:[{set:['nodes',1,'x'],value:{number:'nan'}},{set:['nodes',1,'y'],value:{number:'inf'}}]});
for(const field of ['title','nodes','axes','links','notes'])add('invalid-'+field,{ops:[{set:[field],value:null}]});
add('active-sparse-node',{ops:[{delete:['nodes',0]}]});
add('active-sparse-coordinate',{ops:[{delete:['annotations',0,'coordinates',0]}]});
const testPath=fileURLToPath(new URL('../../crates/pele/tests/public_wardley.rs',import.meta.url)),test=readFileSync(testPath,'utf8'),marker='const FLOW_VECTORS:',start=test.indexOf(marker);assert(start>=0);const vectors=JSON.stringify(cases);writeFileSync(testPath,test.slice(0,start)+'const FLOW_VECTORS: &str = r####"'+vectors+'"####;\n');console.log(JSON.stringify({revision,source:'src/diagrams/wardley/model.ts',artifact:'pele/dist/pele.js',node:process.version,cases:cases.length,operations:cases.reduce((n,c)=>n+c.steps.length,0),methods:[],svgs:cases.reduce((n,c)=>n+c.outputs.filter(o=>!o.error).length,0),sha256:sha(vectors)}));
