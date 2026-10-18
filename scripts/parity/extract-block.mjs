// Development-only public API oracle. Pinned Pele JavaScript never ships in consumers.
import {readFile,readdir,writeFile} from 'node:fs/promises';
import {execFileSync} from 'node:child_process';
import {stripTypeScriptTypes} from 'node:module';
import {createHash} from 'node:crypto';
import {gzipSync} from 'node:zlib';
import assert from 'node:assert/strict';
import {parse,render} from 'pele';
const root=process.argv[2]??'/tmp/pele-upstream-458322a5';
const revision='458322a5b8abb1ae57ddf127c32017ab9fb14a92';
assert.equal(execFileSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).trim(),revision);
execFileSync('git',['diff','--quiet',revision,'--','src','tests'],{cwd:root});
const provenance=[];
for(const path of ['package.json',...['index','types','db','lexer','parser','tokens','layout','arrows','render'].map(n=>`src/diagrams/block/${n}.ts`),'src/text/label.ts','src/text/metrics.ts','src/text/measurer.ts','src/util/chars.ts','src/svg/theme.ts','src/diagrams/common/fit-width.ts',...(await readdir(`${root}/tests/compat/block`)).filter(f=>/\.spec\.[jt]s$/.test(f)).map(f=>`tests/compat/block/${f}`),'src/svg/shapes.ts','src/svg/edges.ts','tests/block/oracle.test.ts','tests/block/stress.test.ts','tests/render/block.test.ts','tests/render/block-security.test.ts','tests/corpus/block-docs.json','tests/support/corpus.ts','tests/support/inert.ts']){
  const data=await readFile(`${root}/${path}`);provenance.push({path,sha256:createHash('sha256').update(data).digest('hex')});
}
function literals(text){const out=[];for(const m of text.matchAll(/`((?:[^`\\]|\\.)*)`|'((?:[^'\\\n]|\\.)*)'|"((?:[^"\\\n]|\\.)*)"/g)){const raw=m[1]??m[2]??m[3];try{out.push(new Function('return `'+raw.replace(/`/g,'\\`').replace(/\$\{/g,'\\${')+'`')())}catch{}}return out}
const corpus=new Set();
for(const file of(await readdir(`${root}/tests/compat/block`)).filter(f=>/\.spec\.[jt]s$/.test(f)))for(const source of literals(await readFile(`${root}/tests/compat/block/${file}`,'utf8')))if(/block/.test(source))corpus.add(source);
for(const source of JSON.parse(await readFile(`${root}/tests/corpus/block-docs.json`,'utf8')))corpus.add(source);
const seeds=new Set(corpus);
for(const path of ['tests/block/oracle.test.ts','tests/render/block.test.ts','tests/render/block-security.test.ts'])for(const source of literals(await readFile(`${root}/${path}`,'utf8')))if(/block/.test(source))seeds.add(source);
const {REGULAR,BOLD}=await import(`data:text/javascript,${encodeURIComponent(stripTypeScriptTypes(await readFile(`${root}/src/text/metrics.ts`,'utf8')))}`);
const tables=[REGULAR,BOLD].map(p=>Array.from({length:95},(_,i)=>parseInt(p.slice(i*2,i*2+2),36)));
const metrics={width(text,size,style){const table=tables[style&1?1:0],mono=!!(style&4);let units=0;for(let i=0;i<text.length;i++){const c=text.charCodeAt(i);if(c<32)continue;if(c<127)units+=mono?600:table[c-32];else if(c>=0x300&&c<=0x36f||c>=0x200b&&c<=0x200f||c>=0xdc00&&c<=0xdfff||c>=0xfe00&&c<=0xfe0f||c===0xfeff)continue;else if(c>=0x1100&&c<=0x115f||c>=0x2e80&&c<=0xa4cf||c>=0xac00&&c<=0xd7a3||c>=0xf900&&c<=0xfaff||c>=0xfe30&&c<=0xfe4f||c>=0xff00&&c<=0xff60||c>=0xffe0&&c<=0xffe6||c>=0xd800&&c<=0xdbff)units+=mono?1200:1000;else units+=mono?600:580}return units*size/1000}};
const modelKeys=new Set(['type','root','blocks','edges','classes','byId','title','accTitle','accDescr']);
const normalize=v=>JSON.parse(JSON.stringify(v,function(k,v){if(this===v)return v;if(k&&!modelKeys.has(k)&&this?.type==='block')return undefined;return v instanceof Map?[...v]:v}));
const cases=[];
const projectionExclusions=[];
function projection(model){
 const out={};for(const key of modelKeys)if(Object.hasOwn(model,key))out[key]=model[key];
 const stack=[[out,0]];let visits=0;
 while(stack.length){const [v,d]=stack.pop();if(++visits>1000000||d>125)return null;if(v&&typeof v==='object'){for(const value of v instanceof Map?[...v.values()]:Object.values(v))stack.push([value,d+1]);}}
 const result=normalize(out);return Buffer.byteLength(JSON.stringify(result))>16*1024*1024?null:result;
}
function add(id,source,options={},host={}){
 let expected,model,projectionLimit=false;
 try{const parsed=parse(source,options);model=projection(parsed);projectionLimit=model===null;if(projectionLimit){projectionExclusions.push(id);host.modelProjection='excluded';}const resources={measurer:host.measure==='fixed'?{width:(text,size,style)=>text.length*size*(style&1?.71:.6)}:metrics,...(host.icons?{icons:n=>host.icons[n]}:{})};const result=render(parsed,{...options,...resources});assert.deepEqual(result,render(source,{...options,...resources}));if(Buffer.byteLength(result.svg)>250000){result.svgSha256=createHash('sha256').update(result.svg).digest('hex');delete result.svg}expected={...(projectionLimit?{modelProjectionLimit:true}:{model}),result}}
 catch(e){if(e.name!=='PeleError')throw e;expected={...(projectionLimit?{modelProjectionLimit:true}:model?{model}:{}),error:{name:e.name,message:e.message,code:e.code,type:e.type,line:e.line,column:e.column,snippet:e.snippet}}}
 cases.push({id,source,options,host,expected});
}
[...seeds].forEach((source,i)=>{add(`upstream/${i}`,source);add(`upstream-newline/${i}`,source+'\n')});
const shapes=['["Label"]','("Label")','(("Label"))','((("Label")))','{"Label"}','{{"Label"}}','(["Label"])','[["Label"]]','[("Label")]','>"Label"]','[/"Label"/]','[\\"Label"\\]','[/"Label"\\]','[\\"Label"/]','<["Label"]>(right)','<["Label"]>(left,up,down)','<["Label"]>(x,y)'];
for(const shape of shapes)for(const columns of [1,2,3,'auto',0])for(const maxWidth of [undefined,80,320,640])add(`shape-grid/${cases.length}`,`block\ncolumns ${columns}\na${shape}:2 b space c\na -- "Label" --> c\nc-->a\na-->a`,{maxWidth});
for(const padding of [-1,0,2,12,100,200,999,'12',null])for(const maxWidth of [undefined,80,320,640])for(const fontSize of [8,16,19.5,32])add(`nested-layout/${cases.length}`,'---\ntitle: Nested grid\n---\nblock\ncolumns 2\nblock:g["Container"]:2\ncolumns 2\na["Multi<br>line"] b(("Circle"))\nc<["Arrow"]>(x) space\nend\nd{{"Hex"}} e[("Database")]\ng-->d\nb -- "Outer edge" --> e\ne-.->e',{config:{block:{padding}},maxWidth,fontSize});
for(const link of ['-->','---','--x','--o','x--','o--','<--','<-->','==>','===','<==>','-.->','-.-','.->','<-.->','-..-'])for(const label of ['','Label','<b>Styled</b> #amp;'])for(const maxWidth of [undefined,100,320])add(`links/${cases.length}`,`block\ncolumns 3\na space b\nc d e\na ${label?link.startsWith('=')?'== "'+label+'" ==>':link.includes('.')?'-. "'+label+'" .->':'-- "'+label+'" -->':link} b\nb${link}d\na${link}a`,{maxWidth});
for(const label of ['中文 🙂 é','fa:fa-user Person','A<br/>B\\nC','<b>Bold</b> <i>Italic</i>','<unknown>Text</unknown>','&lt;script&gt; #35; #amp;','    ','**literal** _underscore_','VeryLongSingleWord'.repeat(100)])for(const fontSize of [8,16,19.5,40])for(const maxWidth of [undefined,80,320])add(`labels/${cases.length}`,`block\na["${label}"] b\na -- "${label}" --> b`,{fontSize,maxWidth},{icons:{'fa:fa-user':'<path d="M0 0L24 24"/>'}});
for(const classDef of ['fill:red,color:blue,stroke:#123,font-size:24px','font-weight:bold;font-style:italic;fill:#f96','fill:url(javascript:alert(1)),stroke:red','stroke-width:3,rx:9,opacity:0.3','color:#00f,background:green','font-size:0,font-family:monospace'])for(const position of ['before','after'])add(`styles/${cases.length}`,`block\n${position==='before'?'class a hot\nstyle b fill:blue\n':''}a b\nclassDef hot ${classDef}\n${position==='after'?'class a hot\nstyle b fill:blue\n':''}`);
for(const fontFamily of [undefined,'Arial','monospace','"A & B", sans-serif','Arial); url(https://evil.test)'])for(const responsive of [true,false])add(`options/${cases.length}`,'---\ntitle: Title\n---\nblock\nblock:g["Group"]\na["Bold"] space b\na -- "Edge" --> b\nend',{fontFamily,responsive,idPrefix:'a"&b',fontFamilyMono:'Mono'},{measure:'fixed'});
for(const limit of [0,5,6,100])add(`source-limit/${cases.length}`,'block\na',{limit});
for(const outputLimit of [0,10,100,10000])add(`output-limit/${cases.length}`,'block\na',{outputLimit});
const payloadText=await readFile(`${root}/tests/support/inert.ts`,'utf8');const payloadArray=payloadText.slice(payloadText.indexOf('export const PAYLOADS = ')+23);const payloads=new Function('return '+payloadArray.slice(0,payloadArray.indexOf('\n];')+2))();
const security=stripTypeScriptTypes(await readFile(`${root}/tests/render/block-security.test.ts`,'utf8'));
const templateStart=security.search(/const TEMPLATES[^=]*=/)+security.match(/const TEMPLATES[^=]*=\s*/)[0].length;const templates=new Function('return '+security.slice(templateStart,security.indexOf('\n];',templateStart)+2))();
for(const payload of payloads){for(const template of templates)add(`hostile/${cases.length}`,template(payload));add(`hostile-options/${cases.length}`,'block\na space b\na -- "x" --> b',{idPrefix:payload,fontFamily:payload})}
// Run every actual pinned stress closure through recording public operations.
const stress=stripTypeScriptTypes(await readFile(`${root}/tests/block/stress.test.ts`,'utf8'));
const stressStart=stress.search(/const CASES[^=]*=/)+stress.match(/const CASES[^=]*=\s*/)[0].length;const stressCases=new Function('render','parse','tokenize','big','N','repeat','return '+stress.slice(stressStart,stress.indexOf('\n];',stressStart)+2))((s)=>add(`stress/${cases.length}`,s,{limit:1000000}),(s)=>add(`stress/${cases.length}`,s,{limit:1000000}),(s)=>add(`stress/${cases.length}`,s,{limit:1000000}),{},50000,(count,line)=>Array.from({length:count},(_,i)=>line(i)).join(''));
for(const [,run]of stressCases)run();
for(const source of ['block\na space:100001 b','block\n'+'space:60000 '.repeat(2),'block\na space:5 b','block\ncolumns 3\na:'+'9'.repeat(400)+' b c','block\na:'+'9'.repeat(400)+' b c','block\na:0 b:00 c','block\ncolumns 99999999999999999999\na b','block\nblock:g:99999999\na\nend\nb'])add(`stress-extra/${cases.length}`,source,{limit:1000000});
function mutator(corpus,fragments,seed){let state=seed>>>0;const rnd=()=>{state=(Math.imul(state,1664525)+1013904223)>>>0;return state/4294967296};const pick=a=>a[Math.floor(rnd()*a.length)];return()=>{let s=pick(corpus);for(let k=1+Math.floor(rnd()*3);k>0;k--){const pos=Math.floor(rnd()*(s.length+1)),v=rnd();if(v<.35)s=s.slice(0,pos)+pick(fragments)+s.slice(pos);else if(v<.6)s=s.slice(0,pos)+s.slice(pos+1+Math.floor(rnd()*3));else if(v<.75){const other=pick(corpus),from=Math.floor(rnd()*other.length);s=s.slice(0,pos)+other.slice(from,from+Math.floor(rnd()*20))+s.slice(pos)}else if(v<.9)s=s.slice(0,pos)+s.slice(pos,pos+Math.floor(rnd()*6))+s.slice(pos);else s=s.slice(0,pos)+pick(fragments)+s.slice(pos+1)}return s}}
const oracle=await readFile(`${root}/tests/block/oracle.test.ts`,'utf8');const grammarStart=oracle.indexOf('const FRAGMENTS = ')+18;const grammar=new Function('return '+oracle.slice(grammarStart,oracle.indexOf('\n];',grammarStart)+2))();
let surrogateExcluded=0;
for(const [campaign,count,fragments,seed]of [['grammar',5000,grammar,7],['security',4000,[...payloads,'"','["','"]','<["','"]>(right)','\n',':',' --> ',' -- "','" --> ','style a fill:','classDef a ','class a ','block:','end','space:9','columns 2'],3]]){const next=mutator([...corpus],fragments,seed);for(let i=0;i<count;i++){const source=next();if(source.length>4000)continue;if(/[\uD800-\uDBFF](?![\uDC00-\uDFFF])|(?<![\uD800-\uDBFF])[\uDC00-\uDFFF]/u.test(source)){surrogateExcluded++;continue;}add(`${campaign}-mutation/${i}`,source)}}
let state=11;const random=()=>{state=(Math.imul(state,1664525)+1013904223)>>>0;return state/4294967296};for(let i=0;i<300;i++){const n=4+Math.floor(random()*12);let source=`block\ncolumns ${1+Math.floor(random()*5)}\n`;for(let j=0;j<n;j++)source+=`n${j}${shapes[Math.floor(random()*shapes.length)]}:${1+Math.floor(random()*4)} `;for(let j=0;j<n;j++)source+=`\nn${Math.floor(random()*n)} -- "${random()<.5?'Label':''}" --> n${Math.floor(random()*n)}`;add(`generated-layout/${i}`,source,{maxWidth:[undefined,120,320,640][i%4]});}
await writeFile('fixtures/pele/block.json.gz',gzipSync(JSON.stringify({revision,provenance,extraction:{version:1,privateFieldsExcluded:['pending','edgeCount','ids','colors','spaces','warn'],modelProjectionExclusions:projectionExclusions,projectionLimits:{depth:125,bytes:16777216},grammarMutationSeed:7,grammarMutations:5000,securityMutationSeed:3,securityMutations:4000,surrogateExcluded,stressBudget:1000000,largeSvgHashThreshold:250000},cases})+'\n',{level:9,mtime:0}));
console.log(`${cases.length} block cases; ${cases.filter(c=>c.expected.model).length} models; ${cases.filter(c=>c.expected.result).length} SVG; ${cases.filter(c=>c.expected.error).length} errors; ${projectionExclusions.length} model projection exclusions; ${surrogateExcluded} surrogate exclusions`);
