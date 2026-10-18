import {test,expect} from '@playwright/test';

for(const family of ['packet','radar','quadrant','timeline','journey','flowchart','kanban','mindmap','class','er','xychart']) {
  test(`${family} pinned corpus through WASM source and owned model APIs`, async({page})=>{
    test.setTimeout(120_000);
    await page.goto('/examples/web/');
    const report=await page.evaluate(async family=>{
      const {loadRenderer,metricsMeasurer}=await import('/dist/package-web/package/full.js');
      const engine=await loadRenderer(), fixture=await(await fetch(`/fixtures/pele/${family}.json`)).json();
      const canonical=value=>JSON.stringify(value,(_,v)=>v instanceof Map?[...v]:v&&typeof v==='object'&&!Array.isArray(v)?Object.fromEntries(Object.entries(v).sort(([a],[b])=>a.localeCompare(b))):v);
      const mismatches=[];
      for(const c of fixture.cases) {
        let actual,model;
        const host=c.host??{},options={...c.options,measurer:host.measure==='fixed'?{width:(text,size,style)=>text.length*size*(style&1?.71:.6)}:metricsMeasurer,...(host.allIcons?{icons:()=>'<path d="M0,0"/>'}:host.icons?{icons:n=>Object.hasOwn(host.icons,n)&&typeof host.icons[n]==='string'?host.icons[n]:undefined}:{})};
        try {
          if(host.modelProjection==='excluded') {
            let bounded=false;
            try {const projected=engine.parse(c.source,c.options);engine.dispose(projected)} catch(e){bounded=e.code==='limit'&&e.type===family}
            if(!bounded)mismatches.push(`${c.id}: projection was not bounded`);
            actual={result:engine.render(c.source,options)};
          } else {
            model=engine.parse(c.source,c.options);
            actual={model,result:engine.render(model,options)};
          }
          if(host.modelProjection!=='excluded'&&canonical(actual.result)!==canonical(engine.render(c.source,options)))mismatches.push(`${c.id}: source/model differ`);
          if(c.expected.result?.svgSha256){const hash=await crypto.subtle.digest('SHA-256',new TextEncoder().encode(actual.result.svg));delete actual.result.svg;actual.result.svgSha256=[...new Uint8Array(hash)].map(v=>v.toString(16).padStart(2,'0')).join('');}
        } catch(e) { actual={...(c.expected.model&&model?{model}:{}),error:{name:e.name,message:e.message,code:e.code,type:e.type,line:e.line,column:e.column,snippet:e.snippet}}; }
        finally { if(model)engine.dispose(model); }
        if(canonical(actual)!==canonical(c.expected))mismatches.push(c.id);
      }
      // Rust checks the typed shared-string model without a 400 MB JSON projection.
      for(const c of [...(fixture.stress??[]),...(fixture.typedCases??[])]) {
        let bounded=false;try {const model=engine.parse(c.source,c.options);engine.dispose(model)}catch(e){bounded=e.code==='limit'&&e.type===family}
        if(!bounded)mismatches.push(`stress/${c.id}: model projection was not bounded`);
        const actual=engine.render(c.source,{...c.options,measurer:metricsMeasurer});
        const hash=await crypto.subtle.digest('SHA-256',new TextEncoder().encode(actual.svg));delete actual.svg;actual.svgSha256=[...new Uint8Array(hash)].map(v=>v.toString(16).padStart(2,'0')).join('');
        if(canonical(actual)!==canonical(c.expected.result))mismatches.push(`stress/${c.id}`);
      }
      for(const c of fixture.generatedStress??[]) {
        let model;
        try { model=engine.parse(c.source,c.options);
          if(model.nodes.size!==c.expected.nodes||model.edges.length!==c.expected.edges||model.edges[0].type!==c.expected.firstEdgeType)mismatches.push(`generated/${c.id}`);
        } finally {if(model)engine.dispose(model)}
      }
      return {count:fixture.cases.length,mismatches};
    },family);
    expect(report.count).toBeGreaterThan(100);
    expect(report.mismatches).toEqual([]);
  });
}

test('new families paint resolved SVG through real browser decoders',async({page})=>{
  await page.goto('/examples/web/');
  const report=await page.evaluate(async()=>{
    const {loadRenderer}=await import('/dist/package-web/package/full.js'),engine=await loadRenderer(),proof=[];
    for(const [type,source] of [
      ['xychart','xychart-beta\ntitle Revenue\nx-axis [Jan, Feb, Mar]\ny-axis Sales 0 --> 100\nbar [20, 40, 70]\nline [25, 45, 80]'],
      ['class','classDiagram\nclass Engine {\n +render() SVG\n}\nEngine --> Document : renders'],
      ['er','erDiagram\nUSER ||--o{ DOCUMENT : owns\nUSER {\n int id PK\n string name\n}\nDOCUMENT {\n int id PK\n}'],
      ['flowchart','flowchart TD\nsubgraph one[My Group]\nA --> B\nend\nC --> A'],
      ['kanban','kanban\n  Todo\n    task[Build Rust]\n  Done\n    ship[Ship Swift]'],
      ['mindmap','mindmap\n  root((Rust))\n    Swift\n    Web'],
      ['packet','packet-beta\ntitle Network header\n0-7: "Version"\n8-15: "Flags"\n16-31: "Length"'],
      ['radar','radar-beta\ntitle Skills\naxis a["Speed"], b["Quality"], c["Cost"]\ncurve team["Team"]{4, 3, 5}'],
      ['timeline','timeline\ntitle Releases\nsection Core\n2025: Rust\n2026: Swift: Web'],
      ['journey','journey\ntitle Build and ship\nsection Core\nImplement: 4: Rust\nVerify: 5: Swift, Web'],
      ['quadrantChart','quadrantChart\ntitle Work priorities\nx-axis Low --> High\ny-axis Low --> High\nFast: [0.8, 0.7]\nCareful: [0.3, 0.4]']
    ]) {
      const rendered=engine.render(source,{nativeTheme:{}});
      const image=new Image();image.src='data:image/svg+xml;base64,'+btoa(unescape(encodeURIComponent(rendered.svg)));await image.decode();
      const canvas=document.createElement('canvas');canvas.width=Math.ceil(rendered.width);canvas.height=Math.ceil(rendered.height);
      const ctx=canvas.getContext('2d');ctx.drawImage(image,0,0);const data=ctx.getImageData(0,0,canvas.width,canvas.height).data;
      let painted=0;for(let i=0;i<data.length;i+=4)if(data[i+3]>100)painted++;
      proof.push({type:rendered.type,expectedType:type,painted,unresolved:rendered.svg.includes('var(')||rendered.svg.includes('color-mix(')});
    }
    return proof;
  });
  for(const proof of report){expect(proof.type).toBe(proof.expectedType);expect(proof.unresolved).toBe(false);expect(proof.painted).toBeGreaterThan(1000);}
});
