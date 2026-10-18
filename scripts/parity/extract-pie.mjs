// Dev-only pinned public API oracle. No upstream renderer is shipped or executed in consumers.
import{readFile,writeFile,readdir}from'node:fs/promises';
import{execFileSync}from'node:child_process';
import{stripTypeScriptTypes}from'node:module';
import{createHash}from'node:crypto';
import{parse,render}from'pele';
import assert from'node:assert/strict';
const root=process.argv[2]??'/tmp/pele-upstream-458322a5';
const revision='458322a5b8abb1ae57ddf127c32017ab9fb14a92';
assert.equal(execFileSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).trim(),revision);
execFileSync('git',['diff','--quiet',revision,'--','src','tests'],{cwd:root});
const provenance=[];
for(const path of ['src/diagrams/pie/parser.ts','src/diagrams/pie/model.ts','src/diagrams/pie/render.ts','src/diagrams/common/tokens.ts','src/diagrams/common/fit-width.ts','src/text/label.ts','src/text/measurer.ts','src/text/metrics.ts','src/svg/builder.ts','src/svg/root.ts','src/svg/theme.ts']){const data=await readFile(`${root}/${path}`);provenance.push({path,sha256:createHash('sha256').update(data).digest('hex')})}
function literals(text){let out=[];for(const m of text.matchAll(/`((?:[^`\\]|\\.)*)`|'((?:[^'\\\n]|\\.)*)'|"((?:[^"\\\n]|\\.)*)"/g)){const raw=m[1]??m[2]??m[3];try{out.push(new Function('return `'+raw.replace(/`/g,'\\`').replace(/\$\{/g,'\\${')+'`')())}catch{}}return out}
const seeds=new Set(JSON.parse(await readFile(`${root}/tests/corpus/pie-docs.json`,'utf8')));
for(const file of(await readdir(`${root}/tests/compat/pie`)).filter(f=>/\.(?:spec|test)\.ts$/.test(f))){for(const value of literals(await readFile(`${root}/tests/compat/pie/${file}`,'utf8')))if(/pie/.test(value))seeds.add(value)}
for(const value of literals(await readFile(`${root}/tests/render/pie.test.ts`,'utf8')))if(/pie/.test(value))seeds.add(value);
const metricsText=await readFile(`${root}/src/text/measurer.ts`,'utf8');
const metricsData=await readFile(`${root}/src/text/metrics.ts`,'utf8');
const {REGULAR,BOLD}=await import(`data:text/javascript,${encodeURIComponent(stripTypeScriptTypes(metricsData))}`);
const tables=[REGULAR,BOLD].map(p=>Array.from({length:95},(_,i)=>parseInt(p.slice(i*2,i*2+2),36)));
const metrics={width(text,size,style){const table=tables[style&1?1:0],mono=!!(style&4);let units=0;for(let i=0;i<text.length;i++){const c=text.charCodeAt(i);if(c<32)continue;if(c<127)units+=mono?600:table[c-32];else if(c>=0x300&&c<=0x36f||c>=0x200b&&c<=0x200f||c>=0xdc00&&c<=0xdfff||c>=0xfe00&&c<=0xfe0f||c===0xfeff)continue;else if(c>=0x1100&&c<=0x115f||c>=0x2e80&&c<=0xa4cf||c>=0xac00&&c<=0xd7a3||c>=0xf900&&c<=0xfaff||c>=0xfe30&&c<=0xfe4f||c>=0xff00&&c<=0xff60||c>=0xffe0&&c<=0xffe6||c>=0xd800&&c<=0xdbff)units+=mono?1200:1000;else units+=mono?600:580}return units*size/1000}};
// Node's default measurer is this same upstream fallback; verify our copied deterministic oracle.
assert.equal(render('pie title Metrics\n"a":1').svg,render('pie title Metrics\n"a":1',{measurer:metrics}).svg);
const normalize=v=>JSON.parse(JSON.stringify(v,(_,value)=>value instanceof Map?[...value]:value));
const cases=[];
function add(id,source,options={},host={}){let expected;try{const model=parse(source,options);const resources={measurer:host.measure==='fixed'?{width:(text,size,style)=>text.length*size*(style&1?.71:.6)}:metrics,...(host.icons?{icons:n=>host.icons[n]}:{})};const result=render(model,{...options,...resources});assert.deepEqual(result,render(source,{...options,...resources}));expected={model:normalize(model),result}}catch(e){if(e.name==='AssertionError')throw e;if(e.name!=='PeleError')throw e;expected={error:{name:e.name,message:e.message,code:e.code,type:e.type,line:e.line,column:e.column,snippet:e.snippet}}}cases.push({id,source,options,host,expected})}
[...seeds].forEach((s,i)=>add(`upstream/${i}`,s));
for(const position of ['right','left','top','bottom','center','none','other',null,true,2,{custom:true}])for(const hole of [0,.2,.5,.9,1,-1])for(const maxWidth of [undefined,500,260,140])add(`layout/${cases.length}`,'pie showData title An example with a title\n"Alpha":2\n"<b>Bold</b> Beta":3\n"Gamma":100',{config:{pie:{legendPosition:position,donutHole:hole,textPosition:.5}},...(maxWidth?{maxWidth}:{})});
for(const label of ['Plain',' <strong>Bold <em>Italic</em></strong> ','A<br/>B\\nC','&#x1f642; #35; #amp;','fa:fa-user Person','Person fa:fa-missing','**literal** _under_score_','A\n\nB','<unknown x="foo">Words</unknown>','<b>one</b> tail','Words <i>italic</i> tail','<b/>Self close','    ','中文 🙂 é','__proto__'])for(const fontSize of [12,16,19.5])for(const maxWidth of [undefined,170]){const source='pie showData title '+label.replaceAll('\n',' ')+'\n"'+label.replaceAll('"','\\"').replaceAll('\n','\\n')+'":1\n"Other":7';add(`labels/${cases.length}`,source,{fontSize,...(maxWidth?{maxWidth}:{})},{icons:{'fa:fa-user':'<path d="M0 0L4 4"/>'}})}
for(const description of ['accDescr: short  text','accDescr {\n first\n\n second \n}','accDescr {  a\n\n\n b   \n}','accDescr {\n\n   a\n\n b\n\n }','accDescr {first\u2028 second\u2029 third}', 'accDescr: 🌍 #amp;','accTitle: Named\naccDescr { <b>detail</b> }'])add(`accessibility/${cases.length}`,'pie\n'+description+'\n"x":1');
for(const number of ['-0','00','01','1.','0.','1.2.','12.3.4','1e3','.5','-.5','-1','-0.1','99999999999999999999999999','9'.repeat(400)])add(`numbers/${cases.length}`,'pie\n"number":'+number);
for(const values of [[1,7],[1,39],[1,199],[0,0],[1,1,1],[2,2.4,46,50]])add(`shares/${cases.length}`,'pie\n'+values.map((n,i)=>`"${i}":${n}`).join('\n'));
for(const config of [{},{pie:{useMaxWidth:false}},{flowchart:{useMaxWidth:false}},{pie:{textPosition:-1}},{pie:{textPosition:0}},{pie:{textPosition:1}}])for(const fontFamily of [undefined,'Arial','"A & B", sans-serif','Arial); url(https://evil.test)','中文, sans-serif'])add(`options/${cases.length}`,'---\ntitle: Front title\n---\npie title Own #amp;\n"a":1\n"b":2',{config,fontFamily,fontFamilyMono:'"Mono", monospace',idPrefix:'a"&b',padding:0,responsive:false},{measure:'fixed'});
const payloadText=await readFile(`${root}/tests/support/inert.ts`,'utf8');
const payloadArray=payloadText.slice(payloadText.indexOf('export const PAYLOADS = ')+23);const payloadEnd=payloadArray.indexOf('\n];');const payloads=new Function('return '+payloadArray.slice(0,payloadEnd+2))();
for(const payload of payloads)for(const source of [`pie title ${payload}\n"a":1`,`pie\n"${payload}":1\n"b":2`,`pie\naccTitle: ${payload}\naccDescr: ${payload}\n"a":1`,`pie\naccDescr {\n${payload}\n}\n"a":1`])add(`inert/${cases.length}`,source);
for(const yaml of ["---\n---\nfoo\n---", "---\n---", "---\n---\nx\n---\n---", "--- \t\r\n---\r\nfoo\r\n---", "---\n\n---\n---", "---\nfoo\n--- x", "---\nfoo\n---\u00a0", "---\nfoo\n---\n"])add(`hidden-yaml/${cases.length}`,"pie\n\"a\":1\n"+yaml);
let state=7;const rnd=()=>{state=(Math.imul(state,1664525)+1013904223)>>>0;return state/4294967296};const fragments=['pie','showData','title ','accTitle: ','accDescr {','}',':','"',"'",'1','0','-1','1.5','01','1.','.5','1e3','\n','\r\n',' ','\t','%% c','%%{init: {}}%%','---\n','\\','x','é',';',',','#35;','"a":1','\n"b":2'];const corpus=[...seeds];
for(let i=0;i<3000;i++){let s=corpus[Math.floor(rnd()*corpus.length)];for(let k=1+Math.floor(rnd()*3);k>0;k--){const p=Math.floor(rnd()*(s.length+1));if(rnd()<.4)s=s.slice(0,p)+fragments[Math.floor(rnd()*fragments.length)]+s.slice(p);else s=s.slice(0,p)+s.slice(p+1+Math.floor(rnd()*4))}if(s.length<=2000)add(`mutated/${i}`,s)}
await writeFile('fixtures/pele/pie.json',JSON.stringify({revision,provenance,cases},null,2)+'\n');
console.log(`${cases.length} pie public API scenarios; ${cases.filter(c=>c.expected.result).length} render, ${cases.filter(c=>c.expected.error).length} errors`);
