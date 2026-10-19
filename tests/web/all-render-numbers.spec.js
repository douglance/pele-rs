import {test, expect} from '@playwright/test';

test('all 33 numeric families preserve results, callbacks and complete mutable graphs in WASM', async ({page}) => {
  test.setTimeout(180000);
  await page.goto('/examples/web/');
  const report = await page.evaluate(async () => {
    const {loadRenderer} = await import('/dist/package-web/package/full.js');
    const wasm = await import('/dist/package-web/package/pkg/pele.js');
    const oracle = await import('/dist/evidence/pele-oracle.js');
    const {renderNumericOptions} = await import('/dist/package-web/package/render-numbers.js');
    const fixture = await (await fetch('/fixtures/pele/all-render-numbers.json')).json();
    const engine = await loadRenderer(), data = new DataView(new ArrayBuffer(8));
    const bits = n => { data.setFloat64(0,n); return data.getBigUint64(0).toString(16).padStart(16,'0'); };
    const number = hex => { data.setBigUint64(0,BigInt('0x'+hex)); return data.getFloat64(0); };
    const normalizedBits = hex => Number.isNaN(number(hex)) ? 'computed NaN' : hex;
    const units = text => Array.from({length:text.length},(_,i)=>text.charCodeAt(i));
    function graph(root) {
      const builtin = new Set([Object.prototype,Array.prototype,Map.prototype,Set.prototype,Date.prototype,null]);
      const prototypes = new Map(), ids = new Map(), nodes = [], functions = [];
      if (root && typeof root === 'object' && !builtin.has(Object.getPrototypeOf(root))) prototypes.set(Object.getPrototypeOf(root),1);
      const atom = value => {
        if (value === undefined) return ['undefined']; if (value === null) return ['null'];
        if (typeof value === 'boolean') return ['bool',value];
        if (typeof value === 'string') return ['string',units(value)];
        if (typeof value === 'number') return ['number',bits(value)];
        if (typeof value === 'function') {
          // Observe identical intrinsic materialization order in both native and
          // live oracle helpers; preserve the VM's resulting raw own-key order.
          if (Object.hasOwn(value, 'prototype')) void value.prototype;
          let index = functions.findIndex(entry=>entry.value===value);
          if (index < 0) { index=functions.length; functions.push({value,shape:{arity:value.length,ownKeys:Reflect.ownKeys(value)}}); }
          return ['functionObservation',index];
        }
        if (!ids.has(value)) { ids.set(value,ids.size); nodes.push(null); }
        return ['ref',ids.get(value)];
      };
      const r=atom(root);
      for (const [value,id] of ids) {
        const prototype=Object.getPrototypeOf(value);
        const kind=Array.isArray(value)?'array':value instanceof Map?'map':value instanceof Set?'set':value instanceof Date?'date':!builtin.has(prototype)?'instance':'object';
        if (kind==='instance'&&!prototypes.has(prototype)) prototypes.set(prototype,prototypes.size+1);
        const node={kind}; if(kind==='instance') node.prototype=prototypes.get(prototype); if(kind==='array') node.length=value.length;
        if(kind==='map') node.entries=[...value].map(([key,value])=>[atom(key),atom(value)]);
        if(kind==='set') node.values=[...value].map(atom); if(kind==='date') node.milliseconds=bits(+value);
        node.properties=Object.getOwnPropertyNames(value).filter(key=>!(kind==='array'&&key==='length')).map(key=>{
          const d=Object.getOwnPropertyDescriptor(value,key);
          if(!('value' in d))throw Error('Unexpected fixture accessor');
          return [units(key),atom(d.value),(d.enumerable?1:0)|(d.writable?2:0)|(d.configurable?4:0)];
        }); nodes[id]=node;
      }
      return {root:r,nodes,...(functions.length?{functionObservations:functions.map(entry=>entry.shape)}:{})};
    }
    // Function intrinsic property order is supplied by this browser VM. Keep
    // the fixture's complete data graph; compare functions to live pinned Pele.
    const vmFunctions = new Map();
    for (const c of fixture.cases) if (c.id.startsWith('baseline/') && c.expected.graphBefore.functionObservations) {
      vmFunctions.set(c.family, graph(oracle.parse(c.source)).functionObservations);
    }
    const expectedGraph = (value, family) => vmFunctions.has(family) ? {...value, functionObservations: vmFunctions.get(family)} : value;
    const normalizeGraph = value => {
      const copy={root:value.root,nodes:value.nodes};
      if(value.functionObservations)copy.functionObservations=value.functionObservations.map(({arity,ownKeys})=>({arity,ownKeys}));
      return JSON.stringify(copy,(_,v)=>Array.isArray(v)&&v[0]==='number'?['number',normalizedBits(v[1])]:v?.kind==='date'?{...v,milliseconds:normalizedBits(v.milliseconds)}:v);
    };
    const normalizedCalls = calls => JSON.stringify(calls.map(call=>call[0]==='clock'?call:[call[0],call[1],normalizedBits(call[2]),call[3],normalizedBits(call[4])]));
    const decode = json => {
      const envelope=JSON.parse(json);if(!envelope.ok)throw Object.assign(Error(envelope.error.message),envelope.error);
      const result=envelope.result;if(result.$numbers){for(const [key,value] of Object.entries(result.$numbers))result[key]=number(value);delete result.$numbers;}return result;
    };
    let executed=0, assertions=0, failureCount=0;const failures=[];
    const check=(ok,id,stage)=>{assertions++;if(!ok){failureCount++;if(failures.length<30)failures.push({id,stage});}};
    const originalNow=Date.now;
    try {
      for(const c of fixture.cases)for(const lane of ['source','immutable','mutable']) {
        const id=c.id+'/'+lane,calls=[],options={...c.legacyOptions};
        for(const [key,value] of Object.entries(c.numeric))options[key]=number(value.bits);
        options.measurer={width(text,size,style){const width=text.length*size*(style&1?.71:.6);calls.push(['width',units(text),bits(size),style,bits(width)]);return width;}};
        Date.now=()=>{calls.push(['clock',bits(1704067200000)]);return 1704067200000;};
        let model,result;
        try {
          model=lane==='mutable'?engine.parse(c.source):lane==='immutable'?wasm.parse_v2(c.source,'{}'):null;
          if(lane==='mutable')check(normalizeGraph(graph(model))===normalizeGraph(expectedGraph(c.expected.graphBefore,c.family)),id,'whole graph before');
          if(lane==='immutable'){
            const n=renderNumericOptions(options);
            result=decode(model.render_numbers_with_resources(n.json,n.mask,n.numbers,n.outputKind,n.output,(text,size,style)=>options.measurer.width(text,size,style),undefined,()=>Date.now()));
          }else result=engine.render(lane==='source'?c.source:model,options);
          check(JSON.stringify(result)===JSON.stringify(c.expected.result),id,'whole result/SVG');
          check(normalizedBits(bits(result.width))===normalizedBits(c.expected.widthBits),id,'width IEEE');
          check(normalizedBits(bits(result.height))===normalizedBits(c.expected.heightBits),id,'height IEEE');
          check(normalizedCalls(calls)===normalizedCalls(c.expected.calls),id,'ordered resource callbacks');
          if(lane==='mutable')check(normalizeGraph(graph(model))===normalizeGraph(expectedGraph(c.expected.graphAfter,c.family)),id,'whole graph after');
        }catch(error){check(false,id,'unexpected '+error.name+': '+error.message);}
        finally{if(lane==='mutable'&&model)engine.dispose(model);else model?.free();}
        executed++;
      }
    }finally{Date.now=originalNow;}
    return {executed,assertions,failureCount,failures,families:fixture.families,vmFunctionShapes:Object.fromEntries(vmFunctions)};
  });
  expect(report.failures).toEqual([]);
  expect(report.failureCount).toBe(0);
  expect(report.executed).toBe(3069);
});
