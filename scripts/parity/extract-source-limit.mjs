// Development-only pinned Pele source-limit oracle. Consumer runtime is Rust.
process.env.TZ='UTC';
import {parse,render} from 'pele';
import {readFile,writeFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import assert from 'node:assert/strict';
const revision='458322a5b8abb1ae57ddf127c32017ab9fb14a92',root=process.argv[2]??'/tmp/pele-upstream-458322a5';
assert.equal(execFileSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).trim(),revision);
execFileSync('git',['diff','--quiet',revision,'--','src/core.ts','src/errors.ts','src/types.ts','src/preprocess.ts'],{cwd:root});
const sha=bytes=>createHash('sha256').update(bytes).digest('hex');
const provenance={};for(const path of ['src/core.ts','src/errors.ts','src/types.ts','src/preprocess.ts'])provenance[path]=sha(await readFile(root+'/'+path));
const oracleSha256=sha(await readFile('node_modules/pele/dist/pele.js'));
assert.equal(oracleSha256,'d1ebc1889f3069d7759fe5db34c4ad4b4823285edbee2b537d0ef3d505a76d8d');
const now=1704067200000;Date.now=()=>now;
const options={limit:0,maxEdges:3,config:{pie:{legendPosition:'left'}}};
const host={measure:'fixed',now};
const measurer={width:(text,size,style)=>text.length*size*(style&1?.71:.6)};
const bits=n=>{const b=Buffer.alloc(8);b.writeDoubleBE(n);return b.toString('hex')};
const number=n=>({kind:'number',bits:bits(n)}),undefinedLimit={kind:'undefined'},nullLimit={kind:'null'};
const decode=limit=>{if(limit.kind==='undefined')return undefined;if(limit.kind==='null')return null;return Buffer.from(limit.bits,'hex').readDoubleBE()};
const normalized=v=>JSON.parse(JSON.stringify(v,(_,x)=>x instanceof Map?[...x]:x));
const cases=[];
function add(id,source,limit){let expected;try{const model=parse(source,{...options,limit:decode(limit)});const result=render(model,{...options,now,measurer});expected={type:model.type,model:normalized(model),result,widthBits:bits(result.width),heightBits:bits(result.height)}}catch(e){assert.equal(e.name,'PeleError');expected={error:{name:e.name,code:e.code,message:e.message,type:e.type,line:e.line,column:e.column,snippet:e.snippet}}}cases.push({id,source,limit,expected})}
const seeds=[['ascii','pie title Source\n"A":1\n"B":2'],['astral','pie title 😀\n"é😀":1'],['bom-crlf','\ufeffpie\r\n"😀":1\r\n'],['frontmatter','\ufeff---\r\ntitle: Front\r\nconfig:\r\n  pie:\r\n    useMaxWidth: false\r\n---\r\n%% author\r\npie\r\n"A":1\r\n'],['entity-comment','%% #35; 😀\npie title #amp;\n"#35;":1'],['empty',''],['unknown','unknown text'],['syntax','pie\n"x":?']];
const values=[undefinedLimit,nullLimit,...[NaN,Infinity,-Infinity,0,-0,-1,-.5,.5,1,Number.MIN_VALUE,-Number.MIN_VALUE,-1e-7,-1e21,Number.MAX_VALUE,Number.MAX_SAFE_INTEGER,49999.5,50000,50000.5].map(number),{kind:'number',bits:'fff8000000000042'}];
for(const[id,source]of seeds){for(let i=0;i<values.length;i++)add(`${id}/${i}`,source,values[i]);for(const delta of[-.5,0,.5])add(`${id}/length-${delta}`,source,number(source.length+delta))}
const prefix='pie\n"A":1\n%%';for(const length of[50000,50001,50020]){const source=prefix+'x'.repeat(length-prefix.length);assert.equal(source.length,length);for(const[name,limit]of[['default',undefinedLimit],['infinity',number(Infinity)],['nan',number(NaN)],['equal',number(length)],['fraction-below',number(length-.5)]])add(`default-boundary/${length}/${name}`,source,limit)}
await writeFile('fixtures/pele/source-limit.json',JSON.stringify({revision,oracleSha256,provenance,options,host,scope:'Well-formed source and exact source-limit arguments only; maxEdges/outputLimit retain legacy APIs; malformed UTF16 and arbitrary coercion excluded.',cases},null,2)+'\n');
console.log(`${cases.length} pinned source-limit cases; ${cases.filter(c=>c.expected.type).length} full model/SVG, ${cases.filter(c=>c.expected.error).length} complete errors`);
