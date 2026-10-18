// Development-only public API oracle. Pinned Pele JavaScript never ships in consumers.
import {readFile,readdir,writeFile} from 'node:fs/promises';
import {execFileSync} from 'node:child_process';
import {stripTypeScriptTypes} from 'node:module';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
import {parse,render} from 'pele';
const root=process.argv[2]??'/tmp/pele-upstream-458322a5';
const revision='458322a5b8abb1ae57ddf127c32017ab9fb14a92';
assert.equal(execFileSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).trim(),revision);
execFileSync('git',['diff','--quiet',revision,'--','src','tests'],{cwd:root});
const provenance=[];
for(const path of ['package.json','src/diagrams/info/index.ts','src/diagrams/info/model.ts','src/diagrams/info/parser.ts','src/diagrams/info/render.ts','src/diagrams/common/tokens.ts','src/text/label.ts','src/text/metrics.ts','src/svg/builder.ts','src/svg/root.ts','tests/compat/info/info.spec.ts','tests/compat/info/info.test.ts','tests/info/oracle.test.ts','tests/info/stress.test.ts','tests/render/info.test.ts','tests/render/info-security.test.ts','tests/corpus/info-docs.json','tests/support/corpus.ts','tests/support/inert.ts']){
  const data=await readFile(`${root}/${path}`);provenance.push({path,sha256:createHash('sha256').update(data).digest('hex')});
}
function literals(text){const out=[];for(const m of text.matchAll(/`((?:[^`\\]|\\.)*)`|'((?:[^'\\\n]|\\.)*)'|"((?:[^"\\\n]|\\.)*)"/g)){const raw=m[1]??m[2]??m[3];try{out.push(new Function('return `'+raw.replace(/`/g,'\\`').replace(/\$\{/g,'\\${')+'`')())}catch{}}return out}
const corpus=new Set();
for(const file of(await readdir(`${root}/tests/compat/info`)).filter(f=>/\.spec\.[jt]s$/.test(f)))for(const source of literals(await readFile(`${root}/tests/compat/info/${file}`,'utf8')))if(/info/.test(source))corpus.add(source);
for(const source of JSON.parse(await readFile(`${root}/tests/corpus/info-docs.json`,'utf8')))corpus.add(source);
const seeds=new Set(corpus);
for(const path of ['tests/info/oracle.test.ts','tests/render/info.test.ts','tests/compat/info/info.test.ts'])for(const source of literals(await readFile(`${root}/${path}`,'utf8')))if(/info/.test(source))seeds.add(source);
const {REGULAR,BOLD}=await import(`data:text/javascript,${encodeURIComponent(stripTypeScriptTypes(await readFile(`${root}/src/text/metrics.ts`,'utf8')))}`);
const tables=[REGULAR,BOLD].map(p=>Array.from({length:95},(_,i)=>parseInt(p.slice(i*2,i*2+2),36)));
const metrics={width(text,size,style){const table=tables[style&1?1:0],mono=!!(style&4);let units=0;for(let i=0;i<text.length;i++){const c=text.charCodeAt(i);if(c<32)continue;if(c<127)units+=mono?600:table[c-32];else if(c>=0x300&&c<=0x36f||c>=0x200b&&c<=0x200f||c>=0xdc00&&c<=0xdfff||c>=0xfe00&&c<=0xfe0f||c===0xfeff)continue;else if(c>=0x1100&&c<=0x115f||c>=0x2e80&&c<=0xa4cf||c>=0xac00&&c<=0xd7a3||c>=0xf900&&c<=0xfaff||c>=0xfe30&&c<=0xfe4f||c>=0xff00&&c<=0xff60||c>=0xffe0&&c<=0xffe6||c>=0xd800&&c<=0xdbff)units+=mono?1200:1000;else units+=mono?600:580}return units*size/1000}};
const normalize=v=>JSON.parse(JSON.stringify(v,(_,v)=>v instanceof Map?[...v]:v));
const cases=[];
function add(id,source,options={},host={}){
 let expected,model;
 try{model=normalize(parse(source,options));const resources={measurer:host.measure==='fixed'?{width:(text,size,style)=>text.length*size*(style&1?.71:.6)}:metrics,...(host.icons?{icons:n=>host.icons[n]}:{})};const result=render(source,{...options,...resources});assert.deepEqual(result,render(parse(source,options),{...options,...resources}));if(Buffer.byteLength(result.svg)>250000){result.svgSha256=createHash('sha256').update(result.svg).digest('hex');delete result.svg}expected={model,result}}
 catch(e){if(e.name!=='PeleError')throw e;expected={...(model?{model}:{}),error:{name:e.name,message:e.message,code:e.code,type:e.type,line:e.line,column:e.column,snippet:e.snippet}}}
 cases.push({id,source,options,host,expected});
}
[...seeds].forEach((source,i)=>add(`upstream/${i}`,source));
for(const head of ['info','info showInfo','info\nshowInfo','info showInfo title Caption','Info','INFO','infox','showInfo','info showInfo showInfo','info\ntitle a\nshowInfo'])for(const suffix of ['','\n','\r\n','\naccTitle: A\naccDescr: D','\naccDescr { first\n second }','\n%% comment'])add(`grammar/${cases.length}`,head+suffix);
for(const white of [' ','\t','\r','\n','\r\n','\u000b','\u000c','\u00a0','\u2028','\u2029','\ufeff','\u2000'])for(const keyword of ['info','showInfo','title','accTitle:','accDescr:'])for(const ending of ['','%%comment','\n'])add(`whitespace/${cases.length}`,(keyword==='info'?'':keyword==='showInfo'?'info ':'info\n')+keyword+white+(keyword==='info'?'showInfo':keyword==='showInfo'?'':'Next')+ending);
for(const label of ['Plain',' <strong>Bold <em>Italic</em></strong> ','A<br/>B\\nC','&#x1f642; #35; #amp;','fa:fa-user Person','**literal** _under_score_','<unknown>Words</unknown>','    ','中文 🙂 é','__proto__','VeryLongSingleWordWithoutBreaks'.repeat(200),'a  b\t c'])for(const fontSize of [0,8,16,19.5,40])for(const padding of [-5,0,8,19.5])add(`labels/${cases.length}`,'info\ntitle '+label,{fontSize,padding},{icons:{'fa:fa-user':'<path d="M0 0L4 4"/>'}});
for(const config of [{},{info:{}},{info:{padding:100,fontSize:99}},{info:null},{theme:'dark'},{themeVariables:{fontSize:24,fontFamily:'Courier New'}}])for(const maxWidth of [1,80,320,640,1200])for(const autoDirection of [true,false])add(`config/${cases.length}`,'---\ntitle: Fallback\n---\ninfo\ntitle Own\naccTitle: Accessible\naccDescr: Detail',{config,maxWidth,autoDirection});
for(const source of ['info','info title','info\naccTitle:','info\naccDescr:','info\naccDescr {}','info\naccDescr {  a\n\n\n b   \n}','info\naccDescr {first\u2028 second\u2029 third}','info\ntitle One\ntitle','---\ntitle: Fallback\n---\ninfo\ntitle','info showInfo title a\naccTitle: b','info\ntitle Prefix%% hidden','info\naccDescr { a\n b } title c'])add(`boundary/${cases.length}`,source);
for(const fontFamily of [undefined,'Arial','"A & B", sans-serif','Arial); url(https://evil.test)','中文, sans-serif','monospace'])for(const responsive of [true,false])for(const fontSize of [8,16,19.5,32])add(`options/${cases.length}`,'info\ntitle <b>Bold</b> <i>Italic</i><br>中文 🙂\naccTitle: Named\naccDescr: A & B',{fontFamily,fontFamilyMono:'"Mono", monospace',idPrefix:'a"&b',responsive,fontSize},{measure:'fixed'});
for(const limit of [0,3,4,5,50])add(`source-limit/${cases.length}`,'info',{limit});
for(const outputLimit of [0,10,100,10000])add(`output-limit/${cases.length}`,'info title Caption',{outputLimit});
const payloadText=await readFile(`${root}/tests/support/inert.ts`,'utf8');const payloadArray=payloadText.slice(payloadText.indexOf('export const PAYLOADS = ')+23);const payloadEnd=payloadArray.indexOf('\n];');const payloads=new Function('return '+payloadArray.slice(0,payloadEnd+2))();
const templates=[p=>`info\ntitle ${p}`,p=>`info showInfo title ${p}`,p=>`info\naccTitle: ${p}\naccDescr: ${p}`,p=>`info\naccDescr {\n ${p}\n }`,p=>`info %% ${p}`,p=>`info ${p}`,p=>`---\ntitle: "${p}"\n---\ninfo`,p=>`%%{init: {"themeVariables": {"fontFamily": "${p}"}}}%%\ninfo`];
for(const payload of payloads){for(const template of templates)add(`hostile/${cases.length}`,template(payload));add(`hostile-options/${cases.length}`,'info\naccTitle: t',{idPrefix:payload,fontFamily:payload})}
const N=50000;
const stress=[['title','info\ntitle '+'word '.repeat(N/5)],['titles','info\n'+'title t\n'.repeat(N/8)],['title-entities','info\ntitle '+'#35;&amp;<br>'.repeat(N/13)],['blank-lines','info'+'\n '.repeat(N/2)+'showInfo'],['spaces','info'+' '.repeat(N)+'showInfo'],['comments','info\n'+'%% c\n'.repeat(N/5)],['unclosed-directives','info\n'+'%%{\n'.repeat(N/4)],['fences','info\n'+'---\n'.repeat(N/4)],['unclosed-description','info\naccDescr {'+' a\n'.repeat(N/3)],['closed-description','info\naccDescr {'+' a\n'.repeat(N/3)+'}'],['long-acc-title','info\naccTitle: '+'x'.repeat(N)],['keyword-repeat','info '.repeat(N/5)],['show-info-repeat','info '+'showInfo'.repeat(N/8)]];
for(const [id,source] of stress)add(`stress/${id}`,source,{limit:1000000});
function mutator(corpus,fragments,seed){let state=seed>>>0;const rnd=()=>{state=(Math.imul(state,1664525)+1013904223)>>>0;return state/4294967296};const pick=a=>a[Math.floor(rnd()*a.length)];return()=>{let s=pick(corpus);for(let k=1+Math.floor(rnd()*3);k>0;k--){const pos=Math.floor(rnd()*(s.length+1)),v=rnd();if(v<.35)s=s.slice(0,pos)+pick(fragments)+s.slice(pos);else if(v<.6)s=s.slice(0,pos)+s.slice(pos+1+Math.floor(rnd()*3));else if(v<.75){const other=pick(corpus),from=Math.floor(rnd()*other.length);s=s.slice(0,pos)+other.slice(from,from+Math.floor(rnd()*20))+s.slice(pos)}else if(v<.9)s=s.slice(0,pos)+s.slice(pos,pos+Math.floor(rnd()*6))+s.slice(pos);else s=s.slice(0,pos)+pick(fragments)+s.slice(pos+1)}return s}}
const grammar=['info','showInfo','title','title ','accTitle: ','accDescr: ','accDescr {','accDescr {}','}',':','\n','\r\n',' ','\t','%%','%% c','%%{init: {}}%%','---','---\n','x','é',';','#35;','info showInfo','\ntitle t','Info'];
let surrogateExcluded=0;
for(const [campaign,count,fragments,seed] of [['grammar',3000,grammar,7],['security',4000,[...payloads,'\n','title ','accTitle: ','accDescr {','}','showInfo'],3]]){
 const next=mutator([...seeds],fragments,seed);
 for(let i=0;i<count;i++){const source=next();if(source.length>4000)continue;if(/[\uD800-\uDBFF](?![\uDC00-\uDFFF])|(?<![\uD800-\uDBFF])[\uDC00-\uDFFF]/u.test(source)){surrogateExcluded++;continue}add(`${campaign}-mutation/${i}`,source)}
}
await writeFile('fixtures/pele/info.json',JSON.stringify({revision,provenance,extraction:{version:1,upstreamVersion:'1.6.0',grammarMutationSeed:7,grammarMutations:3000,securityMutationSeed:3,securityMutations:4000,surrogateExcluded,largeSvgHashThreshold:250000,stressBudget:1000000},cases})+'\n');
console.log(`${cases.length} info scenarios; ${cases.filter(c=>c.expected.model).length} models; ${cases.filter(c=>c.expected.result).length} renders; ${cases.filter(c=>c.expected.error).length} errors; ${surrogateExcluded} surrogate exclusions`);
