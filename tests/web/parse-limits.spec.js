import {test, expect} from '@playwright/test';

// Public packed-WASM integration: pinned fixtures retain exact number tags,
// whole SVGs, error fields, ordered graph descriptors and alias identities.
async function fixtureAudit(page, name) {
  await page.goto('/examples/web/');
  return page.evaluate(async name => {
    const {loadRenderer} = await import('/dist/package-web/package/full.js');
    const oracle = await import('/dist/evidence/pele-oracle.js');
    const engine = await loadRenderer(), fixture = await (await fetch(`/fixtures/pele/${name}.json`)).json();
    const failures = [], buffer = new ArrayBuffer(8), view = new DataView(buffer);
    const bits = value => {view.setFloat64(0,value);return view.getBigUint64(0).toString(16).padStart(16,'0');};
    const number = hex => {view.setBigUint64(0,BigInt('0x'+hex));return view.getFloat64(0);};
    const tag = value => value.kind === 'undefined' ? undefined : value.kind === 'null' ? null : number(value.bits);
    const units = text => Array.from({length:text.length},(_,i)=>text.charCodeAt(i));
    const equal = (a,b) => JSON.stringify(a) === JSON.stringify(b);
    const check = (value,id,stage) => {if (!value && failures.length < 30) failures.push(`${id}: ${stage}`);};
    const error = e => ({name:e.name,code:e.code,message:e.message,type:e.type,line:e.line,column:e.column,snippet:e.snippet});
    const graph = root => {
      const prototype=Object.getPrototypeOf(root), ids=new Map(),nodes=[];
      const atom=value=>{
        if(value===undefined)return['undefined'];if(value===null)return['null'];
        if(typeof value==='boolean')return['bool',value];if(typeof value==='string')return['string',units(value)];
        if(typeof value==='number')return['number',bits(value)];
        if(typeof value==='function')throw new Error('unexpected own fixture function');
        if(!ids.has(value)){ids.set(value,ids.size);nodes.push(null);}return['ref',ids.get(value)];
      };
      const r=atom(root);
      for(const[value,id]of ids){
        const kind=Array.isArray(value)?'array':value instanceof Map?'map':value instanceof Set?'set':prototype!==Object.prototype&&Object.getPrototypeOf(value)===prototype?'instance':'object';
        const node={kind};if(kind==='instance')node.prototype=1;if(kind==='array')node.length=value.length;
        if(kind==='map')node.entries=[...value].map(([k,v])=>[atom(k),atom(v)]);
        if(kind==='set')node.values=[...value].map(atom);
        node.properties=Object.getOwnPropertyNames(value).filter(k=>!(kind==='array'&&k==='length')).map(k=>{const d=Object.getOwnPropertyDescriptor(value,k);return[units(k),atom(d.value),(d.enumerable?1:0)|(d.writable?2:0)|(d.configurable?4:0)];});nodes[id]=node;
      }return{root:r,nodes};
    };
    // JS object/JIT materialization can canonicalize a NaN policy even when
    // caller/native retain its payload. Only graph Number NaN atoms compare by
    // class; all finite values, infinities, signed zeros and structure stay exact.
    const graphNumbers=(_,value)=>Array.isArray(value)&&value[0]==='number'&&Number.isNaN(number(value[1]))?['number','NaN']:value;
    const equalGraph=(actual,expected)=>name==='edge-limit'?JSON.stringify(actual,graphNumbers)===JSON.stringify(expected,graphNumbers):equal(actual,expected);
    let parsed=0,drawn=0,rejected=0;
    for(const c of fixture.cases){
      const options={...(c.options??fixture.options)};
      if(name==='source-limit') options.limit=tag(c.limit);
      else {
        if(Object.hasOwn(c.limits,'source'))options.limit=tag(c.limits.source);
        if(Object.hasOwn(c.limits,'maxEdges'))options.maxEdges=tag(c.limits.maxEdges);
      }
      let model,parseError;
      try{model=engine.parse(c.source,options);}catch(e){parseError=error(e);}
      if(c.expected.error){
        rejected++;check(equal(parseError,c.expected.error),c.id,'pinned parse error');
        const calls=[];let renderError;
        try{engine.render(c.source,{...options,measurer:{width(...args){calls.push(args);return 1;}}});}catch(e){renderError=error(e);}
        check(equal(renderError,c.expected.error),c.id,'source operation error order');check(calls.length===0,c.id,'parse rejects before measurement');
        continue;
      }
      if(parseError){check(false,c.id,'unexpected parse error '+parseError.message);continue;}
      parsed++;
      // A fixture NaN must first become this browser's caller Number. Compare
      // its retained bits to pinned Pele receiving that identical Number.
      const callerNaN=name==='edge-limit' && typeof options.maxEdges==='number' && Number.isNaN(options.maxEdges);
      const reference=callerNaN?oracle.parse(c.source,options):null;
      const expectedGraph=reference?graph(reference):c.expected.graph;
      if(expectedGraph)check(equalGraph(graph(model),expectedGraph),c.id,'complete parsed graph');
      if(c.expected.maxEdges){const wanted=tag(c.expected.maxEdges),actual=model.options.maxEdges;check(wanted===null?actual===null:wanted===undefined?actual===undefined:Number.isNaN(wanted)?Number.isNaN(actual):bits(actual)===bits(reference?reference.options.maxEdges:wanted),c.id,'exact builder policy '+JSON.stringify({caller:bits(options.maxEdges),actual:bits(actual),pinned:reference?bits(reference.options.maxEdges):c.expected.maxEdges.bits}));}
      if(c.parseOnly){check(model.edges.length===c.expected.edgeCount,c.id,'5000 boundary parsed without drawing');engine.dispose(model);continue;}
      for(const lane of ['source','model']){
        const calls=[],host={now:fixture.host.now,measurer:{width(text,size,style){calls.push([units(text),bits(size),style]);return text.length*size*(style&1?.71:.6);}}};
        let rendered;
        try{rendered=engine.render(lane==='source'?c.source:model,lane==='source'?{...options,...host}:host);}catch(e){check(false,c.id,lane+' unexpected render '+e.message);continue;}
        check(equal(rendered,c.expected.result),c.id,lane+' exact whole result');
        check(bits(rendered.width)===c.expected.widthBits&&bits(rendered.height)===c.expected.heightBits,c.id,lane+' dimensions');
        if(c.expected.calls)check(equal(calls,c.expected.calls),c.id,lane+' callback order');drawn++;
      }
      if(expectedGraph)check(equalGraph(graph(model),expectedGraph),c.id,'complete post-render graph');
      engine.dispose(model);
    }
    engine.close();return{failures,cases:fixture.cases.length,parsed,drawn,rejected};
  },name);
}

test('exact source thresholds preserve pinned UTF16 length, defaults and preprocessing precedence',async({page})=>{
  test.setTimeout(180000);
  const report=await fixtureAudit(page,'source-limit');
  expect(report.failures).toEqual([]);expect(report.cases).toBe(207);expect(report.parsed+report.rejected).toBe(207);
});

test('exact edge thresholds preserve all three builders, composed policies and 5000 boundary',async({page})=>{
  test.setTimeout(180000);
  const report=await fixtureAudit(page,'edge-limit');
  expect(report.failures).toEqual([]);expect(report.cases).toBe(313);expect(report.parsed+report.rejected).toBe(313);
});

test('fractional builder errors publish partial edges and retain aliases through policy edits and recovery',async({page})=>{
  await page.goto('/examples/web/');
  const failures=await page.evaluate(async()=>{
    const {loadRenderer}=await import('/dist/package-web/package/full.js');
    const fixture=await(await fetch('/fixtures/pele/edge-limit.json')).json(),engine=await loadRenderer(),failures=[];
    const view=new DataView(new ArrayBuffer(8)),number=hex=>{view.setBigUint64(0,BigInt('0x'+hex));return view.getFloat64(0);};
    const tag=v=>v.kind==='undefined'?undefined:v.kind==='null'?null:number(v.bits);
    const equal=(a,b)=>JSON.stringify(a)===JSON.stringify(b),check=(v,s)=>{if(!v)failures.push(s);};
    const error=e=>({name:e.name,code:e.code,message:e.message,type:e.type,line:e.line,column:e.column,snippet:e.snippet});
    for(const c of fixture.methods){
      const model=engine.parse(c.source,{...fixture.options,limit:tag(c.limits.source),maxEdges:tag(c.limits.maxEdges)}),edges=model.edges;
      for(const[index,step]of c.steps.entries()){
        let returned,caught;
        try{if(step.op.method)returned=model[step.op.method](...step.op.args);else model.options.maxEdges=tag(step.op.setMaxEdges);}catch(e){caught=error(e);}
        check(equal(caught,step.error),c.id+'/'+index+' exact operation error');
        if(step.returned)check(returned===undefined,c.id+'/'+index+' undefined return');
        check(model.edges===edges&&step.edgesIdentityRetained,c.id+'/'+index+' retained edge array');
        check(equal(model.edges.map(e=>({id:e.id,start:e.start,end:e.end,edgeByIdSame:model.edgeById.get(e.id)===e})),step.edges),c.id+'/'+index+' ordered edge/cache aliases');
      }
      const options={now:fixture.host.now,measurer:{width:(text,size,style)=>text.length*size*(style&1?.71:.6)}};
      check(equal(engine.render(model,options),c.expected.result),c.id+' recovery whole SVG');engine.dispose(model);
    }
    engine.close();return failures;
  });
  expect(failures).toEqual([]);
});

test('retained rendering ignores parse-only values and escaped native parse errors stay bounded',async({page})=>{
  test.setTimeout(90000);
  await page.goto('/examples/web/');
  const failures=await page.evaluate(async()=>{
    const {loadRegistry}=await import('/dist/package-web/package/registry.js');
    const oracle=await import('/dist/evidence/pele-oracle.js');
    const engine=await loadRegistry(undefined,{allBuiltins:true}),failures=[],source='pie\n"A":1',options={measurer:{width:(text,size,style)=>text.length*size*(style&1?.71:.6)}};
    const model=engine.parse(source),expected=oracle.parse(source),equal=(a,b)=>JSON.stringify(a)===JSON.stringify(b);
    const circular={};circular.self=circular;
    for(const config of [{pie:{legendPosition:'left'}},circular,1]){
      const invalid={...options,limit:-1,maxEdges:-1,config};
      try{if(!equal(engine.render(model,invalid),oracle.render(expected,invalid)))failures.push('retained parse-only options ignored');}catch(e){failures.push('retained rejected unused config: '+e.message);}
    }
    let error;
    try{engine.parse('pie\n'+'\0'.repeat(6*1024*1024),{limit:Infinity});}catch(e){error=e;}
    if(error?.name!=='PeleError'||error.code!=='limit'||error.message!=='Browser numeric transport exceeds byte budget'||new TextEncoder().encode(JSON.stringify(error)).length>64*1024*1024)failures.push('escaped registry.parse failure bounded');
    const recovered=engine.parse(source,{limit:NaN,maxEdges:Infinity});
    if(!equal(engine.render(recovered,options),oracle.render(source,options)))failures.push('registry recovers after bounded error');
    engine.dispose(model);engine.dispose(recovered);engine.close();return failures;
  });
  expect(failures).toEqual([]);
});
