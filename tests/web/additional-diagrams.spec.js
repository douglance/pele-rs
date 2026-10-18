import {test,expect} from '@playwright/test';

for(const family of ['packet','radar','quadrant','timeline','journey','flowchart','kanban','mindmap','class','er','xychart','state','gantt','sequence','git-graph','architecture','sankey','treemap','c4','info','requirement','block','railroad']) {
  test(`${family} pinned corpus through WASM source and owned model APIs`, async({page})=>{
    test.setTimeout(120_000);
    await page.goto('/examples/web/');
    const report=await page.evaluate(async family=>{
      const {loadRenderer,metricsMeasurer}=await import('/dist/package-web/package/full.js');
      const engine=await loadRenderer(), fixture=await(await fetch(`/fixtures/pele/${family}.json`)).json();
      const canonical=value=>JSON.stringify(value,(_,v)=>v instanceof Map?[...v]:v&&typeof v==='object'&&!Array.isArray(v)?Object.fromEntries(Object.entries(v).sort(([a],[b])=>a.localeCompare(b))):v);
      const mismatches=[];
      const oracle=family==='gantt'?await import('/dist/evidence/pele-oracle.js'):null;
      let runtimeDateCases=0;
      for(const c of fixture.cases) {
        let actual,model;
        const host=c.host??{},options={...c.options,measurer:host.measure==='fixed'?{width:(text,size,style)=>text.length*size*(style&1?.71:.6)}:metricsMeasurer,...(host.allIcons?{icons:()=>'<path d="M0,0"/>'}:host.icons?{icons:n=>Object.hasOwn(host.icons,n)&&typeof host.icons[n]==='string'?host.icons[n]:undefined}:{})};
        let expected={...c.expected};
        // Diagnostic provenance is not part of the public parse/render envelope.
        delete expected.upstreamModelSha256;
        // Date.parse is a host resource. Compare divergent legacy inputs against
        // the pinned upstream library in this same browser, without fixture injection.
        if(oracle&&Object.entries(host.oracleDateParse??{}).some(([text,value])=>{
          const parsed=Date.parse(text);return (Number.isFinite(parsed)?parsed:null)!==value;
        })) {
          runtimeDateCases++;
          let oracleModel;
          try {
            const parsed=oracle.parse(c.source,c.options);
            oracleModel=JSON.parse(JSON.stringify(parsed,(_,v)=>v instanceof Map?[...v]:v));
            expected={model:oracleModel,result:oracle.render(parsed,options)};
          } catch(e) {expected={...(oracleModel?{model:oracleModel}:{}),error:{name:e.name,message:e.message,code:e.code,type:e.type,line:e.line,column:e.column,snippet:e.snippet}};}
        }
        try {
          if((host.modelProjection==='excluded'||expected.modelProjectionLimit)) {
            let bounded=false;
            try {const projected=engine.parse(c.source,c.options);engine.dispose(projected)} catch(e){bounded=e.code==='limit'&&e.type===family}
            if(!bounded)mismatches.push(`${c.id}: projection was not bounded`);
            actual={...(expected.modelProjectionLimit?{modelProjectionLimit:true}:{}),result:engine.render(c.source,options)};
          } else {
            model=engine.parse(c.source,c.options);
            actual={model,result:engine.render(model,options)};
          }
          if(!(host.modelProjection==='excluded'||expected.modelProjectionLimit)&&canonical(actual.result)!==canonical(engine.render(c.source,options)))mismatches.push(`${c.id}: source/model differ`);
          if(expected.result?.svgSha256){const hash=await crypto.subtle.digest('SHA-256',new TextEncoder().encode(actual.result.svg));delete actual.result.svg;actual.result.svgSha256=[...new Uint8Array(hash)].map(v=>v.toString(16).padStart(2,'0')).join('');}
        } catch(e) { actual={...(expected.model&&model?{model}:{}),error:{name:e.name,message:e.message,code:e.code,type:e.type,line:e.line,column:e.column,snippet:e.snippet}}; }
        finally { if(model)engine.dispose(model); }
        if(canonical(actual)!==canonical(expected))mismatches.push(c.id);
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
      return {count:fixture.cases.length,runtimeDateCases,mismatches};
    },family);
    if(report.runtimeDateCases)console.log(`${family}: ${report.runtimeDateCases} browser Date.parse cases checked against pinned same-browser Pele`);
    expect(report.count).toBeGreaterThan(100);
    expect(report.mismatches).toEqual([]);
  });
}

test('new families paint resolved SVG through real browser decoders',async({page})=>{
  await page.goto('/examples/web/');
  const report=await page.evaluate(async()=>{
    const {loadRenderer}=await import('/dist/package-web/package/full.js'),engine=await loadRenderer(),proof=[];
    for(const [type,source] of [
      ['requirement','requirementDiagram\nrequirement core {\n id: 1\n text: Shared renderer\n risk: high\n verifymethod: test\n}\nelement app {\n type: service\n}\napp - satisfies -> core'],
      ['block','block-beta\ncolumns 2\napp["Swift"] core["Rust"]\napp --> core'],
      ['railroad','railroad-abnf-beta\nrule = "test" / "more" ;'],
      ['treemap','treemap-beta\n"Category A"\n  "Swift": 10\n  "Web": 20\n"Category B"\n  "Rust": 30'],
      ['c4','C4Context\ntitle Shared system\nPerson(user, "User", "Author")\nSystem(core, "Rust", "Renderer")\nRel(user, core, "Uses")'],
      ['info','info\ntitle Shared engine'],
      ['gitGraph','gitGraph\ncommit id: "base"\nbranch feature\ncommit\ncheckout main\nmerge feature'],
      ['architecture','architecture-beta\nservice app(server)[App]\nservice db(database)[Data]\napp:R --> L:db'],
      ['sankey','sankey\nCapture,Store,10\nStore,Swift,6\nStore,Web,4'],
      ['state','stateDiagram-v2\n[*] --> Ready\nstate Active {\n  [*] --> Working\n  Working --> [*]\n}\nReady --> Active'],
      ['gantt','gantt\ntitle Release plan\ndateFormat YYYY-MM-DD\nsection Core\nBuild :done,a,2024-03-01,2d\nVerify :crit,b,after a,2d'],
      ['sequence','sequenceDiagram\nparticipant App\nparticipant Core\nApp->>Core: Render\nactivate Core\nCore-->>App: SVG\ndeactivate Core'],
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
  for(const proof of report){expect(proof.type).toBe(proof.expectedType);expect(proof.unresolved).toBe(false);expect(proof.painted, proof.expectedType).toBeGreaterThan(proof.expectedType==='info'?500:1000);}
});


test('Sankey keeps non-finite result dimensions across owned WASM documents',async({page})=>{
  await page.goto('/examples/web/');
  const report=await page.evaluate(async()=>{
    const {loadRenderer}=await import('/dist/package-web/package/full.js'),engine=await loadRenderer();
    const model=engine.parse('sankey\na,b,5e-324');
    try { const a=engine.render(model),b=engine.render('sankey\na,b,5e-324');return {heightNaN:Number.isNaN(a.height)&&Number.isNaN(b.height),metadataExposed:Object.keys(a).some(k=>k.startsWith('$'))}; }
    finally {engine.dispose(model)}
  });
  expect(report).toEqual({heightNaN:true,metadataExposed:false});
});
