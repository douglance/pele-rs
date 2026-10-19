import {test,expect} from '@playwright/test';
test('packaged WASM prepares the entire pinned source corpus',async({page})=>{
  await page.goto('/examples/web/');
  const report=await page.evaluate(async()=>{
    const {loadCore}=await import('/dist/package-web/package/core.js');
    const engine=await loadCore();
    const fixture=await (await fetch('/fixtures/pele/source.json')).json();
    const mismatches=[];
    const canonical=v=>JSON.stringify(v,(_,value)=>value&&typeof value==='object'&&!Array.isArray(value)?Object.fromEntries(Object.entries(value).sort(([a],[b])=>a.localeCompare(b))):value);
    for(const c of fixture.cases){let actual;try{actual=engine.inspect(c.source,c.options)}catch(e){actual={error:{name:e.name,message:e.message,code:e.code,type:e.type,line:e.line,column:e.column,snippet:e.snippet}}}if(canonical(actual)!==canonical(c.expected))mismatches.push(c.id)}
    const {detectType,supports}=engine;
    return{count:fixture.cases.length,mismatches,capabilities:engine.capabilities(),detect:detectType('pie'),supports:supports('pie')};
  });
  expect(report.count).toBeGreaterThan(4800);expect(report.mismatches).toEqual([]);
  expect(report.capabilities).toHaveLength(33);expect(report.capabilities.every(c=>c.known&&!c.registered&&!c.renderParity)).toBe(true);
  expect(report.detect).toBe('pie');expect(report.supports).toBe(false);
});
