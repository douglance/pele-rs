import {test,expect} from '@playwright/test';

for(const family of ['packet','radar','quadrant','timeline','journey','flowchart','kanban','mindmap','class','er','xychart','state','gantt','sequence','git-graph','architecture','sankey','treemap','c4','info','requirement','block','railroad','cynefin','treeview','agentflow','wardley','eventmodeling','usecase','venn','ishikawa','swimlane']) {
  test(`${family} pinned corpus through WASM source and owned model APIs`, async({page})=>{
    test.setTimeout(120_000);
    await page.goto('/examples/web/');
    const report=await page.evaluate(async family=>{
      const {loadRenderer,metricsMeasurer}=await import('/dist/package-web/package/full.js');
      const engine=await loadRenderer(), fixture=await(await fetch(`/fixtures/pele/${family}.json`)).json();
      const publicType=family==='treeview'?'treeView':family==='git-graph'?'gitGraph':family;
      const canonical=value=>JSON.stringify(value,(_,v)=>v instanceof Map?[...v]:v&&typeof v==='object'&&!Array.isArray(v)?Object.fromEntries(Object.entries(v).sort(([a],[b])=>a.localeCompare(b))):v);
      const mismatches=[];
      const oracle=family==='gantt'?await import('/dist/evidence/pele-oracle.js'):null;
      let runtimeDateCases=0, flatProjectionCases=0, transportBoundaryCases=0, binaryStyleCases=0;
      // The old corpus omits this declared private lookup. Raw public ownership
      // and alias identity have their own mutable-model E2E proof.
      const comparedModel=model=>{
        const omitted=model?.type==='sankey'?['lookup']:model?.type==='flowchart'?['subgraphLookup','members','edgeById','pairCount','subCount','options']:model?.type==='er'?['subgraphLookup','claimed']:model?.type==='agentflow'?['subgraphLookup','globalNodes','members','edgeById','pairCount','subCount','bareMappings','firstMapping','reported','options']:model?.type==='state'?['dividerCount']:model?.type==='ishikawa'?['stack','baseLevel']:[];
        const view=omitted.length?Object.fromEntries(Object.entries(model).filter(([key])=>!omitted.includes(key))):model;
        if(model?.type==='state')view.direction=model.direction;
        return view;
      };
      for(const c of fixture.cases) {
        let actual,model,semanticModel,projectionBounded=false;
        const host=c.host??{},options={...c.options,measurer:host.measure==='fixed'?{width:(text,size,style)=>text.length*size*(style&1?.71:.6)}:metricsMeasurer,...(host.allIcons?{icons:()=>'<path d="M0,0"/>'}:host.icons?{icons:n=>Object.hasOwn(host.icons,n)&&typeof host.icons[n]==='string'?host.icons[n]:undefined}:{})};
        const classTransportBoundaries={
          'stress/namespace name of 12,000 parts':'046702029bd9c44c84e60976877b377f08f77f0f4a874cde27c3985b9b232bc5',
          'stress/4,000 nested namespaces':'56424df5324ecd9871e81aa3b3e06e3f8504a0fd521cc162e4330a725e13ffa0'
        };
        if(family==='class'&&Object.hasOwn(classTransportBoundaries,c.id)){
          const bytes=await crypto.subtle.digest('SHA-256',new TextEncoder().encode(c.source));
          const hash=[...new Uint8Array(bytes)].map(v=>v.toString(16).padStart(2,'0')).join('');
          if(hash!==classTransportBoundaries[c.id])mismatches.push(c.id+': boundary source hash');
          const errorValue=e=>({name:e.name,message:e.message,code:e.code,type:e.type,line:e.line,column:e.column,snippet:e.snippet});
          let nativeError,sourceError,unexpectedModel;
          try{unexpectedModel=engine.parse(c.source,c.options)}catch(e){nativeError=errorValue(e)}finally{if(unexpectedModel)engine.dispose(unexpectedModel)}
          try{engine.render(c.source,options)}catch(e){sourceError=errorValue(e)}
          if(canonical(nativeError)!==canonical({name:'PeleError',message:'Public model graph exceeds work budget',code:'limit',type:null,line:0,column:0,snippet:''}))mismatches.push(c.id+': native transport boundary');
          if(canonical(sourceError)!==canonical(c.expected.error))mismatches.push(c.id+': source error');
          transportBoundaryCases++;
          continue;
        }
        let expected={...c.expected};
        // Diagnostic provenance is not part of the public parse/render envelope.
        delete expected.upstreamModelSha256;
        delete expected.projectionBudget;
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
            let bounded=false, projected;
            try {projected=engine.parse(c.source,c.options)} catch(e){bounded=e.code==='limit'&&e.type===publicType}
            if(family==='class'&&['stress/1,600 classDefs applied to 1,600 classes','stress/a classDef of 5,000 declarations on 800 classes, each with a style of its own'].includes(c.id)){
              if(!projected)mismatches.push(c.id+': valid model must cross binary transport');
              else binaryStyleCases++;
            }
            projectionBounded=bounded||!!projected;
            if(!projectionBounded)mismatches.push(`${c.id}: projection failed outside declared bounds`);
            let rendered;
            try {
              if(projected) {
                flatProjectionCases++;
                rendered=engine.render(projected,options);
                if(canonical(rendered)!==canonical(engine.render(c.source,options)))mismatches.push(`${c.id}: flat/source differ`);
              } else rendered=engine.render(c.source,options);
            } finally {if(projected)engine.dispose(projected)}
            actual={...(expected.modelProjectionLimit?{modelProjectionLimit:true}:{}),result:rendered};
          } else {
            model=engine.parse(c.source,c.options);
            semanticModel=expected.semanticModel?engine.query(model,"semanticModel"):undefined;
            actual={model:comparedModel(model),...(semanticModel!==undefined?{semanticModel}:{}),result:engine.render(model,options)};
          }
          if(!(host.modelProjection==='excluded'||expected.modelProjectionLimit)&&canonical(actual.result)!==canonical(engine.render(c.source,options)))mismatches.push(`${c.id}: source/model differ`);
          if(expected.result?.svgSha256){const hash=await crypto.subtle.digest('SHA-256',new TextEncoder().encode(actual.result.svg));delete actual.result.svg;actual.result.svgSha256=[...new Uint8Array(hash)].map(v=>v.toString(16).padStart(2,'0')).join('');}
        } catch(e) { actual={...(projectionBounded&&expected.modelProjectionLimit?{modelProjectionLimit:true}:{}),...(expected.model&&model?{model:comparedModel(model)}:{}),...(semanticModel!==undefined?{semanticModel}:{}),error:{name:e.name,message:e.message,code:e.code,type:e.type,line:e.line,column:e.column,snippet:e.snippet}}; }
        finally { if(model)engine.dispose(model); }
        if(canonical(actual)!==canonical(expected))mismatches.push(c.id);
      }
      // Rust checks the typed shared-string model without a 400 MB JSON projection.
      for(const c of [...(fixture.stress??[]),...(fixture.typedCases??[])]) {
        let bounded=false, projected;
        try {projected=engine.parse(c.source,c.options)}catch(e){bounded=e.code==='limit'&&e.type===publicType}
        if(!bounded&&!projected)mismatches.push(`stress/${c.id}: model did not project or reject with declared limit`);
        const actual=engine.render(c.source,{...c.options,measurer:metricsMeasurer});
        if(projected) {
          flatProjectionCases++;
          try {if(canonical(engine.render(projected,{...c.options,measurer:metricsMeasurer}))!==canonical(actual))mismatches.push(`stress/${c.id}: flat/source differ`)}
          finally {engine.dispose(projected)}
        }
        const hash=await crypto.subtle.digest('SHA-256',new TextEncoder().encode(actual.svg));delete actual.svg;actual.svgSha256=[...new Uint8Array(hash)].map(v=>v.toString(16).padStart(2,'0')).join('');
        if(canonical(actual)!==canonical(c.expected.result))mismatches.push(`stress/${c.id}`);
      }
      for(const c of fixture.generatedStress??[]) {
        let model;
        try { model=engine.parse(c.source,c.options);
          if(model.nodes.size!==c.expected.nodes||model.edges.length!==c.expected.edges||model.edges[0].type!==c.expected.firstEdgeType)mismatches.push(`generated/${c.id}`);
        } finally {if(model)engine.dispose(model)}
      }
      return {count:fixture.cases.length,runtimeDateCases,flatProjectionCases,transportBoundaryCases,binaryStyleCases,mismatches};
    },family);
    if(family==='class'){expect(report.binaryStyleCases).toBe(2);expect(report.transportBoundaryCases).toBe(2);}
    if(report.transportBoundaryCases)console.log(`${family}: ${report.transportBoundaryCases} explicit mutable graph transport differences verified separately from source parity`);
    if(report.flatProjectionCases)console.log(`${family}: ${report.flatProjectionCases} previous projection exceptions now exercised through flat public models`);
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
      ["venn", "---\ntitle: Shared sets\n---\nvenn-beta\nset A[\"Swift\"]:20\nset B[\"Web\"]:14\nunion A,B[\"Rust\"]:4"],
      ["ishikawa", "---\ntitle: Delivery quality\n---\nishikawa-beta\n    Release ready\n        Implementation\n            Rust engine\n            Host adapters\n        Verification\n            Swift and web\n            Integration tests"],
      ["swimlane", "---\ntitle: Shared workflow\n---\nswimlane-beta LR\nsubgraph Swift\nA[Native app]\nend\nsubgraph Rust\nB[Core engine]\nend\nsubgraph Web\nC[Browser]\nend\nA --> B --> C"],

      ["wardley","wardley-beta\ntitle Dependency map\nevolution Genesis -> Custom -> Product -> Commodity\ncomponent App [0.8, 0.3]\ncomponent Core [0.4, 0.7]\nApp -> Core"],
      ["eventmodeling","---\ntitle: Command lifecycle\n---\neventmodeling\n    tf 01 ui UI\n    tf 02 cmd RunAction\n    tf 03 evt ActionExecuted"],
      ["usecase","---\ntitle: User journeys\n---\nusecase-beta\ndirection LR\nactor Customer(\"Customer\")\nsystemBoundary \"Order system\"\n  Checkout(\"Place order\")\nend\nCustomer --> Checkout"],
      ['cynefin','cynefin-beta\n  complex\n    "Emergent practice"\n  clear\n    "Best practice"'],
      ['treeView','treeView-beta\nroot/\n    src/\n        index.js\n        App.tsx\n    tests/\n        RendererTests.swift\n    Package.swift\n    README.md'],
      ['agentflow','agentflow-beta LR\nbrief["Brief"]@{ shape: input }\ndraft["Draft"]@{ shape: task }\ncheck["Check"]@{ shape: tool }\nbrief --> draft --> check'],
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
