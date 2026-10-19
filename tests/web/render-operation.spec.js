import {test,expect} from '@playwright/test';

test('stateful WASM render outcomes retain graph ownership through failure and recovery',async({page})=>{
  await page.goto('/examples/web/');
  const report=await page.evaluate(async()=>{
    const {loadRenderer,metricsMeasurer,PeleError}=await import('/dist/package-web/package/full.js');
    const oracle=await import('/dist/evidence/pele-oracle.js');
    const engine=await loadRenderer(),wasm=await import('/dist/package-web/package/pkg/pele.js');
    const parsed=wasm.parse_v2('info title Retained state','{}'),document=parsed.to_public();
    parsed.free();
    const value=json=>{const envelope=JSON.parse(json);if(!envelope.ok)throw envelope.error;return envelope.result;};
    try {
      const first=value(document.render_operation_with_resources('0','{}',metricsMeasurer.width,undefined));
      const stale=JSON.parse(document.render_operation_with_resources('0','{}',metricsMeasurer.width,undefined));
      const limited=value(document.render_operation_with_resources('1','{"outputLimit":1}',metricsMeasurer.width,undefined));
      const failed=value(document.render_operation_with_resources('2','{}',()=>{throw new Error('host failure');},undefined));
      const snapshot=value(document.snapshot());
      const recovered=value(document.render_operation_with_resources('3','{}',metricsMeasurer.width,undefined));
      const model=engine.parse('info title Public recovery'),reference=oracle.parse('info title Public recovery');
      const failures=[];
      for(const sentinel of [undefined,null,false,new Error('identity'),new RangeError('guard')]) {
        let expected;
        try{oracle.render(reference,{measurer:{width(){throw sentinel;}}});}catch(error){expected=error;}
        try{engine.render(model,{measurer:{width(){throw sentinel;}}});failures.push('missing callback throw');}
        catch(error){
          if(sentinel instanceof RangeError){
            const fields=e=>({name:e?.name,message:e?.message,code:e?.code,type:e?.type,line:e?.line,column:e?.column,snippet:e?.snippet});
            if(!(error instanceof PeleError)||error.code!=='limit'||JSON.stringify(fields(error))!==JSON.stringify(fields(expected)))failures.push('pinned RangeError guard');
          }else if(error!==sentinel||expected!==sentinel)failures.push('callback value');
        }
        if(!engine.render(model,{measurer:metricsMeasurer}).svg.includes('Public recovery'))failures.push('recovery');
      }
      engine.dispose(model);
      return {revisions:[first.revision,limited.revision,failed.revision,snapshot.revision,recovered.revision],stale:stale.error.message,initial:first.result.Ok.type,limit:limited.result.Err.value.code,host:failed.result.Err.value.message,retained:JSON.stringify(first.graph)===JSON.stringify(snapshot.graph),recovered:recovered.result.Ok.svg.includes('Retained state'),failures};
    }finally{document.free();}
  });
  expect(report).toEqual({revisions:['1','2','3','3','4'],stale:'Stale public model revision',initial:'info',limit:'limit',host:'host failure',retained:true,recovered:true,failures:[]});
});
