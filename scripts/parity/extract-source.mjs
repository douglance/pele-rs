// Development-only oracle. The distributed Rust/WASM/native library never executes upstream JS.
import {readFile,writeFile,mkdir,readdir} from 'node:fs/promises';
import {stripTypeScriptTypes} from 'node:module';
import {pathToFileURL} from 'node:url';
import {resolve} from 'node:path';
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
const revision='458322a5b8abb1ae57ddf127c32017ab9fb14a92';
const root=resolve(process.argv[2] ?? '/tmp/pele-upstream-458322a5');
assert.equal(execFileSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).trim(),revision,'Wrong Pele oracle revision');
const out=resolve('dist/parity/oracle');
await mkdir(`${out}/util`,{recursive:true});
await mkdir(`${out}/text`,{recursive:true});
execFileSync('git',['diff','--quiet',revision,'--','src','tests'],{cwd:root});
const provenance=[];
for(const file of ['errors.ts','detect.ts','preprocess.ts','util/chars.ts','util/yaml.ts','text/entities.ts']){
  const source=await readFile(`${root}/src/${file}`,'utf8');
  provenance.push({path:`src/${file}`,sha256:createHash('sha256').update(source).digest('hex')});
  await writeFile(`${out}/${file.replace(/\.ts$/,'.js')}`,stripTypeScriptTypes(source,{mode:'transform',sourceUrl:`pele/${file}`}));
}
const {preprocess,encodeEntities}=await import(pathToFileURL(`${out}/preprocess.js`));
const {detect}=await import(pathToFileURL(`${out}/detect.js`));
const {detectType:publicDetect}=await import('pele');
const cases=[];
function add(id,source,options={}){
  assert.equal(typeof source,'string',`Non-string corpus source: ${id}`);
  let expected;
  try{
    if(source.length>(options.limit??50000))throw Object.assign(new Error(`Diagram source is longer than the limit of ${options.limit??50000} characters.`),{name:'PeleError',code:'limit',type:null,line:0,column:0,snippet:''});
    const prepared=preprocess(source);
    prepared.config={...options.config,...prepared.config};
    const type=detect(prepared.text);
    assert.equal(type,publicDetect(source));
    expected={type,registered:false,prepared,encodedText:encodeEntities(prepared.text)};
  }catch(error){if(!['syntax','semantic','limit','unsupported-diagram'].includes(error.code))throw error;expected={error:{name:error.name,message:error.message,code:error.code,type:error.type,line:error.line,column:error.column,snippet:error.snippet}}}
  cases.push({id,source,options,expected});
}
for(const file of (await readdir(`${root}/tests/corpus`)).filter(f=>f.endsWith('.json')).sort()){
  const sources=JSON.parse(await readFile(`${root}/tests/corpus/${file}`,'utf8'));
  provenance.push({path:`tests/corpus/${file}`,sha256:createHash('sha256').update(await readFile(`${root}/tests/corpus/${file}`)).digest('hex'),cases:sources.length});
  sources.forEach((entry,i)=>add(`corpus/${file}/${i}`,typeof entry==='string'?entry:entry.src));
}
// Exercise metadata through the same exported preparation interface as complete documents.
const yamlTest=await readFile(`${root}/tests/util/yaml.test.ts`,'utf8');
const yamlExpression=yamlTest.slice(yamlTest.indexOf('const CASES = ')+14,yamlTest.indexOf('\n];')+2);
const {default:yamlCases}=await import(`data:text/javascript,export default ${encodeURIComponent(yamlExpression)}`);
yamlCases.forEach((body,i)=>add(`yaml/upstream/${i}`,`---\nconfig:\n${body.split('\n').map(l=>'  '+l).join('\n')}\n---\ngraph TD\nA`));
const metadata=[
 'title: [hello, null, [one, two]]\ndisplayMode: true\nconfig: {2: second, 10: tenth, 1: first, z: .nan, a: .inf, n: -.inf}',
 'title: 1e21\nconfig: { small: 1e-7, large: 1e21, zero: -0, hex: -0x1f, oct: +0o10, bin: 0b11, leading: 012, word: yes }',
 'title: false\nconfig: { prototype: {bad: 1}, __proto__: {bad: 1}, constructor: {bad: 1}, nested: {__proto__: 1, ok: true}}',
 'config:\n  list:\n  - one\n  - two\n  objects:\n    - key: value\n      next: 2',
 'title: |+\n  Hi\n\n  There\n\n\nconfig: {a: b}',
 'title: >-\n  One\n  two\n\n  paragraph\n    indented\n  end',
 'title: "a\\n\\t\\x41\\u0042\\U0001F642"',
 'title: "\\ud83d\\ude00"',
 'title: "\\q"', 'title: "\\U00110000"','config: {a: 1 b: 2}', 'config: [a,,b]',
 'config: {a: 1,, b: 2}', 'config: {a: [}', 'config: "unterminated',
 'config:\n \ta: 1', 'config:\n  a: 1\n   b: 2',
 'config: '+ '['.repeat(66)+'1'+']'.repeat(66),
 'title: 1\n...\nignored: true',
 'config: { key: "two\n  lines", list: [one,\n two] }',
 'config:\n  message: a plain\n    continued text\n\n    next paragraph\n  other: true',
];
metadata.forEach((body,i)=>add(`yaml/extended/${i}`,`---\n${body}\n---\npie\n"A": 1`));
const headers=['flowchart-elk','mindmap','architecture','agentflow-beta','C4Context','C4Container','C4Component','C4Dynamic','C4Deployment','kanban','classDiagram','erDiagram','gantt','info','pie','requirement','requirementDiagram','sequenceDiagram','swimlane-beta','graph','flowchart','timeline','gitGraph','stateDiagram','journey','quadrantChart','sankey','packet','xychart','block','eventmodeling','treeView-beta','radar-beta','ishikawa','ishikawa-beta','treemap','railroad-beta','railroad-ebnf-beta','railroad-abnf-beta','railroad-peg-beta','venn-beta','wardley-beta','cynefin-beta','usecase-beta'];
for(const h of headers)for(const suffix of ['', '\n','X',':','-','é','🙂'])for(const prefix of ['', '\ufeff\u2000','x '])add(`detect/${cases.length}`,prefix+h+suffix);
for(const h of headers)add(`detect/case/${h}`,h.toUpperCase());
const sourceCases=[
 '%% one\ngraph TD\n  %% two\n  A --> B\n','%%\ngraph TD','%% \ngraph TD',' \n %% comment\n\ngraph TD',
 '%%{wrap}%%\ngraph TD',"%%{init: {'flowchart': {'rankSpacing': 99}}}%%\ngraph TD",'%%{INIT: {"x":1}}%%\ngraph TD',
 '%%{init: invalid json}%%\ngraph TD', '%%{init: {"x":1}\ngraph TD', '%%{init: {"a":1}}%% %%{initialize:{"b":2}}%%\npie',
 '%%{no-body: body}%%\npie','%%{wrap: body more}%%\npie', '%%{wrap\u2028}%%\npie',
 'graph TD\r\nA["<span class="x">hi</span>"]\r', '<1 a="b" z="a\nb"> pie',
 '---\ntitle: Hello\n---\n%%{wrap}%%\npie', '  ---\n  title: Hello\n  ---\n  pie',
 '\n---\ntitle: Ignored\n---\npie','---\ntitle: Not closed\npie', '---\n---\npie','---\n\n---\n\npie',
 '---\nconfig: {flowchart: {curve: linear, rankSpacing: 10}}\n---\n%%{init: {"flowchart": {"rankSpacing": 99}}}%%\ngraph TD',
];
sourceCases.push('\n'.repeat(20000)+'%%{unknown\npie');
sourceCases.push('%%{init: {"big": 1e400, "small": -1e-9999}}%%\npie');
sourceCases.forEach((s,i)=>add(`prepare/${i}`,s));
for(const indent of [' ', '  ', '\u00a0', '\u2000', ' \u2000'])for(const chomp of ['|','|-','|+','>','>-','>+','|1','>2']){
  add(`yaml/unicode/${cases.length}`,`---\ntitle: ${chomp}\n${indent}first\n${indent}next\n\n${indent}third\n---\npie`);
}
const atoms=['true','False','NULL','~','0x1f','-0xFF','+0o77','0b11','012','-.5','.5','1e-9','1e400','.nan','-.inf','plain text','"a\\n b"',"'it''s text'",'"\\x41"','"\\u0000"','"\\N"','[1, 2, three]','{x:1, y: 2}','[a,,b]','{a: b, c}','[', '"bad\\q"'];
for(let i=0;i<500;i++){
  const a=atoms[i%atoms.length],b=atoms[Math.floor(i/atoms.length)%atoms.length];
  add(`yaml/combinations/${i}`,`---\nconfig:\n  x: ${a}\n  nested:\n    other: ${b}\n---\npie`);
}
add('config/shallow','---\nconfig: {pie: {a: 1}}\n---\npie',{config:{pie:{b:2},extra:true}});
for(const limit of [0,1,2,3,4])add(`limit/utf16/${limit}`,'🙂a',{limit});
const parts=['style','classDef',':','#',';',' ','\n','a','fill','9','\t',':#','#f9f;','\r','\u2028','é'];
let state=11;const rnd=()=>{state=(Math.imul(state,1664525)+1013904223)>>>0;return state/4294967296};
for(let i=0;i<2000;i++){let s='graph TD\n';for(let n=Math.floor(rnd()*14);n>0;n--)s+=parts[Math.floor(rnd()*parts.length)];add(`entities/generated/${i}`,s)}
// Directive/comment/frontmatter mutations exercise scanner precedence and malformed input.
const fragments=['%%{','wrap','init',':',' ','\n','}%%',"'",'"','{','}','%% comment\n','---\n','pie','\u2028'];
for(let i=0;i<1000;i++){let s='';for(let n=Math.floor(rnd()*14);n>0;n--)s+=fragments[Math.floor(rnd()*fragments.length)];add(`prepare/generated/${i}`,s)}
const fixture={revision,scope:'source preparation and detection only; no diagram rendering parity implied',provenance,cases};
await writeFile('fixtures/pele/source.json',JSON.stringify(fixture,null,2)+'\n');
console.log(`${cases.length} pinned public preparation scenarios; ${provenance.length} source records`);

const {decodeEntities}=await import(pathToFileURL(`${out}/text/entities.js`));
const entitySource=await readFile(`${root}/src/text/entities.ts`,'utf8');
const named=[...entitySource.slice(0,entitySource.indexOf('let named:')).matchAll(/'([^']*)'/g)].map(m=>m[1]).join('').split(',').map(p=>p.slice(0,p.indexOf(':')));
const entityInputs=named.flatMap(n=>[`#${n};`,`&${n};`,`ﬂ°${n}¶ß`]);
entityInputs.push('ﬂ°°57343.5¶ß', '&unknown; #unknown;', '&#0; &#55296; &#1114112; &#x1f642; &#X41;', 'ﬂ°°1.5¶ß', 'ﬂ°°-1¶ß ﬂ°°+65¶ß ﬂ°°0x41¶ß', 'ﬂ°broken¶ then ﬂ°amp¶ß', '&amp;#65;', 'ﬂ°amp¶ß#65;');
for(let i=0;i<1000;i++){let s='';for(let n=Math.floor(rnd()*12);n>0;n--)s+=['&','#',';','ﬂ°','°','¶','ß','amp','35','x1f642',' ',':','style',':#f9f;'][Math.floor(rnd()*14)];entityInputs.push(s)}
const entityCases=entityInputs.map((input,i)=>{
  const source=`extension\n${input}`,encoded=encodeEntities(preprocess(source).text)+'\n';
  let expected;try{expected={value:decodeEntities(encoded)}}catch(error){if(!(error instanceof RangeError))throw error;expected={error:'limit'}}
  return{id:`entity-link/${i}`,source,expected};
});
await writeFile('fixtures/pele/entity-links.json',JSON.stringify({revision,cases:entityCases},null,2)+'\n');
console.log(`${entityCases.length} link/entity scenarios`);
