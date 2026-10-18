// Development-only pinned Pele public oracle. Never ships in Rust consumers.
import {readFile,writeFile,readdir} from 'node:fs/promises';
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
import {parse,render,detectType} from 'pele';
const root=process.argv[2]??'/tmp/pele-upstream-458322a5';
const revision='458322a5b8abb1ae57ddf127c32017ab9fb14a92';
assert.equal(execFileSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).trim(),revision);
execFileSync('git',['diff','--quiet',revision,'--','src','tests'],{cwd:root});
const paths=[];
for(const dir of ['src/diagrams/er','tests/compat/er','tests/er','tests/render','src/layout','src/svg','src/text'])for(const name of await readdir(`${root}/${dir}`))if(/\.[jt]s$/.test(name)&&(dir!=='tests/render'||/^(er|security|fonts|max-width|mobile-layouts|all-types)/.test(name)))paths.push(`${dir}/${name}`);
paths.push('tests/corpus/er-docs.json','tests/support/corpus.ts','tests/support/inert.ts','src/diagrams/common/generics.ts','src/diagrams/common/fit-width.ts','src/util/url.ts');
const provenance=[];for(const path of paths){const data=await readFile(`${root}/${path}`);provenance.push({path,sha256:createHash('sha256').update(data).digest('hex')})}
function literals(text){const out=[];for(const m of text.matchAll(/`((?:[^`\\]|\\.)*)`|'((?:[^'\\\n]|\\.)*)'|"((?:[^"\\\n]|\\.)*)"/g)){const raw=m[1]??m[2]??m[3];if(raw.includes('${'))continue;out.push(raw.replace(/\\(u\{[0-9a-fA-F]+\}|u[0-9a-fA-F]{4}|x[0-9a-fA-F]{2}|[\s\S])/g,(_,c)=>{if(c[0]==='u'||c[0]==='x')return String.fromCodePoint(parseInt(c[1]==='{'?c.slice(2,-1):c.slice(1),16));return({n:'\n',r:'\r',t:'\t',v:'\v',f:'\f',b:'\b','0':'\0'})[c]??c}))}return out}
const normalize=model=>JSON.parse(JSON.stringify(Object.fromEntries(Object.entries(model).filter(([k])=>!['subgraphLookup','claimed'].includes(k))),(_,v)=>v instanceof Map?[...v]:v));
const cases=[],exclusions=[];
function add(id,source,options={},host={}){if(!source.isWellFormed()){exclusions.push({id,reason:'UTF-16 lone surrogate has no UTF-8 scalar representation',sourceUtf16:[...Array(source.length)].map((_,i)=>source.charCodeAt(i))});return}let expected;try{if(detectType(source)!=='er')return;const model=parse(source,options);expected={model:normalize(model)};const resources={...(host.icons?{icons:name=>host.icons[name]}:{}),...(host.measure==='fixed'?{measurer:{width:(text,size,style)=>text.length*size*(style&1?.71:.6)}}:{})};const result=render(model,{...options,...resources});assert.deepEqual(result,render(source,{...options,...resources}));if(result.svg.length>250000){const {svg,...rest}=result;expected.result={...rest,svgSha256:createHash('sha256').update(svg).digest('hex')}}else expected.result=result;}catch(e){if(e.name!=='PeleError')throw e;expected={...expected,error:{name:e.name,message:e.message,code:e.code,type:e.type,line:e.line,column:e.column,snippet:e.snippet}}}cases.push({id,source,options,host,...(expected.error?{errorStage:expected.model?'render':'parse'}:{}),expected})}
const seeds=new Set(JSON.parse(await readFile(`${root}/tests/corpus/er-docs.json`,'utf8')));
for(const path of paths.filter(p=>p.startsWith('tests/')&&/\.[jt]s$/.test(p)))for(const s of literals(await readFile(`${root}/${path}`,'utf8')))if(/erDiagram/i.test(s))seeds.add(s);
[...seeds].forEach((s,i)=>add(`upstream/${i}`,s));

// Reconstruct literal-valued const/template expressions inside each pinned spec test.
// This is a bounded string decoder, never eval/new Function or execution of diagram text.
const dynamicAudit=[];
for(const path of ['tests/compat/er/erDiagram.spec.js','tests/compat/er/subgraph.spec.js']){
 const text=await readFile(`${root}/${path}`,'utf8');
 let inherited={};
 for(const [chunkIndex,chunk]of text.split(/\bit(?:\.skip)?\(/).entries()){
  const env={...inherited};
  const lex=/\bconst\s+(\w+)\s*=\s*(`(?:[^`\\]|\\.)*`|'(?:[^'\\\n]|\\.)*'|"(?:[^"\\\n]|\\.)*")\s*;|(`(?:[^`\\]|\\.)*`)/g;
  for(const m of chunk.matchAll(lex)){
   const raw=m[2]??m[3];let unresolved=false;
   const replaced=raw.replace(/\$\{(\w+)\}/g,(_,key)=>{if(env[key]===undefined){unresolved=true;return '${'+key+'}'}return env[key].replaceAll('\\','\\\\').replaceAll('`','\\`')});
   if(unresolved){if(raw.includes('erDiagram'))dynamicAudit.push({path,chunk:chunkIndex,reason:'Parameterized character/property loop reconstructed below',template:raw});continue}
   const decoded=literals(replaced)[0];
   if(m[1])env[m[1]]=decoded;
   if(decoded&&/^erDiagram/.test(decoded))add(`upstream-dynamic/spec/${cases.length}`,decoded);
  }
  inherited=env;
 }
}
const charList=literals((await readFile(`${root}/tests/compat/er/erDiagram.spec.js`,'utf8')).split('const chars =')[1].split('const allowed =')[0])[0].split(' ');
for(const ch of charList)for(const name of [`Blo${ch}rf`,`Blo${ch}${ch}rf`,`Blorf${ch}`,`${ch}Blorf`])for(const quoted of [false,true])add(`upstream-dynamic/entity-char/${cases.length}`,`erDiagram\n ${quoted?'"'+name+'"':name}\n`);
add('upstream-dynamic/combined-chars','erDiagram\n "a'+charList.join('')+'"\n');
for(const ch of ['\n','\r','\b','\v'])add(`upstream-dynamic/disallowed-name/${cases.length}`,`erDiagram\n "Blo${ch}rf"\n`);
for(const ch of '0-[]()')add(`upstream-dynamic/attribute-leading/${cases.length}`,`erDiagram\nBOOK { string ${ch}author }`);
for(const prop of ['__proto__','constructor','prototype'])add(`upstream-dynamic/property-name/${cases.length}`,`erDiagram\n${prop} ||--|{ ORDER : place`);

const cardinalities=['|o','o|','}o','o{','}|','|{','||','u','one or zero','one or more','one or many','one','only one','zero or one','zero or more','zero or many','many(0)','many(1)','many','1','1+','0+'];
for(const a of cardinalities)for(const b of cardinalities)for(const line of ['--','..','.-','-.','to','optionally to'])add(`cardinality/${cases.length}`,`erDiagram\nA ${a}${line.match(/[a-z]/)?' ':''}${line}${line.match(/[a-z]/)?' ':''}${b} B : relates`);
const base='erDiagram\ndirection LR\nsubgraph g[Customer records]\nA[Customer] {\n int id PK "Identifier"\n string name\n List~string~ tags FK,UK "A long comment with **bold** and _italics_"\n}\nA ||--o{ B : contains\nend\nB ||..|| C : has\nA ||--|| A : self\nC |o--|| A : back';
for(const dir of ['TB','BT','LR','RL'])for(const fontSize of [0,1,8,12,16,19.5,32,90])for(const maxWidth of [undefined,80,180,360,640,900])add(`layout/${cases.length}`,base.replace('direction LR','direction '+dir),{fontSize,maxWidth});
for(const key of ['nodeSpacing','rankSpacing'])for(const v of [-1,0,.1,1,8,40,500,2000,9000,'bad',null,true])add(`configuration/${cases.length}`,base,{config:{er:{[key]:v}}});
for(const options of [{responsive:false,fontFamily:'Arial',fontFamilyMono:'Menlo, monospace',idPrefix:'er"&x'},{outputLimit:50},{limit:8},{maxEdges:0},{autoDirection:false,maxWidth:180},{directionBreakpoint:50,maxWidth:180},{directionBreakpoint:1000,maxWidth:800},{links:false},{linkSchemes:[]},{linkRel:''},{padding:0},{padding:32}])add(`host-options/${cases.length}`,base,options);
for(const where of ['node','class','default','group'])for(const style of ['fill:#f00,stroke:#abc,color:#fff,stroke-width:3px','font-weight:bold,font-style:italic,font-size:24px','rx:20,ry:8,opacity:0.6,fill-opacity:0.2','font-family:monospace,text-decoration:underline,letter-spacing:2px,word-spacing:4px','fill:url(javascript:bad),color:expression(bad),stroke:#abc','color:#0f0,color:#00f,font-weight:700,font-size:16.5','stroke-dasharray:3 4,stroke-linecap:butt,stroke-linejoin:bevel'])add(`styles/${cases.length}`,base+'\n'+({node:'style A '+style,class:'classDef c '+style+'\nclass A c',default:'classDef default '+style,group:'style g '+style})[where]);
for(const text of ['fa:fa-user Person','Person fa:fa-missing','fa:fa-user <strong>person</strong>','fa:fa-user<br/>fa:fa-missing Person',' **Strong** _italics_ ','<b>Bold</b><i>Italic</i>','\n  first\n    second\n','á\n中文🙂','List~Map~K,V~~','a'.repeat(500)])for(const maxWidth of [undefined,150])add(`labels/${cases.length}`,`erDiagram\nA["${text}"] ||--|| B : "${text}"\nA { string value "${text}" }\nsubgraph g["${text}"]\nC\nend`,{maxWidth},{icons:{'fa:fa-user':'<path d="M0 0L4 4"/>'}});
for(const display of ['Note','https://example.com/a b','javascript:bad','ja%76ascript:bad','&#106;avascript:bad','//server/share','../relative','__proto__'])for(const options of [{},{links:false},{linkSchemes:[]},{linkRel:''}])add(`internal-links/${cases.length}`,`erDiagram\np["${display}"]:::internal-link { string body }`,options);
const securityText=await readFile(`${root}/tests/render/er-security.test.ts`,'utf8');
const inertText=await readFile(`${root}/tests/support/inert.ts`,'utf8');
const payloads=literals(inertText.slice(inertText.indexOf('export const PAYLOADS ='),inertText.indexOf('\n];',inertText.indexOf('export const PAYLOADS ='))+3));
const templates=[];for(const m of securityText.matchAll(/`((?:[^`\\]|\\.)*)`/g)){if(!m[1].includes('${p}'))continue;const raw=m[1].replaceAll('${p}','__PAYLOAD__');if(raw.includes('${'))throw new Error('Unaccounted security interpolation');const s=literals('`'+raw+'`')[0];if(s&&/^(erDiagram|---|%%)/.test(s))templates.push(s)}
for(const s of templates)for(const p of payloads)add(`upstream-dynamic/security/${cases.length}`,s.replaceAll('__PAYLOAD__',p));
for(const p of payloads)add(`upstream-dynamic/option-security/${cases.length}`,base,{idPrefix:p,fontFamily:p});
const spec=await readFile(`${root}/tests/compat/er/erDiagram.spec.js`,'utf8');
// Every dynamic cardinality and attribute combination is reconstructed explicitly.
for(const key of ['PK','FK','UK','pk','fk','uk','PK,FK','PK, FK, UK','FK,PK,UK','bad',''])for(const type of ['int','string','string(99)','float?','List~int~','Map~K,V~','Array~Array~string~~','a~b','`my type`','µ','é','*','9'])for(const name of ['id','*name','`my name`','name[]','name(12)','a,b'])add(`attributes/${cases.length}`,`erDiagram\nA { ${type} ${name} ${key} "Comment" }`);
for(const code of [0x85,0xa0,0x1680,0x2028,0x2029,0xfeff,0x2000,0x3000]){const w=String.fromCodePoint(code);for(const s of [`erDiagram${w}A`,`erDiagram\nA${w}||--||${w}B : x`,`erDiagram\nA 1${w}to many B : x`,`erDiagram\naccTitle${w}:${w}Title\nA`,`erDiagram\nA { string${w}id }`])add(`unicode-whitespace/${cases.length}`,s)}
for(const code of [0xaa,0xb5,0xc0,0x300,0x345,0x378,0x4e00,0x1f600,0x10400,0xffdc,0xffdd]){const ch=String.fromCodePoint(code);add(`unicode-name/${code}`,`erDiagram\n${ch} ||--|| B : x`);add(`unicode-attribute/${code}`,`erDiagram\nA { ${ch} ${ch} }`)}
const corpus=[...seeds];let state=7;const rnd=()=>{state=(Math.imul(state,1664525)+1013904223)>>>0;return state/4294967296};const pick=a=>a[Math.floor(rnd()*a.length)];
const oracleText=await readFile(`${root}/tests/er/oracle.test.ts`,'utf8');const fragments=literals(oracleText.slice(oracleText.indexOf('const FRAGMENTS ='),oracleText.indexOf('\n];',oracleText.indexOf('const FRAGMENTS ='))+3));
for(let i=0;i<5000;i++){let source=pick(corpus);for(let k=0,n=1+Math.floor(rnd()*3);k<n;k++){const pos=Math.floor(rnd()*(source.length+1)),r=rnd();if(r<.35)source=source.slice(0,pos)+pick(fragments)+source.slice(pos);else if(r<.6)source=source.slice(0,pos)+source.slice(pos+1+Math.floor(rnd()*3));else if(r<.75){const other=pick(corpus),from=Math.floor(rnd()*other.length);source=source.slice(0,pos)+other.slice(from,from+Math.floor(rnd()*20))+source.slice(pos)}else if(r<.9)source=source.slice(0,pos)+source.slice(pos,pos+Math.floor(rnd()*6))+source.slice(pos);else source=source.slice(0,pos)+pick(fragments)+source.slice(pos+1)}if(source.length<=4000)add(`mutated/${i}`,source)}
state=8;const pieces=[...new Set([...fragments,...fragments.map(f=>f.trim()).filter(f=>f!=='')])];const openers=['erDiagram\n','erDiagram\nA {\n','erDiagram\nstyle A ','erDiagram\nA ','erDiagram\nsubgraph s\n',''];for(let i=0;i<5000;i++){let s=pick(openers);for(let k=1+Math.floor(rnd()*10);k>0;k--)s+=pick(pieces)+(rnd()<.3?' ':'');add(`random-tokens/${i}`,s)}

state=3;const securityFragments=[...payloads,'"','`','~','{','}','[',']','\n',':::',' : ','||--o{','style A fill:','classDef c ','subgraph ','end\n'];
for(let i=0;i<4000;i++){let source=pick(corpus);for(let k=0,n=1+Math.floor(rnd()*3);k<n;k++){const pos=Math.floor(rnd()*(source.length+1)),r=rnd();if(r<.35)source=source.slice(0,pos)+pick(securityFragments)+source.slice(pos);else if(r<.6)source=source.slice(0,pos)+source.slice(pos+1+Math.floor(rnd()*3));else if(r<.75){const other=pick(corpus),from=Math.floor(rnd()*other.length);source=source.slice(0,pos)+other.slice(from,from+Math.floor(rnd()*20))+source.slice(pos)}else if(r<.9)source=source.slice(0,pos)+source.slice(pos,pos+Math.floor(rnd()*6))+source.slice(pos);else source=source.slice(0,pos)+pick(securityFragments)+source.slice(pos+1)}if(source.length<=4000)add(`upstream-security-mutated/${i}`,source)}
const N=50000,repeat=(count,line)=>Array.from({length:count},(_,i)=>line(i)).join('');
const stressSources=[
 ['long-direction','A '.repeat(N/2)+'direction XX'],['direction-words','direction '.repeat(N/10)],['direction-blank-lines',('direction'+'\n'.repeat(9)).repeat(N/18)+'TB'],['direction-many-lines','A direction B\n'.repeat(N/14)],['direction-spaces','direction'+' '.repeat(N)+'XX'],['accTitle-no-colon',('accTitle'+' '.repeat(40)).repeat(N/48)],['ones-spaces',('1'+' '.repeat(99)).repeat(N/100)+'!'],['card-symbols','|o}o}|o{|{||--..'.repeat(N/16)],['unclosed-quotes','"a%'.repeat(N/3)],
 ['single-tilde','A {\n'+'! '.repeat(N/4)+'~'+' !'.repeat(N/4)],['unmatched-tilde','A {\n'+'!'.repeat(N)+'~~'],['tilde-every-line','A {\n'+'a~ b\n'.repeat(N/5)+'}'],['tilde-pairs','A {\n'+'~a~ '.repeat(N/4)+'}'],['unclosed-backtick','A {\n`'+'a '.repeat(N/2)],['backtick-pairs','A {\n'+'`a``b` '.repeat(N/7)+'}'],['unclosed-comment','A {\n'+'a b "'.repeat(N/5)],['unclosed-description','accDescr {'+' text\n'.repeat(N/6)],
 ['colon-hash-style','A\nstyle A '+':#'.repeat(N/2)+'\n'],['10000-style-ids','style '+repeat(10000,i=>`e${i},`)+'z fill:red\n'],['10000-style-declarations','A\nstyle A '+repeat(10000,i=>`x${i}:1,`)+'fill:red\n'],['10000-class-ids','class '+repeat(10000,i=>`e${i},`)+'z '+repeat(10000,i=>`c${i},`)+'z\n'],['20000-class-names','A:::'+repeat(20000,i=>`c${i},`)+'z\n'],
 ['20000-nested','subgraph s\n'.repeat(20000)+'A\n'+'end\n'.repeat(20000)],['3000-nested',repeat(3000,i=>`subgraph s${i}\n`)+'A ||--|| B : x\n'+'end\n'.repeat(3000)],['5000-siblings',repeat(5000,i=>`subgraph s${i}\nn${i}\nend\n`)],['5000-duplicate-subgraphs',repeat(5000,()=>`subgraph s\nA\nB\nC\nend\n`)],['10000-entities-subgraph','subgraph s\n'+repeat(10000,i=>`n${i}\n`)+'end\n'],['unclosed-subgraphs','subgraph s\n'.repeat(N/11)],
 ['5000-attributes','A {\n'+repeat(5000,i=>`t${i} n${i}\n`)+'}'],['10000-attribute-keys','A {\n string id '+'PK,'.repeat(10000)+'FK\n}'],['5000-attribute-blocks',repeat(5000,i=>`A { t n${i} }\n`)],['50000-comment','A {\n string id "'+'word '.repeat(N/5)+'"\n}'],['50000-attribute-name','A {\n string '+'a'.repeat(N)+'\n}'],['50000-tildes','A {\n '+'~'.repeat(N)+' name\n}'],['tilde-comma-type','A {\n '+'a~b,'.repeat(N/4)+' name\n}'],['emphasis-comment','A {\n string id "'+'*a _b '.repeat(N/6)+'"\n}'],['entities-linebreak-name','"'+'#35;&amp;<br>'.repeat(N/13)+'"'],['50000-relationship-label','A ||--o{ B : "'+'word '.repeat(N/5)+'"'],
 ['5000-entities',repeat(5000,i=>`n${i}\n`)],['5000-chain',repeat(5000,i=>`n${i} ||--o{ n${i+1} : r\n`)],['5000-star',repeat(5000,i=>`c ||--o{ n${i} : r\n`)],['2000-parallel-self','A ||--o{ B : r\n'.repeat(2000)+'A ||--|| A : s\n'.repeat(2000)],['complete-graph-50',repeat(50,i=>repeat(50-i-1,k=>`n${i} }o--o{ n${i+k+1} : r\n`))],['2000-group-relationships','subgraph a\nA\nend\nsubgraph b\nB\nend\n'+'a ||--|| b : r\nA ||--|| b : s\n'.repeat(1000)],['10000-aliases','a["Some Alias"]\n'.repeat(10000)],
];
for(const [id,body]of stressSources)add(`upstream-stress/${id}`,'erDiagram\n'+body,{limit:1000000});
add('upstream-stress/default-source-limit','erDiagram\n'+'A ||--o{ B : r\n'.repeat(5000));

for(const depth of [1,2,4,16,100,500,3000])add(`nest-depth/${depth}`,'erDiagram\n'+Array.from({length:depth},(_,i)=>'subgraph s'+i).join('\n')+'\nA ||--|| B : x\n'+'end\n'.repeat(depth),{limit:1000000});
for(const count of [0,1,2,5,50,300,1000]){add(`star/${count}`,'erDiagram\n'+Array.from({length:count},(_,i)=>'root ||--o{ N'+i+' : relation').join('\n'),{limit:1000000});add(`parallel/${count}`,'erDiagram\n'+'A ||--o{ B : r\n'.repeat(count),{limit:1000000});add(`self/${count}`,'erDiagram\n'+'A ||--o{ A : r\n'.repeat(count),{limit:1000000});}
for(const count of [1,50,500,5000])add(`attribute-stress/${count}`,'erDiagram\nA {\n'+Array.from({length:count},(_,i)=>'string n'+i+' PK "Comment"').join('\n')+'\n}',{limit:1000000,outputLimit:10000000});
for(const count of [1,50,500,5000])add(`generic-stress/${count}`,'erDiagram\nA { '+ 'a~'.repeat(count)+',,,'+'~b'.repeat(count)+' value }',{limit:1000000,outputLimit:10000000});
for(const source of [base,'erDiagram\nA[fa:fa-user Person] { string name "fa:fa-user User" }\nA ||--|| B : fa:fa-user'])add(`fixed-measurer/${cases.length}`,source,{}, {measure:'fixed',icons:{'fa:fa-user':'<path d="M0 0L4 4"/>'}});
await writeFile('fixtures/pele/er.json',JSON.stringify({revision,phase:'model-and-render',provenance,exclusions,sourceModelExclusions:['subgraphLookup: private lookup cache','claimed: private first-owner bookkeeping'],dynamicCampaigns:{specTemplateAudit:dynamicAudit,stressCases:stressSources.length+1,securityMutations:{seed:3,operations:4000},cardinalities:cardinalities.length,mutations:{seed:7,operations:5000},randomTokens:{seed:8,operations:5000},securityTemplates:templates.length,payloads:payloads.length},cases},null,2)+'\n');console.log(`${cases.length} ER scenarios: ${cases.filter(c=>c.expected.model).length} models, ${cases.filter(c=>c.expected.result).length} renders, ${cases.filter(c=>c.expected.error).length} errors`);
