// Development-only pinned Pele 458322a5 oracle. No consumer JS runtime.
// `node scripts/parity/probe-public-railroad.mjs --write` updates embedded oracle.
process.env.TZ='UTC';
import {parse,render} from 'pele';
import {readFile,writeFile} from 'node:fs/promises';
import{createHash}from'node:crypto';
import assert from'node:assert/strict';
const revision='458322a5b8abb1ae57ddf127c32017ab9fb14a92',now=1704067200000;
Date.now=()=>now;
const digest=s=>createHash('sha256').update(s).digest('hex');
const provenance={};for(const file of['db','types','common','render'])provenance[file]=digest(await readFile(`/tmp/pele-upstream-458322a5/src/diagrams/railroad/${file}.ts`));


assert.equal(provenance.db,'90fbe88ac944f2b9663f379bd61024e15949648c5c971f2667b64671b0d35cd0');
const bits=n=>{const b=Buffer.alloc(8);b.writeDoubleBE(n);return b.toString('hex')};
const sample=parse('railroad-beta'),proto=Object.getPrototypeOf(sample),functions=new Map(Object.getOwnPropertyNames(proto).filter(k=>k!=='constructor').map((k,i)=>[proto[k],i+1]));
const text=s=>Array.from({length:s.length},(_,i)=>s.charCodeAt(i));
function canonical(roots){const ids=new Map,queue=[];function atom(v){if(v===undefined)return{kind:'undefined'};if(v===null)return{kind:'null'};if(typeof v==='boolean')return{kind:'bool',value:v};if(typeof v==='number')return{kind:'number',value:bits(v)};if(typeof v==='string')return{kind:'string',value:text(v)};if(typeof v==='function'){assert(functions.has(v),'unknown function');return{kind:'function',value:functions.get(v)}}if(!ids.has(v)){ids.set(v,queue.length);queue.push(v)}return{kind:'ref',value:ids.get(v)}}const out={roots:roots.map(atom),nodes:[]};for(let i=0;i<queue.length;i++){const v=queue[i],properties=[];for(const key of Object.getOwnPropertyNames(v)){const d=Object.getOwnPropertyDescriptor(v,key);if(Array.isArray(v)&&key==='length'&&d.writable)continue;assert('value'in d,'accessor capability is separate');properties.push({key:text(key),value:atom(d.value),flags:(d.enumerable?1:0)|(d.writable?2:0)|(d.configurable?4:0)})}let n={kind:Object.getPrototypeOf(v)===proto?'instance':v instanceof Date?'date':v instanceof Map?'map':v instanceof Set?'set':Array.isArray(v)?'array':'object',properties};if(n.kind==='instance')n.prototype='RailroadDb';if(n.kind==='array')n.length=v.length;if(n.kind==='date')n.milliseconds=bits(+v);if(n.kind==='map')n.entries=[...v].map(([k,v])=>[atom(k),atom(v)]);if(n.kind==='set')n.values=[...v].map(atom);out.nodes.push(n)}return out}
const node=(...keys)=>({$node:keys}),date=ms=>({$date:ms}),undef={$undefined:true},raw=(...units)=>({$utf16:units});
function resolve(v,m){if(v&&typeof v==='object'&&!Array.isArray(v)){if(v.$record)return Object.fromEntries(v.$record.map(([k,v])=>[k,resolve(v,m)]));if(v.$map)return new Map(v.$map.map(([k,v])=>[resolve(k,m),resolve(v,m)]));if(v.$borrowed){const b={...m};for(const [f,id]of functions)b[Object.getOwnPropertyNames(proto).find(k=>proto[k]===f)]=f;return b;}if(v.$node){let out=m;for(const key of v.$node)out=out instanceof Map?out.get(key):out[key];return out}if(v.$undefined)return undefined;if(v.$utf16)return String.fromCharCode(...v.$utf16);if(v.$number)return Buffer.from(v.$number,'hex').readDoubleBE();if('$date'in v)return new Date(resolve(v.$date,m));if(v.$method)return proto[v.$method];return Object.fromEntries(Object.entries(v).map(([k,v])=>[k,resolve(v,m)]))}return Array.isArray(v)?v.map(v=>resolve(v,m)):v}


const source='railroad-beta\nr = sequence(terminal("a"),optional(nonterminal("other")));\nr = terminal("last");\nother = oneOrMore(terminal("x"));';
const term=value=>({$record:[['type','terminal'],['value',value]]});
const nonterminal=name=>({$record:[['type','nonterminal'],['name',name]]});
const sequence=elements=>({$record:[['type','sequence'],['elements',elements]]});
const choice=alternatives=>({$record:[['type','choice'],['alternatives',alternatives]]});
const optional=element=>({$record:[['type','optional'],['element',element]]});
const repeat=(element,min,max)=>({$record:[['type','repetition'],['element',element],['min',min],['max',max]]});
const rule=(name,definition)=>({$record:[['name',name],['definition',definition]]});
const vectors=[];function add(name,calls,extra={}){vectors.push({name,source,calls,...extra})}
add('all-methods-and-duplicate-alias',[['setTitle','Grammar'],['setAccTitle','  Accessible  '],['setAccDescription','A\n  B\n\tC'],['getRule','r'],['getRule','other'],['getRule','missing'],['addRule',rule('r',choice([term('new'),nonterminal('other')]))],['getRule','r'],['addRule',node('rules',0)],['getRule','r']]);
for(const[notation,source]of[
 ['ebnf','railroad-ebnf-beta\ntitle Grammar\nr = "a" b ( [ ? any ? ] | { "x" } ) ;\nr = "z"+ ;'],
 ['abnf','railroad-abnf-beta\naccTitle: ABNF\nr = "a" b ( [ c ] / *"x" ) ;\nr = 2*4"z" ;'],
 ['peg','railroad-peg-beta\naccDescr: PEG\nr <- "a" b ( c? / "x"* ) ;\nr <- "z"+ ;']])add('notation-'+notation,[['getRule','r'],['addRule',rule('new',term(notation))],['getRule','new']],{source});
add('all-ir-node-shapes',[['addRule',rule('all',sequence([term(' '),nonterminal('r'),{$record:[['type','special'],['text','<tag> *literal*']]},optional(term('optional')),choice([]),repeat(term('once'),1,1),repeat(term('few'),2,4),repeat(term('none'),0,{$number:'7ff0000000000000'})]))]]);
add('edited-rule-name-retains-private-map',[{method:'getRule',args:['r'],edit:{target:node('rules',1),key:'name',value:'renamed',flags:7}},['getRule','renamed'],['addRule',node('rules',1)],['getRule','r'],['getRule','renamed']]);
add('reordered-replaced-rules-detached-map',[{method:'getRule',args:['r'],edit:{target:node(),key:'rules',value:[node('rules',2),node('rules',0)],flags:7}},['getRule','other'],['addRule',rule('new',term('n'))],['getRule','r']]);
add('lookup-map-replacement-delete',[{method:'getRule',args:['other'],edit:{target:node(),key:'byName',value:{$map:[['only',node('rules',0)]]},flags:7}},['getRule','r'],['getRule','only'],['addRule',rule('r',term('replacement'))],['getRule','r']]);
add('array-repeated-rule-object',[{method:'getRule',args:['r'],edit:{target:node(),key:'rules',value:[node('rules',0),node('rules',0),node('rules',1)],flags:7}},['getRule','r']]);
add('borrowed-receiver-shares-rule-map',[{method:'setTitle',receiver:{$borrowed:true},args:['Borrowed']},{method:'addRule',receiver:{$borrowed:true},args:[rule('borrowed',term('b'))]},{method:'getRule',receiver:{$borrowed:true},args:['borrowed']}]);
add('readonly-title-and-acc-errors',[{method:'setTitle',args:['new'],edit:{target:node(),key:'title',value:'old',flags:5}},['setAccTitle',42],['setAccDescription',null],['getRule','r']]);
add('readonly-rules-array-stops-before-map',[{method:'addRule',args:[rule('no',term('n'))],edit:{target:node('rules'),key:'length',value:3,flags:0}},['getRule','no']]);
add('bad-map-error-publishes-pushed-rule',[{method:'addRule',args:[rule('pushed',term('p'))],edit:{target:node(),key:'byName',value:{$record:[]},flags:7}},{method:'getRule',args:['r']},{method:'getRule',args:['pushed'],edit:{target:node(),key:'byName',value:{$map:[['pushed',node('rules',3)]]},flags:7}}]);
add('missing-rule-name-is-undefined-map-key',[['addRule',{$record:[['definition',term('incomplete')]]}],['getRule',undef]],{renderGap:'Missing/nonstring rule name is outside concrete typed text renderer'});
add('null-rule-after-push-and-member-resolution',[{method:'addRule',args:[null],edit:{target:node(),key:'byName',value:{$record:[]},flags:7}}],{renderGap:'Malformed null rule retained after partial method failure; typed renderer refuses'});
add('utf16-metadata-and-rule-storage',[['setTitle',raw(84,0xd800)],['setAccTitle',raw(32,0xdc00)],['setAccDescription',raw(65,10,32,0xd800)],['addRule',rule(raw(78,0xdc00),term(raw(86,0xd800)))],['getRule',raw(78,0xdc00)]],{renderGap:'Active lone-surrogate SVG text transport remains explicit'});
add('inactive-private-utf16-map-key',[{method:'getRule',args:[raw(75,0xd800)],edit:{target:node(),key:'byName',value:{$map:[[raw(75,0xd800),node('rules',0)]]},flags:7}}]);
for(const[i,min,max]of[[0,.5,2.5],[1,-1,4],[2,{$number:bits(-0)},{$number:bits(-0)}],[3,{$number:'7ff8000000000000'},{$number:'7ff0000000000000'}],[4,{$number:'fff0000000000000'},{$number:'fff0000000000000'}],[5,{$number:'7ff0000000000000'},3]])add('ieee-repeat-'+i,[['addRule',rule('count',repeat(term('repeat'),min,max))]]);
// One sequence object appears at unequal depths. Room is a BFS identity cache,
// not independently propagated down each occurrence of the shared subtree.
for(const[variant,width]of[[0,220],[1,300],[2,430]]){
 const shared=sequence([term('alpha'),term('beta'),term('gamma'),term('delta')]);
 add('shared-ir-unequal-depth-'+variant,[['addRule',rule('shared',shared)],['addRule',rule('alias',choice([node('rules',3,'definition'),optional(repeat(node('rules',3,'definition'),0,3))]))]],{options:{maxWidth:width},trace:true});
}
add('shared-leaf-and-branch-trace',[['addRule',rule('leaf',term('shared'))],['addRule',rule('aliased',sequence([node('rules',3,'definition'),choice([node('rules',3,'definition'),optional(node('rules',3,'definition'))]),node('rules',3,'definition')]))]],{options:{maxWidth:190},trace:true});
for(const v of vectors){const m=parse(v.source);v.initial=canonical([m]);v.steps=[];for(let entry of v.calls){if(Array.isArray(entry))entry={method:entry[0],args:entry.slice(1)};if(entry.edit){const e=entry.edit;Object.defineProperty(resolve(e.target,m),e.key,{value:resolve(e.value,m),enumerable:!!(e.flags&1),writable:!!(e.flags&2),configurable:!!(e.flags&4)})}const args=entry.args.map(v=>resolve(v,m));const receiver=entry.receiver?resolve(entry.receiver,m):m;let value,result;try{value=Reflect.apply(proto[entry.method],receiver,args);result={kind:'value'}}catch(e){result={kind:'error',name:e.name,message:text(e.message),code:e.code,type:e.type,line:e.line,column:e.column,snippet:e.snippet}}const after=canonical([m,receiver,...args,value]);let rendering;if(v.renderGap||entry.renderGap){rendering={gap:v.renderGap||entry.renderGap}}else{try{const measureTrace=[];const measurer=v.trace?{width:(t,z,s)=>{measureTrace.push([text(t),bits(z),s]);return t.length*z*((s&1)?0.71:0.6)}}:undefined;const r=render(m,{...v.options,now,measurer});rendering={width:r.width,height:r.height,svgSha256:digest(r.svg),...(v.trace?{measureTrace}:undefined)}}catch(e){rendering={error:{name:e.name,message:text(e.message),code:e.code,type:e.type,line:e.line,column:e.column,snippet:e.snippet}}} }v.steps.push({entry,result,after,rendering,renderedGraph:canonical([m])})}delete v.calls;}
const output={revision,provenance,now,descriptorNameExclusions:['Function.name','Function.prototype.toString'],capabilityGaps:['constructor/callback and native-helper overrides','active lone-surrogate SVG scalar transport','malformed/unknown IR and nonnumeric bounds beyond the concrete typed renderer','active IR cycles or exponential alias expansion: bounded native refusal','pre-callback host graph synchronization and reentrancy'],own:Object.keys(sample),methods:[...functions].map(([f,id])=>({id,name:Object.getOwnPropertyNames(proto).find(k=>proto[k]===f),arity:f.length,flags:6})),vectors};
const json=JSON.stringify(output);if(process.argv.includes('--write')){const path='crates/pele/tests/public_railroad.rs',s=await readFile(path,'utf8'),a=s.indexOf('// ORACLE_BEGIN'),b=s.indexOf('// ORACLE_END');assert(a>=0&&b>a);await writeFile(path,s.slice(0,a)+'// ORACLE_BEGIN\nconst ORACLE:&str=r'+ '#'.repeat(32)+'"'+json+'"'+'#'.repeat(32)+';\n'+s.slice(b));console.log(`${vectors.length} vectors, ${vectors.reduce((n,v)=>n+v.steps.length,0)} calls, ${Buffer.byteLength(json)} bytes`)}else console.log(json);
