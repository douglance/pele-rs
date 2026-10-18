import {test,expect} from '@playwright/test';
test('full pie corpus matches pinned models/SVG through source and owned model APIs',async({page})=>{
  await page.goto('/examples/web/');
  const report=await page.evaluate(async()=>{
    const {loadRenderer,metricsMeasurer}=await import('/dist/package-web/package/full.js');const engine=await loadRenderer();
    const fixture=await(await fetch('/fixtures/pele/pie.json')).json();const mismatches=[];
    const canonical=v=>JSON.stringify(v,(_,value)=>value instanceof Map?[...value]:value&&typeof value==='object'&&!Array.isArray(value)?Object.fromEntries(Object.entries(value).sort(([a],[b])=>a.localeCompare(b))):value);
    for(const c of fixture.cases){let actual,model;const options={...c.options,measurer:metricsMeasurer,...(c.host.measure==='fixed'?{measurer:{width:(text,size,style)=>text.length*size*(style&1?.71:.6)}}:{}),...(c.host.icons?{icons:n=>c.host.icons[n]}:{})};
      try{model=engine.parse(c.source,c.options);actual={model,result:engine.render(model,options)};if(canonical(actual.result)!==canonical(engine.render(c.source,options)))mismatches.push(`${c.id}: source/model differ`)}catch(e){actual={error:{name:e.name,message:e.message,code:e.code,type:e.type,line:e.line,column:e.column,snippet:e.snippet}}}finally{if(model)engine.dispose(model)}
      if(canonical(actual)!==canonical(c.expected))mismatches.push(c.id);
    }
    let forged=false;try{engine.render({type:'pie',sections:new Map()})}catch(e){forged=e instanceof TypeError}
    const model=engine.parse('pie\n"one":1');const retained=engine.render(model,{limit:0});engine.dispose(model);
    return{count:fixture.cases.length,mismatches,forged,retained:retained.type,capabilities:engine.capabilities()};
  });
  expect(report.count).toBeGreaterThan(3600);expect(report.mismatches).toEqual([]);expect(report.forged).toBe(true);expect(report.retained).toBe('pie');expect(report.capabilities.filter(c=>c.registered).map(c=>c.type)).toEqual(['flowchart','pie','er','mindmap','kanban','timeline','journey','class','quadrantChart','xychart','state','gantt','architecture','requirement','sankey','packet','radar','treemap','info','sequence','gitGraph','c4','block','railroad']);
});
test('native export paints colors and preserves authored text in real browser SVG',async({page})=>{
  await page.goto('/examples/web/');
  const proof=await page.evaluate(async()=>{
    const {loadRenderer}=await import('/dist/package-web/package/full.js');const engine=await loadRenderer();
    const diagram=engine.render('pie title Shared Rust\n"Capture":2\n"Keep":5\n"Use":3',{nativeTheme:{}});
    const host=document.createElement('div');document.body.append(host);host.innerHTML=diagram.svg;
    const image=new Image();image.src='data:image/svg+xml;base64,'+btoa(unescape(encodeURIComponent(diagram.svg)));await image.decode();
    const canvas=document.createElement('canvas');canvas.width=diagram.width;canvas.height=diagram.height;const ctx=canvas.getContext('2d');ctx.drawImage(image,0,0);
    const pixels=ctx.getImageData(0,0,canvas.width,canvas.height).data;let colored=0;for(let i=0;i<pixels.length;i+=4)if(pixels[i+3]>200&&Math.max(pixels[i],pixels[i+1],pixels[i+2])-Math.min(pixels[i],pixels[i+1],pixels[i+2])>40)colored++;
    host.remove();return{colored,width:diagram.width,height:diagram.height,unresolved:diagram.svg.includes('var(')};
  });expect(proof.unresolved).toBe(false);expect(proof.colored).toBeGreaterThan(10000);
});

test('browser Canvas widths and host callback failures match upstream public behavior',async({page})=>{
  await page.goto('/examples/web/');
  const report=await page.evaluate(async()=>{
    const [{loadRenderer,forgetTextWidths},oracle]=await Promise.all([import('/dist/package-web/package/full.js'),import('/dist/evidence/pele-oracle.js')]);const engine=await loadRenderer();let mismatches=[];
    for(const family of ['sans-serif','Arial','serif','monospace'])for(const maxWidth of [undefined,240,170])for(const fontSize of [12,16,20]){
      const options={fontFamily:family,fontSize,...(maxWidth?{maxWidth}:{})};
      const source='pie title Browser font metrics\n"A longer label":2\n"<b>Strong</b> emphasis":3\n"中文 🙂":1';
      if(engine.render(source,options).svg!==oracle.render(source,options).svg)mismatches.push({family,maxWidth,fontSize});
    }
    forgetTextWidths();oracle.forgetTextWidths();
    let original=false,range=false;
    const failure=new Error('Host failure');try{engine.render('pie title Test',{measurer:{width(){throw failure}}})}catch(e){original=e===failure}
    try{engine.render('pie title Test',{measurer:{width(){throw new RangeError('Host depth')}}})}catch(e){range=e.code==='limit'&&e.message==='Diagram is nested too deeply.'}
    return{mismatches,original,range};
  });expect(report.mismatches).toEqual([]);expect(report.original).toBe(true);expect(report.range).toBe(true);
});
