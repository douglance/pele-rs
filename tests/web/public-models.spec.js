import {test, expect} from '@playwright/test';

test('mutable Pie models preserve edits, collection identity and retained render provenance', async ({page}) => {
  await page.goto('/examples/web/');
  const report = await page.evaluate(async () => {
    const {loadRenderer, metricsMeasurer} = await import('/dist/package-web/package/full.js');
    const oracle = await import('/dist/evidence/pele-oracle.js');
    const engine = await loadRenderer(), source = 'pie title Before\n"A":2\n"B":3';
    const model = engine.parse(source), expected = oracle.parse(source);
    const options = {measurer: metricsMeasurer}, failures = [];
    const compare = name => {
      if (JSON.stringify(engine.render(model, options)) !== JSON.stringify(oracle.render(expected, options))) failures.push(name);
    };
    const fields = Object.getOwnPropertyNames(model);
    const expectedFields = Object.getOwnPropertyNames(expected);
    const originalSections = model.sections;
    compare('initial');
    for (const value of [model, expected]) {
      value.sections.set('C', 7); value.sections.delete('A'); value.sections.set('A', 1);
      value.title = 'After'; value.accTitle = 'Mutable chart'; value.showData = true;
    }
    compare('Map edits and labels');
    const identityPreserved = model.sections === originalSections;
    for (const value of [model, expected]) {
      value.sections = new Map([['Replacement', 4], ['Second', 9]]);
      value.type = 'consumer-extension';
    }
    compare('Map replacement and retained type');
    const replacedIdentity = model.sections !== originalSections;
    const extra = {text: '\ud800A\udc00', nan: NaN, negativeZero: -0, infinity: Infinity};
    extra.self = extra; model.extra = extra;
    let tail = extra;
    for (let i = 0; i < 5000; i++) { tail.next = {}; tail = tail.next; }
    tail.next = extra;
    compare('deep cycles and lossless extra values');
    const extrasPreserved = model.extra === extra && extra.self === extra &&
      extra.text.charCodeAt(0) === 0xd800 && extra.text.charCodeAt(2) === 0xdc00 &&
      Number.isNaN(extra.nan) && Object.is(extra.negativeZero, -0) && extra.infinity === Infinity;
    const sentinel = new Error('callback identity'); let sameError = false;
    try { engine.render(model, {measurer: {width() { throw sentinel; }}}); }
    catch (error) { sameError = error === sentinel; }
    compare('recovery after host failure');
    const retainedType = engine.render(model, {...options, limit: 0}).type;
    engine.dispose(model);
    let disposed = false;
    try { engine.render(model, options); } catch (error) { disposed = error instanceof TypeError; }
    return {failures, fields, expectedFields, identityPreserved, replacedIdentity, extrasPreserved, sameError, retainedType, disposed};
  });
  expect(report.failures).toEqual([]);
  expect(report.fields).toEqual(report.expectedFields);
  expect(report).toMatchObject({identityPreserved: true, replacedIdentity: true, extrasPreserved: true, sameError: true, retainedType: 'pie', disposed: true});
});


test('mutable Info metadata and Sankey aliases keep pinned render behavior', async ({page}) => {
  await page.goto('/examples/web/');
  const report = await page.evaluate(async () => {
    const {loadRenderer, metricsMeasurer} = await import('/dist/package-web/package/full.js');
    const oracle = await import('/dist/evidence/pele-oracle.js'), engine = await loadRenderer();
    const failures = [], options = {measurer: metricsMeasurer};
    for (const source of ['info', 'sankey\nA,B,5\nB,C,3']) {
      const model = engine.parse(source), expected = oracle.parse(source);
      if (JSON.stringify(Object.keys(model)) !== JSON.stringify(Object.keys(expected))) failures.push('own fields '+model.type);
      const compare = stage => {
        if (JSON.stringify(engine.render(model, options)) !== JSON.stringify(oracle.render(expected, options))) failures.push(stage);
      };
      compare('initial '+model.type);
      for (const value of [model, expected]) {value.title = 'Changed **Rust**';value.accTitle='Accessible title';value.accDescr='Accessible description';}
      compare('metadata '+model.type);
      if (model.type === 'sankey') {
        if (model.links[0].source !== model.nodes[0] || model.lookup.get('A') !== model.nodes[0]) failures.push('initial aliases');
        for (const value of [model, expected]) {
          value.links[0].value=7;value.nodes[0].id='Renamed';
          value.lookup=new Map([['detached', value.nodes[0]]]);
        }
        compare('weights nodes and lookup edits');
        const nodes = model.nodes, endpoint = model.links[0].source;
        for (const value of [model, expected]) value.nodes.reverse();
        compare('array reordering');
        if (model.nodes!==nodes || model.links[0].source!==endpoint || model.lookup.get('detached')!==endpoint) failures.push('retained aliases');
      }
      const sentinel = new Error('host identity '+model.type);let sameError=false;
      try {engine.render(model,{measurer:{width(){throw sentinel;}}});}catch(error){sameError=error===sentinel;}
      if (!sameError) failures.push('callback identity');
      compare('recovery '+model.type);
      engine.dispose(model);
    }
    return failures;
  });
  expect(report).toEqual([]);
});

test('native model methods preserve prototypes, aliases and partial errors through WASM', async ({page}) => {
  await page.goto('/examples/web/');
  const failures = await page.evaluate(async () => {
    const {loadRenderer,metricsMeasurer}=await import('/dist/package-web/package/full.js');
    const oracle=await import('/dist/evidence/pele-oracle.js'), engine=await loadRenderer(), failures=[];
    const sources={journey:'journey\nsection Start\nTask:3:Alice',quadrant:'quadrantChart\nA:[0.2,0.3]',xychart:'xychart-beta\nx-axis [A,B]\nline [1,2]'};
    const canonical=v=>JSON.stringify(v,(_,x)=>x instanceof Map?[...x]:x);
    for(const [family,source] of Object.entries(sources)) {
      const model=engine.parse(source),expected=oracle.parse(source), second=engine.parse(source);
      const proto=Object.getPrototypeOf(model), expectedProto=Object.getPrototypeOf(expected);
      for(const name of Object.getOwnPropertyNames(expectedProto).filter(x=>x!=='constructor')) {
        const a=Object.getOwnPropertyDescriptor(proto,name),b=Object.getOwnPropertyDescriptor(expectedProto,name);
        if(!a||a.enumerable!==b.enumerable||a.configurable!==b.configurable||a.writable!==b.writable||a.value.length!==b.value.length)failures.push(family+' descriptor '+name);
        if(model[name]!==second[name])failures.push(family+' shared function '+name);
        if(canonical(Reflect.ownKeys(a.value))!==canonical(Reflect.ownKeys(b.value)))failures.push(family+' callable shape '+name+' '+canonical(Reflect.ownKeys(a.value))+' / '+canonical(Reflect.ownKeys(b.value)));
      }
      const compare=stage=>{
        if(canonical(model)!==canonical(expected))failures.push(family+' model '+stage);
        if(canonical(engine.render(model,{measurer:metricsMeasurer}))!==canonical(oracle.render(expected,{measurer:metricsMeasurer})))failures.push(family+' SVG '+stage);
      };
      const both=(name,...args)=>{const a=model[name](...args),b=expected[name](...args);if(canonical(a)!==canonical(b))failures.push(family+' return '+name);};
      both('setDiagramTitle','Native methods');both('setAccTitle','  Accessible');both('setAccDescription','First\n   Second');
      if(family==='journey') {
        const tasks=model.tasks;both('addSection','Next');both('addTask','Build',':4: Bob, Alice');both('getActors');
        if(tasks!==model.tasks)failures.push('journey collection replaced');
        const borrowed={sections:model.sections,tasks:model.tasks,currentSection:'Borrowed'},reference={sections:expected.sections,tasks:expected.tasks,currentSection:'Borrowed'};
        proto.addTask.call(borrowed,'Shared',':5: Bob');expectedProto.addTask.call(reference,'Shared',':5: Bob');
        if(borrowed.tasks!==tasks)failures.push('borrowed alias');
        compare('borrowed');
        for(const value of [model,expected])value.sections=null;
        let a,b;try{model.addSection('Published')}catch(e){a=e.name}try{expected.addSection('Published')}catch(e){b=e.name}
        if(a!=='TypeError'||a!==b||model.currentSection!==expected.currentSection)failures.push('partial error publication');
        for(const value of [model,expected])value.sections=[];
      } else if(family==='quadrant') {
        both('setQuadrant1Text',{text:'  Top  ',type:'text'});both('addClass','hot',['radius:7','color:#f00']);both('addPoint',{text:'  Added ',type:'text'},'hot','0.6','0.8',[]);
        let a,b;try{model.addClass('bad',['radius:\ud800'])}catch(e){a=e}try{expected.addClass('bad',['radius:\ud800'])}catch(e){b=e}
        if(a?.name!==b?.name||a?.message!==b?.message||a?.code!==b?.code)failures.push('UTF16 PeleError');
      } else {
        const oldAxis=model.xAxis;
        both('setXAxisTitle',{text:'  Category ',type:'text'});both('setXAxisBand',[{text:'A',type:'text'},{text:'B',type:'text'}]);
        both('setYAxisRangeData',0,10);both('setBarData',{text:'Bars',type:'text'},[{value:3,label:''},{value:5,label:''}]);
        if(oldAxis===model.xAxis)failures.push('axis replacement');
        const detached=model.plotData([2,4]);expected.plotData([2,4]);detached[0][1]=9;
      }
      compare('builders and recovery');
      engine.dispose(second);engine.dispose(model);
    }
    return failures;
  });
  expect(failures).toEqual([]);
});

test('host callback throws match pinned guard behavior and recover',async({page})=>{
  await page.goto('/examples/web/');
  const failures=await page.evaluate(async()=>{
    const {loadRenderer,metricsMeasurer}=await import('/dist/package-web/package/full.js'),oracle=await import('/dist/evidence/pele-oracle.js'),engine=await loadRenderer(),model=engine.parse('pie title Callback\n"A":1'),failures=[];
    for(const sentinel of [undefined,null,false,0,NaN,new RangeError('host range'),new Error('host error')]) {
      let caught=false,actual,expected;
      try{engine.render(model,{measurer:{width(){throw sentinel}}})}catch(e){caught=true;actual=e;}
      try{oracle.render('pie title Callback\n"A":1',{measurer:{width(){throw sentinel}}})}catch(e){expected=e;}
      if(sentinel instanceof RangeError) {
        if(actual?.name!==expected?.name||actual?.message!==expected?.message||actual?.code!==expected?.code)failures.push('RangeError guard');
      } else if(!Object.is(actual,expected))failures.push(String(sentinel));
      if(!caught)failures.push('not thrown '+String(sentinel));
      if(engine.render(model,{measurer:metricsMeasurer}).type!=='pie')failures.push('recovery');
    }
    engine.dispose(model);return failures;
  });
  expect(failures).toEqual([]);
});

test('outline timeline graph and architecture methods keep pinned shared identities and mutable renders',async({page})=>{
  test.setTimeout(90_000);
  await page.goto('/examples/web/');
  const failures=await page.evaluate(async()=>{
    const {loadRenderer,metricsMeasurer}=await import('/dist/package-web/package/full.js');
    const oracle=await import('/dist/evidence/pele-oracle.js'),engine=await loadRenderer(),failures=[];
    const sources={kanban:'kanban\n  Todo\n    task[Build]',mindmap:'mindmap\n  root((Rust))\n    Swift',timeline:'timeline\nsection Start\n2025: Rust',sankey:'sankey\nA,B,5',flowchart:'flowchart LR\nsubgraph group[Group]\nA[Start] --> B[End]\nend',swimlane:'swimlane-beta LR\nsubgraph group[Group]\nA[Start] --> B[End]\nend',er:'erDiagram\nUSER ||--o{ DOCUMENT : owns',architecture:'architecture-beta\nservice app(server)[App]'};
    const models={},references={},options={measurer:metricsMeasurer};
    const canonical=value=>JSON.stringify(value,(_,v)=>v instanceof Map?[...v]:v instanceof Set?[...v]:v);
    for(const [family,source]of Object.entries(sources)) {
      const model=engine.parse(source),expected=oracle.parse(source);models[family]=model;references[family]=expected;
      const actualProto=Object.getPrototypeOf(model),expectedProto=Object.getPrototypeOf(expected);
      if(canonical(Object.keys(model))!==canonical(Object.keys(expected)))failures.push(family+' own keys');
      for(const name of Object.getOwnPropertyNames(expectedProto).filter(x=>x!=='constructor')) {
        const a=Object.getOwnPropertyDescriptor(actualProto,name),b=Object.getOwnPropertyDescriptor(expectedProto,name);
        if(!a||a.enumerable!==b.enumerable||a.configurable!==b.configurable||a.writable!==b.writable||a.value.length!==b.value.length)failures.push(family+' method descriptor '+name);
        if(a&&canonical(Reflect.ownKeys(a.value))!==canonical(Reflect.ownKeys(b.value)))failures.push(family+' function shape '+name);
      }
      const compare=stage=>{
        if(canonical(model)!==canonical(expected))failures.push(family+' raw model '+stage);
        if(canonical(engine.render(model,options))!==canonical(oracle.render(expected,options)))failures.push(family+' complete SVG '+stage);
      };
      compare('initial');
      const both=(method,args)=>{
        const a=model[method](...args(model)),b=expected[method](...args(expected));
        if(canonical(a)!==canonical(b))failures.push(family+' return '+method);
      };
      if(family==='kanban') {both('addNode',()=>[4,'check','Verify',0]);both('decorateNode',()=>[{class:'checked',icon:'check'}]);if(model.last!==model.sections[0].items.at(-1))failures.push('kanban last alias');}
      if(family==='mindmap') {both('addNode',()=>[4,'web','Web',0]);both('decorateNode',()=>[{class:'checked'}]);if(model.open.at(-1)!==model.nodes.at(-1))failures.push('mindmap open alias');}
      if(family==='timeline') {
        if(model.getCommonDb()!==model||model.getCommonDb.call(model.periods)!==model.periods)failures.push('timeline returned receiver');
        both('addSection',()=>['Next']);both('addTask',()=>['2026',900,'Swift']);both('addEvent',()=>['Web']);both('setDiagramTitle',()=>['Mutable timeline']);
      }
      if(family==='sankey') {
        both('findOrCreateNode',()=>['C']);both('addLink',v=>[v.lookup.get('B'),v.lookup.get('C'),3]);
        if(model.links.at(-1).target!==model.lookup.get('C'))failures.push('sankey lookup alias');
      }
      if(family==='flowchart'||family==='swimlane') {
        both('addVertex',()=>['C',{text:'Native',type:'text'},'square',[],[]]);
        both('addLink',()=>[['B'],['C'],{type:'arrow_point',stroke:'normal',length:1}]);
        both('setDirection',()=>['TB']);both('addClass',()=>['hot',['fill:#ff0000']]);both('setClass',()=>['C','hot']);both('setTooltip',()=>['C','Native tool']);
        if(model.edgeAt(0)!==model.edges[0]||model.subgraph('group')!==model.subgraphs[0])failures.push(family+' returned alias');
      }
      if(family==='er') {
        both('addEntity',()=>['TEAM','Team']);both('addAttributes',()=>['TEAM',[{type:'int',name:'id',keys:['PK'],comment:'key'}]]);
        both('addRelationship',()=>['USER','joins','TEAM',{cardA:'ONLY_ONE',relType:'IDENTIFYING',cardB:'ZERO_OR_MORE'}]);
        both('setDirection',()=>['LR']);if(model.addEntity('TEAM')!==model.entities.get('TEAM'))failures.push('ER entity alias');expected.addEntity('TEAM');
      }
      if(family==='architecture') {
        both('addService',()=>[{id:'web',icon:'server',title:'Web'}]);
        both('addEdge',()=>[{lhsId:'app',lhsDir:'R',lhsInto:false,lhsGroup:false,rhsId:'web',rhsDir:'L',rhsInto:true,rhsGroup:false}]);
        const hint={direction:'row',members:['app','web']},referenceHint={direction:'row',members:['app','web']};
        model.addLayoutHint(hint);expected.addLayoutHint(referenceHint);
        if(model.layoutHints.at(-1)!==hint)failures.push('architecture hint alias');hint.direction='column';referenceHint.direction='column';
      }
      compare('native methods');
      model.title='Edited metadata';expected.title='Edited metadata';compare('field edit');
      if(family==='timeline') {
        model.sections=null;expected.sections=null;let a,b;try{model.addSection('Before failure')}catch(e){a=e.name}try{expected.addSection('Before failure')}catch(e){b=e.name}
        if(a!=='TypeError'||a!==b||model.currentSection!==expected.currentSection)failures.push('timeline partial failure');
        model.sections=[];expected.sections=[];compare('recovery');
      }
    }
    const k=models.kanban,m=models.mindmap;
    if(k.getType!==m.getType||references.kanban.getType!==references.mindmap.getType)failures.push('cross-schema shared helper identity');
    for(const pair of [['(',')'],['[',']'],['((', undefined],[')',undefined],['))',undefined],['{{',undefined],['none',undefined]])if(k.getType(...pair)!==references.kanban.getType(...pair))failures.push('shared helper behavior');
    const newest=engine.parse(sources.kanban);engine.dispose(newest);
    if(k.getType.call({},'[',']')!==2)failures.push('disposed latest document must not break earlier borrowed helper');
    if(Object.getPrototypeOf(models.flowchart)!==Object.getPrototypeOf(models.swimlane)||models.flowchart.addVertex!==models.swimlane.addVertex)failures.push('shared FlowDb prototype');
    if(models.swimlane.type!=='flowchart'||engine.render(models.swimlane,options).type!=='swimlane')failures.push('retained swimlane renderer');
    engine.dispose(k);
    if(m.getType.call(null,'[',']')!==2)failures.push('shared helper survives other family disposal');
    for(const model of Object.values(models))engine.dispose(model);
    return failures;
  });
  expect(failures).toEqual([]);
});
