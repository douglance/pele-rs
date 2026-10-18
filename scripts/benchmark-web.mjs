// Public API baseline. Run after build-wasm.sh; reuses or starts a local server.
// Includes WASM/JS adapter costs. Excludes module startup and SVG/DOM painting.
import {chromium} from '@playwright/test';
import {mkdir,writeFile,readFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {execFileSync,spawn} from 'node:child_process';
const endpoint='http://127.0.0.1:5191/examples/web/';
let server;
try {await fetch(endpoint)} catch {
  server=spawn(process.execPath,['scripts/serve.mjs'],{stdio:'ignore'});
  let ready=false;
  for(let i=0;i<50;i++){try {if((await fetch(endpoint)).ok){ready=true;break}}catch{}await new Promise(resolve=>setTimeout(resolve,100))}
  if(!ready){server.kill();throw new Error('Benchmark server did not start')}
}
let browser,deadline;
try {
  browser=await chromium.launch(process.env.CI?{}:{channel:'chrome'});
  const page=await browser.newPage();
  await page.goto('http://127.0.0.1:5191/examples/web/');
  const report=await Promise.race([
    new Promise((_,reject)=>{deadline=setTimeout(()=>reject(new Error('Benchmark exceeded 60 second work budget')),60_000)}),
    page.evaluate(async()=>{
      const [{loadRenderer},upstream]=await Promise.all([import('/dist/package-web/package/full.js'),import('/dist/evidence/pele-oracle.js')]);
      const engine=await loadRenderer(), cases=[];
      for(const scale of ['small','large']) {
        const count=scale==='small'?5:150;
        cases.push({type:'pie',scale,source:'pie title Benchmark\n'+Array.from({length:count},(_,i)=>`"Section ${i}":${i+1}`).join('\n')});
        cases.push({type:'packet',scale,source:'packet-beta\ntitle Benchmark\n'+Array.from({length:scale==='small'?8:512},(_,i)=>`+8: "Field ${i}"`).join('\n')});
        const axes=Array.from({length:scale==='small'?5:32},(_,i)=>`a${i}`);
        cases.push({type:'radar',scale,source:'radar-beta\ntitle Benchmark\naxis '+axes.join(',')+'\ncurve '+Array.from({length:scale==='small'?2:8},(_,i)=>`c${i}{${axes.map((_,j)=>(i+j)%9+1).join(',')}}`).join(',')});
        cases.push({type:'quadrantChart',scale,source:'quadrantChart\ntitle Benchmark\n'+Array.from({length:scale==='small'?5:250},(_,i)=>`Point ${i}: [${((i%100)/100).toFixed(2)},${((i*7%100)/100).toFixed(2)}]`).join('\n')});
      }
      const batch=(work,count)=>{const start=performance.now();for(let i=0;i<count;i++)work();return performance.now()-start};
      const quantile=(values,f)=>{const sorted=[...values].sort((a,b)=>a-b);return sorted[Math.ceil(sorted.length*f)-1]};
      const rows=[];let checksum=0;
      for(const item of cases) {
        const options={limit:500_000},nativeModel=engine.parse(item.source,options),jsModel=upstream.parse(item.source,options);
        if(engine.render(nativeModel,options).svg!==upstream.render(jsModel,options).svg)throw new Error(`${item.type}/${item.scale}: output mismatch`);
        try {
          for(const stage of ['parse','render-source','render-model']) {
            const work={
              wasm:stage==='parse'?()=>{const m=engine.parse(item.source,options);checksum+=m.type.length;engine.dispose(m)}:()=>{const r=engine.render(stage==='render-source'?item.source:nativeModel,options);checksum+=r.svg.length+r.width+r.height},
              js:stage==='parse'?()=>{checksum+=upstream.parse(item.source,options).type.length}:()=>{const r=upstream.render(stage==='render-source'?item.source:jsModel,options);checksum+=r.svg.length+r.width+r.height}
            };
            // Warm both APIs, calibrate each independently, then alternate sample order.
            const counts={};
            for(const key of ['js','wasm']) {
              for(let i=0;i<24;i++)work[key]();
              let count=1;while(count<256&&batch(work[key],count)<15)count*=2;counts[key]=count;
            }
            const samples={js:[],wasm:[]};
            for(let trial=0;trial<9;trial++)for(const key of trial%2?['wasm','js']:['js','wasm'])samples[key].push(batch(work[key],counts[key])/counts[key]);
            const jsMedian=quantile(samples.js,.5),wasmMedian=quantile(samples.wasm,.5);
            rows.push({type:item.type,scale:item.scale,stage,sourceBytes:new TextEncoder().encode(item.source).length,jsMedianMs:jsMedian,wasmMedianMs:wasmMedian,jsP95Ms:quantile(samples.js,.95),wasmP95Ms:quantile(samples.wasm,.95),wasmOverJs:wasmMedian/jsMedian,batchCounts:counts,samples});
          }
        } finally {engine.dispose(nativeModel)}
      }
      return {userAgent:navigator.userAgent,scope:'Warm public APIs with browser Canvas widths; includes JS/WASM adapter, options/result JSON and ownership. Reads returned metadata/SVG length; excludes loading, SVG DOM insertion, layout/paint, native Swift and GPU.',checksum,rows};
    })
  ]);
  clearTimeout(deadline);
  report.timestamp=new Date().toISOString();report.browser=browser.version();report.revision=execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim();
  report.worktree=execFileSync('git',['status','--porcelain'],{encoding:'utf8'}).trim();report.platform={os:process.platform,arch:process.arch,node:process.version};report.upstream='458322a5b8abb1ae57ddf127c32017ab9fb14a92';
  report.runtimeDigests={};
  for(const path of ['dist/package-web/package/pkg/pele_bg.wasm','dist/package-web/package/pkg/pele.js','dist/package-web/package/full.js','dist/package-web/package/resources.js','dist/evidence/pele-oracle.js'])report.runtimeDigests[path]=createHash('sha256').update(await readFile(path)).digest('hex');
  await mkdir('dist/evidence',{recursive:true});await writeFile('dist/evidence/browser-benchmark.json',JSON.stringify(report,null,2)+'\n');
  console.table(report.rows.map(({type,scale,stage,jsMedianMs,wasmMedianMs,wasmOverJs})=>({type,scale,stage,jsMs:jsMedianMs.toFixed(4),wasmMs:wasmMedianMs.toFixed(4),wasmOverJs:wasmOverJs.toFixed(2)})));
} finally {clearTimeout(deadline);await browser?.close();server?.kill()}
