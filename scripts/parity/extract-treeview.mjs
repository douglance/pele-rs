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
for(const path of ['package.json',...['index','parser','model','boxDrawing','icons','render'].map(n=>`src/diagrams/treeview/${n}.ts`),'src/text/label.ts','src/text/metrics.ts','src/text/measurer.ts','src/util/chars.ts','src/svg/theme.ts','src/diagrams/common/fit-width.ts',...(await readdir(`${root}/tests/compat/treeview`)).filter(f=>/\.spec\.[jt]s$/.test(f)).map(f=>`tests/compat/treeview/${f}`),'src/diagrams/common/accDescr.ts','src/diagrams/common/tokens.ts','src/diagrams/common/literal-label.ts','tests/treeview/oracle.test.ts','tests/treeview/stress.test.ts','tests/render/treeview.test.ts','tests/render/treeview-security.test.ts','tests/corpus/treeview-docs.json','tests/support/corpus.ts','tests/support/inert.ts']){
  const data=await readFile(`${root}/${path}`);provenance.push({path,sha256:createHash('sha256').update(data).digest('hex')});
}
function literals(text){const out=[];for(const m of text.matchAll(/`((?:[^`\\]|\\.)*)`|'((?:[^'\\\n]|\\.)*)'|"((?:[^"\\\n]|\\.)*)"/g)){const raw=m[1]??m[2]??m[3];try{out.push(new Function('return `'+raw.replace(/`/g,'\\`').replace(/\$\{/g,'\\${')+'`')())}catch{}}return out}
const corpus=new Set();
for(const file of(await readdir(`${root}/tests/compat/treeview`)).filter(f=>/\.spec\.[jt]s$/.test(f)))for(const source of literals(await readFile(`${root}/tests/compat/treeview/${file}`,'utf8')))if(/treeView-beta/.test(source))corpus.add(source);
for(const source of JSON.parse(await readFile(`${root}/tests/corpus/treeview-docs.json`,'utf8')))corpus.add(source);
const seeds=new Set(corpus);
for(const file of(await readdir(`${root}/tests/compat/treeview`)).filter(f=>/\.test\.[jt]s$/.test(f)))for(const source of literals(await readFile(`${root}/tests/compat/treeview/${file}`,'utf8')))if(/treeView-beta/.test(source))seeds.add(source);
for(const path of ['tests/treeview/oracle.test.ts','tests/render/treeview.test.ts','tests/render/treeview-security.test.ts'])for(const source of literals(await readFile(`${root}/${path}`,'utf8')))if(/treeView-beta/.test(source))seeds.add(source);
const {REGULAR,BOLD}=await import(`data:text/javascript,${encodeURIComponent(stripTypeScriptTypes(await readFile(`${root}/src/text/metrics.ts`,'utf8')))}`);
const tables=[REGULAR,BOLD].map(p=>Array.from({length:95},(_,i)=>parseInt(p.slice(i*2,i*2+2),36)));
const metrics={width(text,size,style){const table=tables[style&1?1:0],mono=!!(style&4);let units=0;for(let i=0;i<text.length;i++){const c=text.charCodeAt(i);if(c<32)continue;if(c<127)units+=mono?600:table[c-32];else if(c>=0x300&&c<=0x36f||c>=0x200b&&c<=0x200f||c>=0xdc00&&c<=0xdfff||c>=0xfe00&&c<=0xfe0f||c===0xfeff)continue;else if(c>=0x1100&&c<=0x115f||c>=0x2e80&&c<=0xa4cf||c>=0xac00&&c<=0xd7a3||c>=0xf900&&c<=0xfaff||c>=0xfe30&&c<=0xfe4f||c>=0xff00&&c<=0xff60||c>=0xffe0&&c<=0xffe6||c>=0xd800&&c<=0xdbff)units+=mono?1200:1000;else units+=mono?600:580}return units*size/1000}};
const modelKeys=new Set(['type','root','count','title','accTitle','accDescr']);
const normalize=v=>JSON.parse(JSON.stringify(v,(_,v)=>v instanceof Map?[...v]:v));
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
const tree='treeView-beta\ntitle Project\naccTitle: Accessible tree\naccDescr { Native\n and web }\nproject/\n  src/ :::highlight ## Sources\n    main.ts icon(logos:ts) ## Main module\n    empty icon()\n  README.md ## Documentation\n__proto__\nconstructor\n';
for(const maxWidth of [undefined,1,80,120,240,320,640,800])for(const fontSize of [8,16,19.5,32])for(const padding of [0,8,19.5])add(`layout/${cases.length}`,tree,{maxWidth,fontSize,padding});
for(const key of ['paddingX','paddingY','rowIndent','lineThickness'])for(const value of [-1,0,.25,1,2,5,12,100,400,401,1000,null,'3',{},[]])for(const maxWidth of [undefined,80,320])add(`settings/${cases.length}`,tree,{config:{treeView:{[key]:value}},maxWidth});
const iconTree='treeView-beta\nfolder/\n  a.ts\n  Dockerfile\n  notes.txt\n  .hidden\n  UPPER.TS\n  noext\n  x icon(custom)\n  y icon()\n  z icon(none)\n  a.b.C\n  __proto__\n  constructor\n';
for(const showIcons of [true,false,0,1,'false','',{},[],null])for(const defaultIconPack of ['', 'pack','mermaid-treeview',null,12])for(const maxWidth of [undefined,160])add(`icon-config/${cases.length}`,iconTree,{config:{treeView:{showIcons,defaultIconPack,filenameIcons:{Dockerfile:'docker',constructor:'own',__proto__:null},extensionIcons:{'.ts':'typescript',txt:'none','.c':'file'}}},maxWidth},{icons:{'pack:docker':'<circle r="4"/>','mermaid-treeview:file':'','mermaid-treeview:folder':'<path d="M0 0L24 24"/>'}});
for(const mapping of [{},{filenameIcons:null,extensionIcons:[]},{filenameIcons:{'a.ts':'none'},extensionIcons:{'.ts':'code'}},{filenameIcons:{'a.ts':''},extensionIcons:{'.ts':'' ,ts:'other'}},{extensionIcons:{'.ts':null,ts:'code'}},{filenameIcons:{'__proto__':'file',constructor:'folder',toString:'none'}}])add(`icon-maps/${cases.length}`,iconTree,{config:{treeView:{showIcons:true,defaultIconPack:'custom',...mapping}}});
for(const name of ['Plain',' a  b   c ','<script>alert(1)</script>','A<br/>B\\nC','&#x1f642; #35; #amp;','**literal** _under_score_','中文 🙂 é','__proto__','VeryLongSingleWord'.repeat(100),'fa:fa-user Text'])for(const quote of ['','"',"'"])for(const maxWidth of [undefined,80,190])add(`labels/${cases.length}`,`treeView-beta\n${quote}${name}${quote} ## ${name}\n  child/ :::highlight\n`,{maxWidth},{icons:{'fa:fa-user':'<path d="M0 0L24 24"/>'}});
for(const source of ['treeView-beta','treeView-beta ','treeView-beta title T','treeView-beta\ntitle','treeView-beta\naccTitle:','treeView-beta\naccDescr {}','treeView-beta\na\ntitle Later','treeView-beta\n"a" "b" c','treeView-beta\n"a\n b"','treeView-beta\na :::x :::y icon() icon(c)','treeView-beta\n├── src/\n│\n│   └── "a\n','treeView-beta\n├── \n','treeView-beta\n├── a\n   b\n','treeView-beta\na ─ b\n','treeView-beta\n├── a\r\n└── b\r\n'])add(`boundary/${cases.length}`,source);
for(const branch of ['├──','└──','┣━━','┗━━','├─','└'])for(const segment of ['    ','│   ','┃   ','\t','  '])for(const maxWidth of [undefined,80,320])add(`box/${cases.length}`,`treeView-beta\n${branch} src/\n│\n${segment}${branch} index.ts ## Main\n${segment}${branch} utils.ts\n└── README.md`,{maxWidth});
for(const fontFamily of [undefined,'Arial','monospace','"A & B", sans-serif','Arial); url(https://evil.test)'])for(const responsive of [true,false])add(`options/${cases.length}`,tree,{fontFamily,responsive,idPrefix:'a"&b',fontFamilyMono:'Mono'},{measure:'fixed'});
for(const limit of [0,12,13,100])add(`source-limit/${cases.length}`,'treeView-beta',{limit});
for(const outputLimit of [0,10,100,10000])add(`output-limit/${cases.length}`,'treeView-beta\na',{outputLimit});
const payloadText=await readFile(`${root}/tests/support/inert.ts`,'utf8');const payloadArray=payloadText.slice(payloadText.indexOf('export const PAYLOADS = ')+23);const payloads=new Function('return '+payloadArray.slice(0,payloadArray.indexOf('\n];')+2))();
const security=stripTypeScriptTypes(await readFile(`${root}/tests/render/treeview-security.test.ts`,'utf8'));
const templateStart=security.search(/const TEMPLATES[^=]*=/)+security.match(/const TEMPLATES[^=]*=\s*/)[0].length;const templates=new Function('return '+security.slice(templateStart,security.indexOf('\n];',templateStart)+2))();
for(const payload of payloads){for(const template of templates)add(`hostile/${cases.length}`,template(payload));add(`hostile-options/${cases.length}`,'treeView-beta\na',{idPrefix:payload,fontFamily:payload})}
const stress=stripTypeScriptTypes(await readFile(`${root}/tests/treeview/stress.test.ts`,'utf8'));
const stressStart=stress.search(/const CASES[^=]*=/)+stress.match(/const CASES[^=]*=\s*/)[0].length;const stressCases=new Function('HEAD','N','repeat','return '+stress.slice(stressStart,stress.indexOf('\n];',stressStart)+2))('treeView-beta\n',50000,(count,line)=>Array.from({length:count},(_,i)=>line(i)).join(''));
for(const [name,source]of stressCases)add(`stress/${name}`,source,{limit:1000000});
add('stress/default-limit','treeView-beta\n'+'a\n'.repeat(30000));
function mutator(corpus,fragments,seed){let state=seed>>>0;const rnd=()=>{state=(Math.imul(state,1664525)+1013904223)>>>0;return state/4294967296};const pick=a=>a[Math.floor(rnd()*a.length)];return()=>{let s=pick(corpus);for(let k=1+Math.floor(rnd()*3);k>0;k--){const pos=Math.floor(rnd()*(s.length+1)),v=rnd();if(v<.35)s=s.slice(0,pos)+pick(fragments)+s.slice(pos);else if(v<.6)s=s.slice(0,pos)+s.slice(pos+1+Math.floor(rnd()*3));else if(v<.75){const other=pick(corpus),from=Math.floor(rnd()*other.length);s=s.slice(0,pos)+other.slice(from,from+Math.floor(rnd()*20))+s.slice(pos)}else if(v<.9)s=s.slice(0,pos)+s.slice(pos,pos+Math.floor(rnd()*6))+s.slice(pos);else s=s.slice(0,pos)+pick(fragments)+s.slice(pos+1)}return s}}
const oracle=await readFile(`${root}/tests/treeview/oracle.test.ts`,'utf8');const grammarStart=oracle.indexOf('const FRAGMENTS = ')+18;const grammar=new Function('return '+oracle.slice(grammarStart,oracle.indexOf('\n];',grammarStart)+2))();
function constantArray(name){const match=oracle.match(new RegExp('const '+name+'[^=]*=\\s*'));const start=match.index+match[0].length;return new Function('return '+oracle.slice(start,oracle.indexOf('\n];',start)+2))();}
const grammarCorpus=[...corpus,...constantArray('VALID'),...constantArray('INVALID')];
let surrogateExcluded=0;const surrogateInputExclusions=[];
for(const [campaign,count,fragments,seed]of [['grammar',3000,grammar,7],['security',2000,[...payloads,' :::',' icon(',' ## ','"',"'",'\n','    ','├── ','/'],11]]){const next=mutator(campaign==='grammar'?grammarCorpus:[...corpus],fragments,seed);for(let i=0;i<count;i++){const source=next();if(source.length>4000)continue;if(/[\uD800-\uDBFF](?![\uDC00-\uDFFF])|(?<![\uD800-\uDBFF])[\uDC00-\uDFFF]/u.test(source)){surrogateExcluded++;surrogateInputExclusions.push({id:`${campaign}-mutation/${i}`,utf16:Array.from({length:source.length},(_,j)=>source.charCodeAt(j))});continue;}add(`${campaign}-mutation/${i}`,source)}}
let state=19;const random=()=>{state=(Math.imul(state,1664525)+1013904223)>>>0;return state/4294967296};for(let i=0;i<300;i++){let source='treeView-beta\n';const n=1+Math.floor(random()*30);for(let j=0;j<n;j++)source+=' '.repeat(Math.floor(random()*12))+`node${j}${random()<.3?'/':''}${random()<.3?' :::highlight':''}${random()<.3?' icon(custom:item)':''}${random()<.3?' ## Description':''}\n`;add(`generated-tree/${i}`,source,{config:{treeView:{showIcons:random()<.5,rowIndent:Math.floor(random()*40)}},maxWidth:[undefined,80,160,320][i%4]});}
await writeFile('fixtures/pele/treeview.json.gz',gzipSync(JSON.stringify({revision,provenance,extraction:{version:1,modelProjectionExclusions:projectionExclusions,projectionLimits:{depth:125,bytes:16777216},grammarMutationSeed:7,grammarMutations:3000,securityMutationSeed:11,securityMutations:2000,surrogateExcluded,surrogateInputExclusions,stressBudget:1000000,largeSvgHashThreshold:250000},cases})+'\n',{level:9,mtime:0}));
console.log(`${cases.length} treeview cases; ${cases.filter(c=>c.expected.model).length} models; ${cases.filter(c=>c.expected.result).length} SVG; ${cases.filter(c=>c.expected.error).length} errors; ${projectionExclusions.length} model projection exclusions; ${surrogateExcluded} surrogate exclusions`);
