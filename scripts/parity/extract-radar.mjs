// Dev-only oracle from pinned Pele public APIs. Generated fixtures are data only.
import {readFile,writeFile,readdir} from 'node:fs/promises';
import {execFileSync} from 'node:child_process';
import {stripTypeScriptTypes} from 'node:module';
import {createHash} from 'node:crypto';
import {parse,render} from 'pele';
import assert from 'node:assert/strict';
const root=process.argv[2]??'/tmp/pele-upstream-458322a5';
const revision='458322a5b8abb1ae57ddf127c32017ab9fb14a92';
assert.equal(execFileSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).trim(),revision);
execFileSync('git',['diff','--quiet',revision,'--','src','tests'],{cwd:root});
const provenance=[];
for(const path of ['src/diagrams/radar/index.ts','src/diagrams/radar/parser.ts','src/diagrams/radar/model.ts','src/diagrams/radar/render.ts','src/diagrams/common/tokens.ts','src/diagrams/common/fit-width.ts','src/text/label.ts','src/text/measurer.ts','src/text/metrics.ts','src/svg/builder.ts','src/svg/root.ts','src/svg/theme.ts','tests/support/inert.ts','tests/support/corpus.ts','tests/corpus/radar-docs.json']) {const data=await readFile(`${root}/${path}`);provenance.push({path,sha256:createHash('sha256').update(data).digest('hex')})}
// Decode literal text without evaluating any fixture string as JavaScript.
function decode(raw) {return raw.replace(/\\(?:u\{([0-9a-fA-F]+)\}|u([0-9a-fA-F]{4})|x([0-9a-fA-F]{2})|(\r?\n)|([\s\S]))/g,(_,u,U,x,n,c)=>u||U||x?String.fromCodePoint(parseInt(u??U??x,16)):n?'':({n:'\n',r:'\r',t:'\t',b:'\b',f:'\f',v:'\v',0:'\0'}[c]??c))}
function literals(text,interpolated=false) {const out=[];for(const m of text.matchAll(/`((?:[^`\\]|\\.)*)`|'((?:[^'\\\n]|\\.)*)'|"((?:[^"\\\n]|\\.)*)"/g)) {const raw=m[1]??m[2]??m[3];if(!interpolated&&raw.includes('${'))continue;out.push(decode(raw))}return out}
const seeds=new Set(JSON.parse(await readFile(`${root}/tests/corpus/radar-docs.json`,'utf8')));
const extraction=[];
for(const directory of ['tests/compat/radar','tests/radar','tests/render']) {
 for(const file of await readdir(`${root}/${directory}`)) {if(!/\.(?:spec|test)\.ts$/.test(file)||(directory==='tests/render'&&!/^radar(?:-security)?\.test\.ts$/.test(file)))continue;
 const path=`${directory}/${file}`,data=await readFile(`${root}/${path}`,'utf8');let accepted=0;
 for(let source of literals(data)) {if(!/radar/.test(source)) {if(!/^\s*(?:axis|curve|ticks|min|max|graticule|showLegend|title|accTitle|accDescr)\b/.test(source))continue;source='radar-beta\naxis my-axis, ax1, ax2\n'+source}seeds.add(source);accepted++}
 extraction.push({path,sha256:createHash('sha256').update(data).digest('hex'),literalCandidates:accepted});
 }}
const metricsText=await readFile(`${root}/src/text/measurer.ts`,'utf8');
const metricsData=await readFile(`${root}/src/text/metrics.ts`,'utf8');
const {REGULAR,BOLD}=await import(`data:text/javascript,${encodeURIComponent(stripTypeScriptTypes(metricsData))}`);
const tables=[REGULAR,BOLD].map(p=>Array.from({length:95},(_,i)=>parseInt(p.slice(i*2,i*2+2),36)));
const metrics={width(text,size,style){const table=tables[style&1?1:0],mono=!!(style&4);let units=0;for(let i=0;i<text.length;i++){const c=text.charCodeAt(i);if(c<32)continue;if(c<127)units+=mono?600:table[c-32];else if(c>=0x300&&c<=0x36f||c>=0x200b&&c<=0x200f||c>=0xdc00&&c<=0xdfff||c>=0xfe00&&c<=0xfe0f||c===0xfeff)continue;else if(c>=0x1100&&c<=0x115f||c>=0x2e80&&c<=0xa4cf||c>=0xac00&&c<=0xd7a3||c>=0xf900&&c<=0xfaff||c>=0xfe30&&c<=0xfe4f||c>=0xff00&&c<=0xff60||c>=0xffe0&&c<=0xffe6||c>=0xd800&&c<=0xdbff)units+=mono?1200:1000;else units+=mono?600:580}return units*size/1000}};
// Node's default measurer is this same upstream fallback; verify our copied deterministic oracle.
assert.equal(render('radar-beta\naxis A\ncurve c{1}').svg,render('radar-beta\naxis A\ncurve c{1}',{measurer:metrics}).svg);
const normalize=v=>JSON.parse(JSON.stringify(v,(_,value)=>value instanceof Map?[...value]:value));
const cases=[];
function add(id,source,options={},host={}){let expected;try{const model=parse(source,options);const resources={measurer:host.measure==='fixed'?{width:(text,size,style)=>text.length*size*(style&1?.71:.6)}:metrics,...(host.icons?{icons:n=>host.icons[n]}:{})};const result=render(model,{...options,...resources});assert.deepEqual(result,render(source,{...options,...resources}));expected={model:normalize(model),result}}catch(e){if(e.name==='AssertionError')throw e;if(e.name!=='PeleError')throw e;expected={error:{name:e.name,message:e.message,code:e.code,type:e.type,line:e.line,column:e.column,snippet:e.snippet}}}cases.push({id,source,options,host,expected})}
[...seeds].forEach((source,i)=>add(`upstream/${i}`,source));
const basic='radar-beta\n axis A["Top"], B["Right side"], C["Bottom"], D["Left side"]\n curve one["First"]{1,2,3,4}\n curve two{4,3,2,1}';
for(const source of [
'radar-beta','radar-beta:','radar-beta :','radar-beta axis A, B curve c{1, 2} ticks 5 max 10',
'radar-beta\naccDescr {}\naxis A','radar-beta\naccDescr {\naxis A\ncurve c{1}',
'radar-beta\naxis A,B\ncurve c { B: 2, A 1 }', 'radar-beta\naxis A,B\ncurve c { X: 2, A 1 }',
'radar-beta\ncurve c { X: 2 }','radar-beta\naxis A,A\ncurve c{A 1,A 2}',
'radar-beta\ncurve c{1} axis A','radar-beta\naxis A\ncurve c{1, A 2}',
'radar-beta\naxis A\ncurve c{1\n,2}','radar-beta\naxis A,\nB','radar-beta\naxis true',
'radar-beta\naxis circles','radar-beta\naxis maxi, min-1, _x, 1a',
'radar-beta\nshowLegend TRUE','radar-beta\nticks 05','radar-beta\nmax 1.2.3',
'radar-beta\nmax 5.','radar-beta\nmax -5','radar-beta\ngraticule circle polygon',
'radar-beta\ncurve c{00}','radar-beta\ncurve c{0.5.}','radar-beta\ncurve c{0000.5, 12.34.5, 100.25}',
'radar-beta\naxis A["x\\ny"]','radar-beta\naxis A\ncurve c{\n1,\n2\n}',
'radar-beta\naxis A,B\ncurve c{1,1}\nmin 1\nmax 1','radar-beta\naxis A,B\ncurve c{0,0}',
'radar-beta\naxis A,B\ncurve c{1,2}\nmin 9\nmax 2',
'radar-beta\naxis A,B\ncurve c{1,2}\nticks 0.3\ngraticule polygon',
'radar-beta\naxis A,B\ncurve c{1,2}\nmax '+'9'.repeat(400),
'radar-beta\naxis A,B\ncurve c{'+'9'.repeat(400)+',2}',
])add(`grammar/${cases.length}`,source);
for(const graticule of ['circle','polygon'])for(const ticks of [0,.3,1,5,12,32,33,1e9])for(const width of [undefined,500,260,100])add(`grid/${cases.length}`,basic+`\ngraticule ${graticule}\nticks ${ticks}`,width?{maxWidth:width}:{});
for(const [key,values] of Object.entries({width:[-5,0,1,160,320,600,5000,'wide'],height:[-1,0,1,300,600,4001,'tall'],axisScaleFactor:[-1,0,.5,1,10,11,'short'],axisLabelFactor:[-1,0,1.05,1.5,10,11],curveTension:[-1,0,.17,.5,1,2],marginTop:[-1,0,16,80,2000,2001],marginBottom:[-1,0,100,3000],marginLeft:[-1,0,120,2000],marginRight:[-1,0,120,2000]}))for(const value of values)for(const maxWidth of [undefined,220])add(`config/${cases.length}`,basic,{config:{radar:{[key]:value}},...(maxWidth?{maxWidth}:{})});
for(const text of ['Plain',' <strong>Bold <em>Italic</em></strong> ','A<br/>B\\nC','&#x1f642; #35; #amp;','fa:fa-user Person','Person fa:fa-missing','**literal** _under_score_','A\n\nB','<unknown x="foo">Words</unknown>','Words <i>italic</i> tail','<b/>Self close','    ','中文 🙂 é','__proto__'])for(const fontSize of [12,16,19.5])for(const maxWidth of [undefined,170]){const encoded=text.replaceAll('"','\\"').replaceAll('\n','\\n');add(`labels/${cases.length}`,`radar-beta\ntitle ${text.replaceAll('\n',' ')}\naxis a["${encoded}"],b,c\ncurve c["${encoded}"]{1,2,3}`,{fontSize,...(maxWidth?{maxWidth}:{})},{icons:{'fa:fa-user':'<path d="M0 0L4 4"/>'}})}
for(const description of ['accDescr: short  text','accDescr {\n first\n\n second \n}','accDescr {  a\n\n\n b   \n}','accDescr {\n\n   a\n\n b\n\n }','accDescr {first\u2028 second\u2029 third}', 'accDescr: 🌍 #amp;','accTitle: Named\naccDescr { <b>detail</b> }'])add(`accessibility/${cases.length}`,basic+'\n'+description);
for(const config of [{},{radar:{useMaxWidth:false}},{radar:{curveTension:0,marginLeft:80}},{pie:{useMaxWidth:false}}])for(const fontFamily of [undefined,'Arial','"A & B", sans-serif','Arial); url(https://evil.test)','中文, sans-serif'])add(`options/${cases.length}`,'---\ntitle: Front title\n---\n'+basic+'\ntitle Own #amp;',{config,fontFamily,fontFamilyMono:'"Mono", monospace',idPrefix:'a"&b',padding:0,responsive:false},{measure:'fixed'});
for(const payload of ['<script>alert(1)</script>','</text><image href="javascript:alert(1)">','&quot; onload=&quot;evil','<svg onload=alert(1)>','&#x0;','\u0001','var(--_fg)','style=\'var(--pele-bg)\''])for(const source of [`radar-beta\ntitle ${payload}\naxis A,B\ncurve c{1,2}`,`radar-beta\naxis A["${payload.replaceAll('"','\\"')}"],B\ncurve c{1,2}`,`radar-beta\naccTitle: ${payload}\naccDescr: ${payload}\naxis A`])add(`inert/${cases.length}`,source);
for(const width of [1.1,160.1,320.25,3999.99])for(const height of [1.5,160.3,320.75])add(`fractional-config/${cases.length}`,basic,{config:{radar:{width,height}},maxWidth:180.25});
for(const settings of [{fontSize:0},{fontSize:1.1},{fontSize:36.5,padding:3.25},{fontSize:12.5,maxWidth:200.5,padding:0},{maxWidth:0},{maxWidth:-1},{padding:-1},{limit:10},{outputLimit:1},{config:{radar:{useMaxWidth:false}},responsive:true}])add(`contract/${cases.length}`,basic,settings);
const payloadSource=await readFile(`${root}/tests/support/inert.ts`,'utf8');
const payloadBlock=payloadSource.slice(payloadSource.indexOf('export const PAYLOADS = [')+24);
const payloads=[];for(const m of payloadBlock.slice(0,payloadBlock.indexOf('\n];')).matchAll(/'((?:[^'\\]|\\.)*)'/g))payloads.push(decode(m[1]));
assert.equal(payloads.length,43);
const securityTemplates=[
 p=>`radar-beta\n axis a["${p}"], b, c\n curve x{1,2,3}`,
 p=>`radar-beta\n axis a['${p}'], b, c\n curve x{1,2,3}`,
 p=>`radar-beta\n axis a, b, c\n curve x["${p}"]{1,2,3}`,
 p=>`radar-beta\n axis a, b, c\n curve x['${p}']{ a: 1, b: 2, c: 3 }`,
 p=>`radar-beta\n axis ${p}, b\n curve x{1,2}`,
 p=>`radar-beta\n axis a, b\n curve ${p}{1,2}`,
 p=>`radar-beta\n axis a, b\n curve x{ ${p}: 1, a: 1, b: 2 }`,
 p=>`radar-beta\n axis a, b\n curve x{ ${p}, 2 }`,
 p=>`radar-beta\n title ${p}\n axis a, b\n curve x{1,2}`,
 p=>`radar-beta title ${p}`,
 p=>`radar-beta\n accTitle: ${p}\n accDescr: ${p}\n axis a, b\n curve x{1,2}`,
 p=>`radar-beta\n accDescr {\n ${p}\n }\n axis a, b\n curve x{1,2}`,
 p=>`radar-beta\n axis a, b\n curve x{1,2}\n graticule ${p}`,
 p=>`radar-beta\n axis a, b\n curve x{1,2}\n max ${p}\n ticks ${p}`,
 p=>`radar-beta\n axis a, b %% ${p}\n curve x{1,2}`,
 p=>`radar-beta: ${p}\n axis a, b`,
 p=>`---\ntitle: "${p}"\n---\nradar-beta\n axis a, b\n curve x{1,2}`,
 p=>`---\nconfig:\n  radar:\n    width: "${p}"\n    curveTension: "${p}"\n    marginLeft: "${p}"\n---\nradar-beta\n axis a, b\n curve x{1,2}`,
 p=>`%%{init: {"radar": {"axisLabelFactor": "${p}"}, "themeVariables": {"cScale0": "${p}", "radar": {"axisColor": "${p}"}}}}%%\nradar-beta\n axis a, b\n curve x{1,2}`,
];
for(const template of securityTemplates)for(const payload of payloads)add(`security/${cases.length}`,template(payload));
for(const payload of payloads)add(`security-options/${cases.length}`,'radar-beta\n accTitle: t\n axis a, b\n curve x{1,2}',{idPrefix:payload,fontFamily:payload});
// Reproduce the pinned security campaign's exact mutation operations and seed.
const securitySeeds=new Set();for(const file of(await readdir(`${root}/tests/compat/radar`)).filter(f=>/\.spec\.[jt]s$/.test(f)))for(const source of literals(await readFile(`${root}/tests/compat/radar/${file}`,'utf8'),true))if(/radar/.test(source))securitySeeds.add(source);
for(const source of JSON.parse(await readFile(`${root}/tests/corpus/radar-docs.json`,'utf8')))securitySeeds.add(source);
const securityCorpus=[...securitySeeds],securityFragments=[...payloads,'"',"'",'[',']','{','}',':',',','\n','title ','axis ','curve c{1,2}','max 3','graticule polygon'];let securityState=3;
const securityRandom=()=>{securityState=(Math.imul(securityState,1664525)+1013904223)>>>0;return securityState/4294967296};const pick=list=>list[Math.floor(securityRandom()*list.length)];
for(let i=0;i<4000;i++){let source=pick(securityCorpus);const edits=1+Math.floor(securityRandom()*3);for(let k=0;k<edits;k++){const at=Math.floor(securityRandom()*(source.length+1)),r=securityRandom();if(r<.35)source=source.slice(0,at)+pick(securityFragments)+source.slice(at);else if(r<.6)source=source.slice(0,at)+source.slice(at+1+Math.floor(securityRandom()*3));else if(r<.75){const other=pick(securityCorpus),from=Math.floor(securityRandom()*other.length);source=source.slice(0,at)+other.slice(from,from+Math.floor(securityRandom()*20))+source.slice(at)}else if(r<.9)source=source.slice(0,at)+source.slice(at,at+Math.floor(securityRandom()*6))+source.slice(at);else source=source.slice(0,at)+pick(securityFragments)+source.slice(at+1)}if(source.length<=4000)add(`security-mutation/${i}`,source)}
for(const yaml of ["---\n---\nfoo\n---", "---\n---", "---\n---\nx\n---\n---", "--- \t\r\n---\r\nfoo\r\n---", "---\n\n---\n---", "---\nfoo\n--- x", "---\nfoo\n---\u00a0", "---\nfoo\n---\n"])add(`hidden-yaml/${cases.length}`,"radar-beta\naxis A\ncurve c{1}\n"+yaml);
let state=7;const rnd=()=>{state=(Math.imul(state,1664525)+1013904223)>>>0;return state/4294967296};const fragments=['radar-beta','radar-beta:','title ','accTitle: ','accDescr {','}',':','"',"'",'1','0','-1','1.5','01','1.','.5','1e3','\n','\r\n',' ','\t','%% c','%%{init: {}}%%','---\n','\\','x','é',';',',','#35;','axis','axis A,B','curve','curve c{1,2}','curve d{A:1,B 2}','showLegend false','ticks 5','graticule polygon','true','circles','["L"]'];const corpus=[...seeds,basic];
for(let i=0;i<3000;i++){let source=corpus[Math.floor(rnd()*corpus.length)];for(let k=1+Math.floor(rnd()*3);k>0;k--){const at=Math.floor(rnd()*(source.length+1));if(rnd()<.4)source=source.slice(0,at)+fragments[Math.floor(rnd()*fragments.length)]+source.slice(at);else source=source.slice(0,at)+source.slice(at+1+Math.floor(rnd()*4))}if(source.length<=2000)add(`mutated/${i}`,source)}
await writeFile('fixtures/pele/radar.json',JSON.stringify({revision,tool:{name:'extract-radar',version:1},provenance,extraction,generated:{seed:7,mutations:3000,securitySeed:3,securityMutations:4000,securityTemplates:19,payloads:43},exclusions:[{source:'tests/compat/radar/radar.spec.ts relativeRadius and closedRoundCurve cases',reason:'Private helper assertions represented by public complete-render grid/min/max/tension configurations.'},{source:'Interpolated test source literals',reason:'Reproduced as explicit generated grid, config, labels, limits and degeneracy scenarios; no fixture text is executed.'}],cases},null,2)+'\n');
console.log(`${cases.length} radar public API scenarios; ${cases.filter(c=>c.expected.result).length} render, ${cases.filter(c=>c.expected.error).length} errors`);
