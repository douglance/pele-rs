// Development-only pinned Pele registry oracle; never a consumer runtime.
// Usage: node scripts/parity/extract-registry.mjs [--upstream PATH] [--out PATH]
// --repo PATH overrides the script-relative repository for isolated preparation.
import {readFile,writeFile,readdir} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {pathToFileURL,fileURLToPath} from 'node:url';
import {resolve} from 'node:path';
import assert from 'node:assert/strict';
const args=process.argv.slice(2);
const option=(flag,fallback)=>{const index=args.indexOf(flag);if(index<0)return fallback;assert.ok(args[index+1]&&!args[index+1].startsWith('--'),`Missing ${flag} value`);return args[index+1]};
const script=fileURLToPath(import.meta.url);
const repo=resolve(option('--repo',fileURLToPath(new URL('../../',import.meta.url))));
const upstream=resolve(option('--upstream','/tmp/pele-upstream-458322a5'));
const output=resolve(option('--out',repo+'/fixtures/pele/registry.json'));
const worker=option('--worker',undefined);
const contract={kind:'owned-selective-builtin-registry',source:'well-formed UTF-8',identifiers:'canonical built-ins and exact unknown strings',budgets:'Existing native source/preparation/work limits remain independent',exclusions:['arbitrary JavaScript object coercion','lazy loading','mount/zoom','constructor capability parity']};
const revision='458322a5b8abb1ae57ddf127c32017ab9fb14a92';
const sha=b=>createHash('sha256').update(b).digest('hex');
const unknown=['','unknown','PIE','Pie','sequenceDiagram','flowchart-elk','swimlane-beta','__proto__','constructor','toString'];
const headings={flowchart:'flowchart TD\nA-->B',sequence:'sequenceDiagram\nA->>B: Hi',class:'classDiagram\nclass A',state:'stateDiagram-v2\nA-->B',er:'erDiagram\nA',pie:'pie\n"A":1',gantt:'gantt\ndateFormat YYYY-MM-DD\nA:2024-01-01,1d',gitGraph:'gitGraph\ncommit',mindmap:'mindmap\n  root',timeline:'timeline\n2024: Start',journey:'journey\nsection S\nTask:5:A',xychart:'xychart-beta\nline [1]',quadrantChart:'quadrantChart',requirement:'requirementDiagram',c4:'C4Context\nPerson(a,"A")',sankey:'sankey\na,b,1',block:'block-beta\na',packet:'packet-beta\n0:"A"',kanban:'kanban\n  a',architecture:'architecture-beta\nservice a(server)[A]',radar:'radar-beta\naxis A',treemap:'treemap\n"A":1',info:'info title Registry',agentflow:'agentflow-beta TB\nA-->B',usecase:'usecase-beta\nactor A',railroad:'railroad-beta\nr = "a";',treeView:'treeView-beta\nroot',venn:'venn-beta\nA:1',cynefin:'cynefin-beta',wardley:'wardley-beta\ncomponent A [0.1,0.2]',swimlane:'swimlane-beta TB\nA-->B',ishikawa:'ishikawa-beta\nCause --> Effect',eventmodeling:'eventmodeling\nactor A'};
const cases=Object.entries(headings).map(([type,source])=>({id:'canonical/'+type,source,parse:['flowchart','pie','info'].includes(type)}));
cases.push(...[
 ['invalid-flow','flowchart TD\nA-->',true],['invalid-pie','pie\n"A":?',true],
 ['frontmatter','---\ntitle: Front\n---\npie\n"A":1',true],
 ['invalid-yaml','---\ntitle: [\n---\npie\n"A":1',true],
 ['invalid-init-ignored',"%%{init: {broken}}%%\npie\n\"A\":1",true],
 ['unclosed-frontmatter','---\ntitle: Front\npie\n"A":1',true],
 ['unmatched-bom-fence','\ufeff---\r\ntitle: Front\r\n---\r\npie\r\n"A":1',true],
 ['unknown','not a diagram',true],['empty','',true],['comment','%% comment only',true],
 ['comment-prefix','%% author\r\nflowchart TD\r\nA-->B',true],
 ['c4-unanchored','prefix C4Container elsewhere',true],['flowchart-prefix','flowchart-elk-extra',false],
 ['swimlane-word-boundary','swimlane-betax\nA-->B',true],
 ['long-discovery','%%'+'x'.repeat(50001)+'\npie\n"A":1',true],
].map(([id,source,parse])=>({id,source,parse})));
const snapshot=(api,types)=>Object.fromEntries([...types,...unknown].map(type=>[type,api.registered(type)]));
const outcome=run=>{try{return{value:run()}}catch(e){return{error:{name:e.name,code:e.code??null,message:e.message,type:e.type??null,line:e.line??null,column:e.column??null,snippet:e.snippet??null}}}};
if(worker!==undefined){
  process.env.TZ='UTC';Date.now=()=>1704067200000;
  const mode=worker;
  const types=Object.keys(headings);
  const api=await import(pathToFileURL(repo+'/node_modules/pele/dist/'+(mode==='full'?'pele.js':'esm/core.js')).href);
  const selection=mode==='all'||mode==='full'?types:mode==='pie'?['pie']:mode==='flow-info'?['flowchart','info']:[];
  const before=snapshot(api,types);
  if(mode!=='full')for(const type of selection){const module=await import(pathToFileURL(repo+'/node_modules/pele/dist/esm/diagrams/'+type+'.js').href);assert.equal(module.default.type,type);assert.equal(api.register(module.default),undefined)}
  const registered=snapshot(api,types);
  assert.equal(api.register(),undefined);assert.deepEqual(snapshot(api,types),registered);
  if(mode!=='full')for(const type of selection){const module=await import(pathToFileURL(repo+'/node_modules/pele/dist/esm/diagrams/'+type+'.js').href);api.register(module.default)}
  assert.deepEqual(snapshot(api,types),registered);
  const observed=cases.map(c=>({id:c.id,source:c.source,expected:{detection:outcome(()=>api.detectType(c.source)),supports:api.supports(c.source),...(c.parse?{parse:outcome(()=>{const m=api.parse(c.source);return{type:m.type}})}:{})}}));
  let ownership;
  if(selection.includes('pie')){
    const source='pie title Retained\n"A":1';
    const model=api.parse(source,{config:{pie:{useMaxWidth:false}}});
    const measurer={width:(text,size,style)=>text.length*size*(style&1?.71:.6)};
    const before=api.render(model,{measurer});
    if(mode!=='full'){const module=await import(pathToFileURL(repo+'/node_modules/pele/dist/esm/diagrams/pie.js').href);api.register(module.default)}
    model.type='foreign-type';
    const after=api.render(model,{config:{pie:{useMaxWidth:true}},measurer});
    assert.deepEqual(after,before);
    const fresh=api.render(source,{measurer});assert.notEqual(fresh.svg,before.svg);
    ownership={source,modelOwnType:model.type,expected:{before,after,fresh}};
  }
  process.stdout.write(JSON.stringify({id:mode,selection,before,registered,noArgumentRegistrationUnchanged:true,duplicateRegistrationUnchanged:true,cases:observed,...(ownership?{ownership}:{})}));
}else{
  assert.equal(execFileSync('git',['rev-parse','HEAD'],{cwd:upstream,encoding:'utf8'}).trim(),revision);
  execFileSync('git',['diff','--quiet',revision,'--','src'],{cwd:upstream});
  const provenance={};for(const p of['src/core.ts','src/index.ts','src/diagrams/registry.ts','src/detect.ts','src/preprocess.ts','tests/entries.test.ts'])provenance[p]=sha(await readFile(upstream+'/'+p));
  const oracleSha256=sha(await readFile(repo+'/node_modules/pele/dist/pele.js'));assert.equal(oracleSha256,'d1ebc1889f3069d7759fe5db34c4ad4b4823285edbee2b537d0ef3d505a76d8d');
  const artifacts={};
  for(const folder of['esm','esm/chunks','esm/diagrams'])for(const f of await readdir(repo+'/node_modules/pele/dist/'+folder)){if(f.endsWith('.js')){const path=folder+'/'+f;artifacts[path]=sha(await readFile(repo+'/node_modules/pele/dist/'+path))}}
  const env={PATH:process.env.PATH??'/usr/bin:/bin',TZ:'UTC',LANG:'C',LC_ALL:'C'};
  const registries=[];for(const mode of['empty','pie','flow-info','all','full'])registries.push(JSON.parse(execFileSync(process.execPath,[script,'--worker',mode,'--repo',repo],{env,maxBuffer:8*1024*1024,encoding:'utf8'})));
  assert.deepEqual(registries.find(r=>r.id==='all').cases,registries.find(r=>r.id==='full').cases);
  const fixture={contract,revision,oracleSha256,provenance,artifacts,host:{measure:'fixedUTF16',now:1704067200000,timezone:'UTC'},scope:'Built-in canonical type selection, exact string membership, well-formed UTF8 source. No arbitrary object coercion, lazy/mount/runtime lifecycle or constructor capability claims.',registries};
  await writeFile(output,JSON.stringify(fixture,null,2)+'\n');
  console.log(JSON.stringify({registries:registries.length,casesPerRegistry:cases.length,canonicalTypes:Object.keys(headings).length,ownershipVectors:registries.filter(r=>r.ownership).length,fixture:output,fixtureSha256:sha(JSON.stringify(fixture,null,2)+'\n')}));
}
