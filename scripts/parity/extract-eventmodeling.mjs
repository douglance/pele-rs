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
for(const path of ['package.json',...['index','parser','model','render'].map(n=>`src/diagrams/eventmodeling/${n}.ts`),'src/text/label.ts','src/text/metrics.ts','src/text/measurer.ts','src/util/chars.ts','src/svg/theme.ts','src/diagrams/common/fit-width.ts',...(await readdir(`${root}/tests/compat/eventmodeling`)).filter(f=>/\.spec\.[jt]s$/.test(f)).map(f=>`tests/compat/eventmodeling/${f}`),'src/diagrams/common/accDescr.ts','src/diagrams/common/tokens.ts','src/diagrams/common/literal-label.ts','tests/eventmodeling/oracle.test.ts','tests/eventmodeling/stress.test.ts','tests/render/eventmodeling.test.ts','tests/render/eventmodeling-security.test.ts','tests/corpus/eventmodeling-docs.json','tests/support/corpus.ts','tests/support/inert.ts']){
  const data=await readFile(`${root}/${path}`);provenance.push({path,sha256:createHash('sha256').update(data).digest('hex')});
}
function literals(text){const out=[];for(const m of text.matchAll(/`((?:[^`\\]|\\.)*)`|'((?:[^'\\\n]|\\.)*)'|"((?:[^"\\\n]|\\.)*)"/g)){const raw=m[1]??m[2]??m[3];try{out.push(new Function('return `'+raw.replace(/`/g,'\\`').replace(/\$\{/g,'\\${')+'`')())}catch{}}return out}
const corpus=new Set();
for(const file of(await readdir(`${root}/tests/compat/eventmodeling`)).filter(f=>/\.spec\.[jt]s$/.test(f)))for(const source of literals(await readFile(`${root}/tests/compat/eventmodeling/${file}`,'utf8')))if(/eventmodeling/.test(source))corpus.add(source);
for(const source of JSON.parse(await readFile(`${root}/tests/corpus/eventmodeling-docs.json`,'utf8')))corpus.add(source);
const seeds=new Set(corpus);
for(const file of(await readdir(`${root}/tests/compat/eventmodeling`)).filter(f=>/\.test\.[jt]s$/.test(f)))for(const source of literals(await readFile(`${root}/tests/compat/eventmodeling/${file}`,'utf8')))if(/eventmodeling/.test(source))seeds.add(source);
for(const path of ['tests/eventmodeling/oracle.test.ts','tests/render/eventmodeling.test.ts','tests/render/eventmodeling-security.test.ts'])for(const source of literals(await readFile(`${root}/${path}`,'utf8')))if(/eventmodeling/.test(source))seeds.add(source);
const {REGULAR,BOLD}=await import(`data:text/javascript,${encodeURIComponent(stripTypeScriptTypes(await readFile(`${root}/src/text/metrics.ts`,'utf8')))}`);
const tables=[REGULAR,BOLD].map(p=>Array.from({length:95},(_,i)=>parseInt(p.slice(i*2,i*2+2),36)));
const metrics={width(text,size,style){const table=tables[style&1?1:0],mono=!!(style&4);let units=0;for(let i=0;i<text.length;i++){const c=text.charCodeAt(i);if(c<32)continue;if(c<127)units+=mono?600:table[c-32];else if(c>=0x300&&c<=0x36f||c>=0x200b&&c<=0x200f||c>=0xdc00&&c<=0xdfff||c>=0xfe00&&c<=0xfe0f||c===0xfeff)continue;else if(c>=0x1100&&c<=0x115f||c>=0x2e80&&c<=0xa4cf||c>=0xac00&&c<=0xd7a3||c>=0xf900&&c<=0xfaff||c>=0xfe30&&c<=0xfe4f||c>=0xff00&&c<=0xff60||c>=0xffe0&&c<=0xffe6||c>=0xd800&&c<=0xdbff)units+=mono?1200:1000;else units+=mono?600:580}return units*size/1000}};
const modelKeys=new Set(['type','title','accTitle','accDescr','boxes','lanes','relations','notes','specifications','entities']);
const normalize=v=>JSON.parse(JSON.stringify(v,(_,v)=>v instanceof Map?[...v]:v));
const cases=[];
const projectionExclusions=[];
const utf16OutputExclusions=[];
function projection(model){
 const out={};for(const key of modelKeys)if(Object.hasOwn(model,key))out[key]=model[key];
 const stack=[[out,0]];let visits=0;
 while(stack.length){const [v,d]=stack.pop();if(++visits>1000000||d>125)return null;if(v&&typeof v==='object'){for(const value of v instanceof Map?[...v.values()]:Object.values(v))stack.push([value,d+1]);}}
 const result=normalize(out);return Buffer.byteLength(JSON.stringify(result))>16*1024*1024?null:result;
}
function add(id,source,options={},host={}){
 let expected,model,projectionLimit=false;
 try{const parsed=parse(source,options);model=projection(parsed);projectionLimit=model===null;if(projectionLimit){projectionExclusions.push(id);host.modelProjection='excluded';}const resources={measurer:host.measure==='fixed'?{width:(text,size,style)=>text.length*size*(style&1?.71:.6)}:metrics,...(host.icons?{icons:n=>host.icons[n]}:{})};const result=render(parsed,{...options,...resources});assert.deepEqual(result,render(source,{...options,...resources}));if(/[\uD800-\uDBFF](?![\uDC00-\uDFFF])|(?<![\uD800-\uDBFF])[\uDC00-\uDFFF]/u.test(result.svg)){utf16OutputExclusions.push({id,source,options,host,model,width:result.width,height:result.height,svgUtf16:Array.from({length:result.svg.length},(_,j)=>result.svg.charCodeAt(j))});return;}if(Buffer.byteLength(result.svg)>250000){result.svgSha256=createHash('sha256').update(result.svg).digest('hex');delete result.svg}expected={...(projectionLimit?{modelProjectionLimit:true}:{model}),result}}
 catch(e){if(e.name!=='PeleError')throw e;expected={...(projectionLimit?{modelProjectionLimit:true}:model?{model}:{}),error:{name:e.name,message:e.message,code:e.code,type:e.type,line:e.line,column:e.column,snippet:e.snippet}}}
 cases.push({id,source,options,host,expected});
}
[...seeds].forEach((source,i)=>{add(`upstream/${i}`,source);add(`upstream-newline/${i}`,source+'\n')});
const basic='eventmodeling title Checkout\ntf 01 ui Cart.View\ntf 02 cmd Cart.AddItem { productId: 7 }\ntf 03 evt Cart.ItemAdded [[Payload]]\ntf 04 rmo Cart.Items ->> 01 ->> 03\nrf 05 processor Other.Index ->> 03\nentity Item\ndata Payload `json` {\n  a: 1\n  %% retained\n  nested: true\n}\nnote 03 `md` {\n  First line\n  Second line\n}\ngwt 02 given evt ItemAdded when cmd AddItem then rmo Items\n';
for(const maxWidth of [undefined,1,80,120,240,320,639,640,800,1200])for(const autoDirection of [undefined,true,false])for(const fontSize of [8,16,19.5,32])for(const padding of [0,8,19.5])add(`layout/${cases.length}`,basic,{maxWidth,autoDirection,fontSize,padding});
for(const directionBreakpoint of [-1,0,160,320,640,1000])for(const maxWidth of [80,160,320,640,800])add(`breakpoint/${cases.length}`,basic,{maxWidth,directionBreakpoint});
for(const entityKind of ['ui','cmd','command','evt','event','rmo','readmodel','pcr','processor'])for(const namespace of ['','Cart.','a.b.'])for(const maxWidth of [undefined,80,320])add(`kinds/${cases.length}`,`eventmodeling\ntf 01 ${entityKind} ${namespace}A\ntf 02 evt Other.B\ntf 03 ${entityKind} ${namespace}C ->> 01 ->> 02\nrf 04 ${entityKind} ${namespace}D`,{maxWidth});
for(const refs of ['',' ->> 01',' ->> 03',' ->> 04 ->> 02 ->> 02',' ->> 01 ->> 04 ->> 09',' ->> 03 ->> 02 ->> 01'])for(const reset of ['tf','rf'])for(const maxWidth of [undefined,80,160,320])add(`relations/${cases.length}`,`eventmodeling\ntf 01 evt A\ntf 02 ui B\n${reset} 03 cmd C${refs}\ntf 04 rmo D ->> 01 ->> 02 ->> 03\ntf 05 cmd E ->> 01 ->> 02\ntf 06 evt F ->> 01 ->> 03\ntf 07 ui G ->> 03`,{maxWidth});
for(const type of ['','`json`','`jsobj`','`figma`','`salt`','`uri`','`md`','`html`','`text`'])for(const data of ['{ x: 1 }','"quoted text"',"'single text'",'{ { nested } }','{}','""'])for(const maxWidth of [undefined,120,320])add(`data/${cases.length}`,`eventmodeling\ntf 01 cmd A ${type}${data}\ntf 02 evt B [[Payload]] { Ignored }\ndata Payload ${type} {\n  first\n    second\n\n  third\n}\nnote 01 ${type} {\n  Note\n}\n`,{maxWidth});
for(const text of ['Plain','<script>alert(1)</script>','A<br/>B\\nC','&#x1f642; #35; #amp;','**literal** _under_score_','中文 🙂 é','__proto__','LongUnbreakableWord'.repeat(100),'fa:fa-user Text'])for(const maxWidth of [undefined,80,320])add(`labels/${cases.length}`,`eventmodeling title ${text}\ntf 01 evt A { ${text} }\nnote 01 {\n  ${text}\n}\n`,{maxWidth},{icons:{'fa:fa-user':'<path d="M0 0L24 24"/>'}});
for(const length of [46,47,48,49,78,79,80,81])for(const maxWidth of [undefined,80,320])add(`utf16-boundary/${length}/${maxWidth??'default'}`,`eventmodeling\ntf 01 evt A { ${'a'.repeat(length)}🙂Tail }\nnote 01 {\n ${'a'.repeat(length)}🙂Tail\n}\n`,{maxWidth});
for(const key of ['__proto__','constructor','toString','hasOwnProperty','valueOf'])add(`object-keys/${cases.length}`,`eventmodeling\ntf 01 cmd ${key}.${key} [[${key}]]\ntf 02 evt ${key}\ndata ${key} {\n x\n}\nentity ${key}\ngwt 01 given evt ${key} then evt ${key}\n`);
for(const source of ['eventmodeling','eventmodeling ','eventmodeling\ntitle Later','eventmodeling title','eventmodeling accTitle:','eventmodeling accDescr:','eventmodeling accDescr {}','eventmodeling\ntf 01 evt A\ntf 01 evt B','eventmodeling\ntf 1234 evt A','eventmodeling\nentity A.B.C','eventmodeling\n/* comment */\ntf 01 evt A // comment\n','eventmodeling\ndata D {\n}\n','eventmodeling\ndata D {\n}\n}\n','eventmodeling\ndata D {\n}\ndata D {\nsecond\n}\ntf 01 evt A [[D]]\n'])add(`boundary/${cases.length}`,source);
for(const fontFamily of [undefined,'Arial','monospace','"A & B", sans-serif','Arial); url(https://evil.test)'])for(const responsive of [true,false])add(`options/${cases.length}`,basic,{fontFamily,responsive,idPrefix:'a"&b',fontFamilyMono:'Mono'},{measure:'fixed'});
for(const limit of [0,12,13,100])add(`source-limit/${cases.length}`,'eventmodeling',{limit});
for(const outputLimit of [0,10,100,10000])add(`output-limit/${cases.length}`,'eventmodeling\ntf 01 evt A',{outputLimit});
const payloadText=await readFile(`${root}/tests/support/inert.ts`,'utf8');const payloadArray=payloadText.slice(payloadText.indexOf('export const PAYLOADS = ')+23);const payloads=new Function('return '+payloadArray.slice(0,payloadArray.indexOf('\n];')+2))();
const security=stripTypeScriptTypes(await readFile(`${root}/tests/render/eventmodeling-security.test.ts`,'utf8'));
const templateStart=security.search(/const TEMPLATES[^=]*=/)+security.match(/const TEMPLATES[^=]*=\s*/)[0].length;const templates=new Function('line','return '+security.slice(templateStart,security.indexOf('\n];',templateStart)+2))(p=>p.replace(/[\n\r]/g,' '));
for(const payload of payloads){for(const template of templates)add(`hostile/${cases.length}`,template(payload));add(`hostile-options/${cases.length}`,'eventmodeling\ntf 01 evt A',{idPrefix:payload,fontFamily:payload})}
const stress=stripTypeScriptTypes(await readFile(`${root}/tests/eventmodeling/stress.test.ts`,'utf8'));
const stressStart=stress.search(/const CASES[^=]*=/)+stress.match(/const CASES[^=]*=\s*/)[0].length;const stressCases=new Function('HEAD','N','repeat','id','KINDS','return '+stress.slice(stressStart,stress.indexOf('\n];',stressStart)+2))('eventmodeling\n',50000,(count,line)=>Array.from({length:count},(_,i)=>line(i)).join(''),i=>String(i%1000).padStart(3,'0'),['ui','cmd','evt','rmo','pcr']);
for(const [name,source]of stressCases)add(`stress/${name}`,source,{limit:1000000});
add('stress/default-limit','eventmodeling\n'+'tf 01 evt A\n'.repeat(5000));
function mutator(corpus,fragments,seed){let state=seed>>>0;const rnd=()=>{state=(Math.imul(state,1664525)+1013904223)>>>0;return state/4294967296};const pick=a=>a[Math.floor(rnd()*a.length)];return()=>{let s=pick(corpus);for(let k=1+Math.floor(rnd()*3);k>0;k--){const pos=Math.floor(rnd()*(s.length+1)),v=rnd();if(v<.35)s=s.slice(0,pos)+pick(fragments)+s.slice(pos);else if(v<.6)s=s.slice(0,pos)+s.slice(pos+1+Math.floor(rnd()*3));else if(v<.75){const other=pick(corpus),from=Math.floor(rnd()*other.length);s=s.slice(0,pos)+other.slice(from,from+Math.floor(rnd()*20))+s.slice(pos)}else if(v<.9)s=s.slice(0,pos)+s.slice(pos,pos+Math.floor(rnd()*6))+s.slice(pos);else s=s.slice(0,pos)+pick(fragments)+s.slice(pos+1)}return s}}
const oracle=await readFile(`${root}/tests/eventmodeling/oracle.test.ts`,'utf8');const grammarStart=oracle.indexOf('const FRAGMENTS = ')+18;const grammar=new Function('return '+oracle.slice(grammarStart,oracle.indexOf('\n];',grammarStart)+2))();
function constantArray(name){const match=oracle.match(new RegExp('const '+name+'[^=]*=\\s*'));const start=match.index+match[0].length;return new Function('return '+oracle.slice(start,oracle.indexOf('\n];',start)+2))();}
const grammarCorpus=[...corpus,...constantArray('VALID'),...constantArray('INVALID')];
let surrogateExcluded=0;const surrogateInputExclusions=[];
for(const [campaign,count,fragments,seed]of [['grammar',3000,grammar,7],['security',2000,[...payloads,'"',"'",'\n','{','}',' ->> 01','[[',']]','tf 09 evt Z','note 01 {\n','\n}\n'],19]]){const next=mutator(campaign==='grammar'?grammarCorpus:[...corpus],fragments,seed);for(let i=0;i<count;i++){const source=next();if(source.length>4000)continue;if(/[\uD800-\uDBFF](?![\uDC00-\uDFFF])|(?<![\uD800-\uDBFF])[\uDC00-\uDFFF]/u.test(source)){surrogateExcluded++;surrogateInputExclusions.push({id:`${campaign}-mutation/${i}`,utf16:Array.from({length:source.length},(_,j)=>source.charCodeAt(j))});continue;}add(`${campaign}-mutation/${i}`,source)}}
let state=29;const random=()=>{state=(Math.imul(state,1664525)+1013904223)>>>0;return state/4294967296};for(let i=0;i<300;i++){let source='eventmodeling\n';const n=2+Math.floor(random()*20);for(let j=0;j<n;j++){source+=`${random()<.15?'rf':'tf'} ${String(j).padStart(3,'0')} ${['ui','cmd','evt','rmo','pcr'][Math.floor(random()*5)]} ${random()<.6?'N'+Math.floor(random()*4)+'.':''}E${j}`;if(random()<.4)for(let k=0;k<3;k++)source+=' ->> '+String(Math.floor(random()*n)).padStart(3,'0');source+=(random()<.3?' { Data: '+j+' }':'')+'\n';}add(`generated-model/${i}`,source,{maxWidth:[undefined,80,160,320][i%4]});}
await writeFile('fixtures/pele/eventmodeling.json.gz',gzipSync(JSON.stringify({revision,provenance,extraction:{version:1,modelProjectionExclusions:projectionExclusions,projectionLimits:{depth:125,bytes:16777216},grammarMutationSeed:7,grammarMutations:3000,securityMutationSeed:19,securityMutations:2000,surrogateExcluded,surrogateInputExclusions,utf16OutputExclusions,stressBudget:1000000,largeSvgHashThreshold:250000},cases})+'\n',{level:9,mtime:0}));
console.log(`${cases.length} eventmodeling cases; ${cases.filter(c=>c.expected.model).length} models; ${cases.filter(c=>c.expected.result).length} SVG; ${cases.filter(c=>c.expected.error).length} errors; ${projectionExclusions.length} model projection exclusions; ${surrogateExcluded} surrogate input exclusions; ${utf16OutputExclusions.length} UTF16 output exclusions`);
