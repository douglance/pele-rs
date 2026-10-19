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
for(const path of ['package.json',...['index','db','lexer','parser','types','render'].map(n=>`src/diagrams/ishikawa/${n}.ts`),'src/text/label.ts','src/text/metrics.ts','src/text/measurer.ts','src/util/chars.ts','src/svg/theme.ts','src/diagrams/common/fit-width.ts',...(await readdir(`${root}/tests/compat/ishikawa`)).filter(f=>/\.spec\.[jt]s$/.test(f)).map(f=>`tests/compat/ishikawa/${f}`),'src/diagrams/common/accDescr.ts','src/diagrams/common/tokens.ts','src/diagrams/common/literal-label.ts','tests/ishikawa/oracle.test.ts','tests/ishikawa/stress.test.ts','tests/render/ishikawa.test.ts','tests/render/ishikawa-security.test.ts','tests/corpus/ishikawa-docs.json','tests/support/corpus.ts','tests/support/inert.ts']){
  const data=await readFile(`${root}/${path}`);provenance.push({path,sha256:createHash('sha256').update(data).digest('hex')});
}
function literals(text){const out=[];for(const m of text.matchAll(/`((?:[^`\\]|\\.)*)`|'((?:[^'\\\n]|\\.)*)'|"((?:[^"\\\n]|\\.)*)"/g)){const raw=m[1]??m[2]??m[3];try{out.push(new Function('return `'+raw.replace(/`/g,'\\`').replace(/\$\{/g,'\\${')+'`')())}catch{}}return out}
const corpus=new Set();
for(const file of(await readdir(`${root}/tests/compat/ishikawa`)).filter(f=>/\.spec\.[jt]s$/.test(f)))for(const source of literals(await readFile(`${root}/tests/compat/ishikawa/${file}`,'utf8')))if(/ishikawa/.test(source))corpus.add(source);
for(const source of JSON.parse(await readFile(`${root}/tests/corpus/ishikawa-docs.json`,'utf8')))corpus.add(source);
const seeds=new Set(corpus);
for(const file of(await readdir(`${root}/tests/compat/ishikawa`)).filter(f=>/\.test\.[jt]s$/.test(f)))for(const source of literals(await readFile(`${root}/tests/compat/ishikawa/${file}`,'utf8')))if(/ishikawa/.test(source))seeds.add(source);
for(const path of ['tests/ishikawa/oracle.test.ts','tests/render/ishikawa.test.ts','tests/render/ishikawa-security.test.ts'])for(const source of literals(await readFile(`${root}/${path}`,'utf8')))if(/ishikawa/.test(source))seeds.add(source);
const {REGULAR,BOLD}=await import(`data:text/javascript,${encodeURIComponent(stripTypeScriptTypes(await readFile(`${root}/src/text/metrics.ts`,'utf8')))}`);
const tables=[REGULAR,BOLD].map(p=>Array.from({length:95},(_,i)=>parseInt(p.slice(i*2,i*2+2),36)));
const metrics={width(text,size,style){const table=tables[style&1?1:0],mono=!!(style&4);let units=0;for(let i=0;i<text.length;i++){const c=text.charCodeAt(i);if(c<32)continue;if(c<127)units+=mono?600:table[c-32];else if(c>=0x300&&c<=0x36f||c>=0x200b&&c<=0x200f||c>=0xdc00&&c<=0xdfff||c>=0xfe00&&c<=0xfe0f||c===0xfeff)continue;else if(c>=0x1100&&c<=0x115f||c>=0x2e80&&c<=0xa4cf||c>=0xac00&&c<=0xd7a3||c>=0xf900&&c<=0xfaff||c>=0xfe30&&c<=0xfe4f||c>=0xff00&&c<=0xff60||c>=0xffe0&&c<=0xffe6||c>=0xd800&&c<=0xdbff)units+=mono?1200:1000;else units+=mono?600:580}return units*size/1000}};
const modelKeys=new Set(['type','title','root']);
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
// Every literal passed to the upstream svgOf helper is a complete body, prefixed there.
const renderTests=await readFile(`${root}/tests/render/ishikawa.test.ts`,'utf8');
for(const match of renderTests.matchAll(/svgOf\(\s*('(?:[^'\\\n]|\\.)*'|"(?:[^"\\\n]|\\.)*"|`(?:[^`\\]|\\.)*`)/g))for(const body of literals(match[1]))add(`render-helper/${cases.length}`,'ishikawa-beta\n'+body);
const basic='---\ntitle: Root causes\n---\nishikawa-beta\n  Delayed delivery\n    People\n      Limited training\n        Few examples\n          Missing documentation\n        No pairing\n      Short staffing\n    Process\n      Too many approvals\n      Unclear goals\n    Equipment\n      Broken machine\n    Material\n      Delayed parts\n    Environment\n      Noisy workspace\n    Measurement\n      Missing metrics\n';
for(const maxWidth of [undefined,1,80,120,240,320,639,640,800,1200])for(const autoDirection of [undefined,true,false])for(const fontSize of [8,16,19.5,32])for(const padding of [0,8,19.5])add(`layout/${cases.length}`,basic,{maxWidth,autoDirection,fontSize,padding});
for(const directionBreakpoint of [-1,0,160,320,640,1000])for(const maxWidth of [80,160,320,640,800])add(`breakpoint/${cases.length}`,basic,{maxWidth,directionBreakpoint});
for(const text of ['Plain','<script>alert(1)</script>','A<br/>B\\nC','&#x1f642; #35; #amp;','**literal** _under_score_','中文 🙂 é','__proto__','LongUnbreakableWord'.repeat(100),'fa:fa-user Text'])for(const maxWidth of [undefined,80,320])add(`labels/${cases.length}`,`---\ntitle: ${JSON.stringify(text)}\n---\nishikawa-beta\n  ${text}\n    ${text}\n      ${text}\n        ${text}\n`,{maxWidth},{icons:{'fa:fa-user':'<path d="M0 0L24 24"/>'}});
for(const fontFamily of [undefined,'Arial','monospace','"A & B", sans-serif','Arial); url(https://evil.test)'])for(const responsive of [true,false])add(`options/${cases.length}`,basic,{fontFamily,responsive,idPrefix:'a"&b',fontFamilyMono:'Mono'},{measure:'fixed'});
for(const limit of [0,12,13,100])add(`source-limit/${cases.length}`,'ishikawa-beta',{limit});
for(const outputLimit of [0,10,100,10000])add(`output-limit/${cases.length}`,'ishikawa-beta\nEffect\n  Cause',{outputLimit});
const payloadText=await readFile(`${root}/tests/support/inert.ts`,'utf8');const payloadArray=payloadText.slice(payloadText.indexOf('export const PAYLOADS = ')+23);const payloads=new Function('return '+payloadArray.slice(0,payloadArray.indexOf('\n];')+2))();
function constantArray(text,name,...args){const match=text.match(new RegExp('const '+name+'[^=]*=\\s*'));const start=match.index+match[0].length;return new Function(...args.map(a=>a[0]),'return '+text.slice(start,text.indexOf('\n];',start)+2))(...args.map(a=>a[1]));}
const security=stripTypeScriptTypes(await readFile(`${root}/tests/render/ishikawa-security.test.ts`,'utf8'));
const templates=constantArray(security,'TEMPLATES',['I','ishikawa-beta\n']);
for(const payload of payloads){for(const template of templates)add(`hostile/${cases.length}`,template(payload));add(`hostile-options/${cases.length}`,'ishikawa-beta\n  Effect\n    Category\n      Cause',{idPrefix:payload,fontFamily:payload})}
// Execute every actual stress closure, capturing its source through the public API.
const stress=stripTypeScriptTypes(await readFile(`${root}/tests/ishikawa/stress.test.ts`,'utf8'));
let activeStress='';const capture=source=>{add(`stress/${activeStress}`,source,{limit:12000000});};
const repeat=(count,line)=>Array.from({length:count},(_,i)=>line(i)).join('');
for(const [name,run]of constantArray(stress,'CASES',['render',capture],['parse',capture],['tokenize',capture],['I','ishikawa-beta\n'],['N',50000],['repeat',repeat],['big',{}])){activeStress=name;run()}
add('stress/finite-400-deep','ishikawa-beta\n'+repeat(400,i=>' '.repeat(i+1)+`n${i}\n`),{limit:12000000});
add('stress/default-limit','ishikawa-beta\n'+'  cause\n'.repeat(8000));
function random(seed){let state=seed>>>0;return()=>{state=(Math.imul(state,1664525)+1013904223)>>>0;return state/4294967296}}
function mutator(corpus,fragments,seed){const rnd=random(seed);const pick=a=>a[Math.floor(rnd()*a.length)];return()=>{let s=pick(corpus);for(let k=1+Math.floor(rnd()*3);k>0;k--){const pos=Math.floor(rnd()*(s.length+1)),v=rnd();if(v<.35)s=s.slice(0,pos)+pick(fragments)+s.slice(pos);else if(v<.6)s=s.slice(0,pos)+s.slice(pos+1+Math.floor(rnd()*3));else if(v<.75){const other=pick(corpus),from=Math.floor(rnd()*other.length);s=s.slice(0,pos)+other.slice(from,from+Math.floor(rnd()*20))+s.slice(pos)}else if(v<.9)s=s.slice(0,pos)+s.slice(pos,pos+Math.floor(rnd()*6))+s.slice(pos);else s=s.slice(0,pos)+pick(fragments)+s.slice(pos+1)}return s}}
const oracle=await readFile(`${root}/tests/ishikawa/oracle.test.ts`,'utf8');
const grammar=constantArray(oracle,'FRAGMENTS');
const grammarCorpus=[...new Set([...corpus,...constantArray(oracle,'SEEDS')])];
let surrogateExcluded=0;const surrogateInputExclusions=[];
function generated(id,source){if(source.length>4000)return;if(/[\uD800-\uDBFF](?![\uDC00-\uDFFF])|(?<![\uD800-\uDBFF])[\uDC00-\uDFFF]/u.test(source)){surrogateExcluded++;surrogateInputExclusions.push({id,utf16:Array.from({length:source.length},(_,j)=>source.charCodeAt(j))});return;}add(id,source)}
for(const [campaign,count,fragments,seed]of [['grammar',5000,grammar,7],['security',4000,[...payloads,'\n','\n  ','\n      ','\t',' ','%%','<br>','#35;','ishikawa'],3]]){const next=mutator(campaign==='grammar'?grammarCorpus:[...corpus].filter(src=>/^\s*ishikawa/i.test(src)),fragments,seed);for(let i=0;i<count;i++)generated(`${campaign}-mutation/${i}`,next())}
{
const rnd=random(8),pick=()=>grammar[Math.floor(rnd()*grammar.length)],openers=['ishikawa-beta\n','ishikawa-beta\nEffect\n','ishikawa\n  E\n','ishikawa-beta','%% c\nishikawa-beta\n','\n',''];
for(let i=0;i<5000;i++){let source=openers[Math.floor(rnd()*openers.length)];for(let k=1+Math.floor(rnd()*8);k>0;k--)source+=pick();generated(`random-tokens/${i}`,source)}
}
{
const rnd=random(9),pick=list=>list[Math.floor(rnd()*list.length)];
for(let i=0;i<5000;i++){let source=pick(['ishikawa-beta','ishikawa','Ishikawa-BETA'])+pick(['\n','\n','\r\n',' ','\n\n']);for(let k=Math.floor(rnd()*12);k>0;k--)source+=pick(['',' ','  ','    ','      ','\t','\t\t',' \t '])+pick(['Cause','A b','x','%% note','','ishikawa'])+pick(['\n','\n','\n','\r\n','\n\n',' \n']);generated(`random-outlines/${i}`,source)}
}
{
const rnd=random(29);for(let i=0;i<300;i++){let source='ishikawa-beta\n  E\n';for(let c=0;c<1+Math.floor(rnd()*12);c++){source+=`    Category ${c}\n`;for(let k=0;k<1+Math.floor(rnd()*6);k++){source+=`      Cause ${c}.${k}\n`;for(let s=0;s<Math.floor(rnd()*5);s++)source+=`        Sub ${c}.${k}.${s}\n          Detail\n`;}}add(`generated-model/${i}`,source,{maxWidth:[undefined,80,160,320][i%4]})}
}
await writeFile('fixtures/pele/ishikawa.json.gz',gzipSync(JSON.stringify({revision,provenance,extraction:{version:1,modelProjectionFields:[...modelKeys],privateDiagnosticsExcluded:['stack','baseLevel'],modelProjectionExclusions:projectionExclusions,projectionLimits:{depth:125,bytes:16777216},grammarMutationSeed:7,grammarMutations:5000,randomTokenSeed:8,randomTokenInputs:5000,randomOutlineSeed:9,randomOutlines:5000,securityMutationSeed:3,securityMutations:4000,surrogateExcluded,surrogateInputExclusions,utf16OutputExclusions,stressBudget:12000000,largeSvgHashThreshold:250000},cases})+'\n',{level:9,mtime:0}));
console.log(`${cases.length} ishikawa cases; ${cases.filter(c=>c.expected.model).length} models; ${cases.filter(c=>c.expected.result).length} SVG; ${cases.filter(c=>c.expected.error).length} errors; ${projectionExclusions.length} model projection exclusions; ${surrogateExcluded} surrogate input exclusions; ${utf16OutputExclusions.length} UTF16 output exclusions`);
