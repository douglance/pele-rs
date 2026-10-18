import {test,expect} from '@playwright/test';

for(const zone of ['UTC','America/New_York','Asia/Kolkata','Australia/Lord_Howe','Pacific/Apia']) {
  test.describe(zone,()=>{
    test.use({timezoneId:zone});
    test('Gantt scheduling matches pinned public API through DST and calendar boundaries',async({page})=>{
      await page.goto('/examples/web/');
      const report=await page.evaluate(async zone=>{
        const {loadRenderer}=await import('/dist/package-web/package/full.js');
        const oracle=await import('/dist/evidence/pele-oracle.js');
        const engine=await loadRenderer();
        const fixture=await(await fetch('/fixtures/pele/calendar.json')).json();
        const measurer={width:(text,size,style)=>text.length*size*(style&1?.71:.6)};
        const mismatches=[];
        const canonical=value=>JSON.stringify(value,(_,v)=>v&&typeof v==='object'&&!Array.isArray(v)?Object.fromEntries(Object.entries(v).sort(([a],[b])=>a.localeCompare(b))):v);
        for(const c of fixture.cases.filter(c=>c.zone===zone)) {
          const options={...c.options,measurer};
          const run=render=>{try{return {result:render()}}catch(e){return {error:{name:e.name,message:e.message,code:e.code,type:e.type,line:e.line,column:e.column,snippet:e.snippet}}}};
          const expected=run(()=>oracle.render(c.source,options));
          const actual=run(()=>engine.render(c.source,options));
          if(canonical(actual)!==canonical(expected))mismatches.push(c.id);
          const model=engine.parse(c.source);
          try{if(canonical(run(()=>engine.render(model,options)))!==canonical(expected))mismatches.push(`${c.id}/retained`)}finally{engine.dispose(model)}
        }
        return mismatches;
      },zone);
      expect(report).toEqual([]);
    });
  });
}
