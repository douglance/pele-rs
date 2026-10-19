import {test, expect} from '@playwright/test';

test('numeric render options: all declared pinned cases across source, immutable and mutable WASM lanes', async ({page}) => {
  test.setTimeout(180000);
  await page.goto('/examples/web/');
  const report = await page.evaluate(async () => {
    const {loadRenderer} = await import('/dist/package-web/package/full.js');
    const wasm = await import('/dist/package-web/package/pkg/pele.js');
    const {renderNumericOptions} = await import('/dist/package-web/package/render-numbers.js');
    const fixture = await (await fetch('/fixtures/pele/render-numbers.json')).json(), engine = await loadRenderer();
    const buffer = new ArrayBuffer(8), view = new DataView(buffer), failures = [], outputs = new Map();
    const bits = number => {view.setFloat64(0, number, false); return view.getBigUint64(0, false).toString(16).padStart(16, '0');};
    const number = hex => {view.setBigUint64(0, BigInt('0x'+hex), false); return view.getFloat64(0, false);};
    const units = text => Array.from({length:text.length}, (_, index) => text.charCodeAt(index));
    const normalizedBits = hex => Number.isNaN(number(hex)) ? 'computed-NaN' : hex;
    const graph = root => {
      const prototype = Object.getPrototypeOf(root), ids = new Map(), nodes = [];
      const atom = value => {
        if (value === undefined) return ['undefined']; if (value === null) return ['null'];
        if (typeof value === 'boolean') return ['bool', value];
        if (typeof value === 'string') return ['string', units(value)];
        if (typeof value === 'number') return ['number', bits(value)];
        if (typeof value === 'function') throw new Error('unexpected own fixture function');
        if (!ids.has(value)) {ids.set(value, ids.size); nodes.push(null);} return ['ref', ids.get(value)];
      };
      const r = atom(root);
      for (const [value, id] of ids) {
        const kind = Array.isArray(value) ? 'array' : value instanceof Map ? 'map' : value instanceof Set ? 'set' : value instanceof Date ? 'date' : prototype !== Object.prototype && Object.getPrototypeOf(value) === prototype ? 'instance' : 'object';
        const node = {kind};
        if (kind === 'instance') node.prototype = 1;
        if (kind === 'array') node.length = value.length;
        if (kind === 'map') node.entries = [...value].map(([key, value]) => [atom(key), atom(value)]);
        if (kind === 'set') node.values = [...value].map(atom);
        if (kind === 'date') node.milliseconds = bits(+value);
        node.properties = Object.getOwnPropertyNames(value).filter(key => !(kind === 'array' && key === 'length')).map(key => {
          const d = Object.getOwnPropertyDescriptor(value, key);
          return [units(key), atom(d.value), (d.enumerable?1:0)|(d.writable?2:0)|(d.configurable?4:0)];
        }); nodes[id] = node;
      }
      return {root:r, nodes};
    };
    const normalizeGraph = input => JSON.stringify(input, (_, value) => {
      if (Array.isArray(value) && value[0] === 'number') return ['number', normalizedBits(value[1])];
      if (value?.kind === 'date') return {...value, milliseconds:normalizedBits(value.milliseconds)};
      return value;
    });
    const trace = calls => calls.map(call => call[0] === 'clock' ? call : [call[0],call[1],normalizedBits(call[2]),call[3],normalizedBits(call[4])]);
    let assertions = 0, failuresCount = 0, executed = 0, excluded = 0, pairs = 0;
    const check = (value, id, stage) => {assertions++; if (!value) {failuresCount++;if (failures.length < 20) failures.push(`${id}: ${stage}`);}};
    const equal = (a,b) => JSON.stringify(a) === JSON.stringify(b);
    const decodeTag = tag => tag.kind === 'undefined' ? undefined : tag.kind === 'null' ? null : tag.kind === 'date' ? new Date(number(tag.bits)) : number(tag.bits);
    const errorValue = error => ({name:error.name,message:error.message,...(error.name === 'PeleError' ? {code:error.code,type:error.type,line:error.line,column:error.column,snippet:error.snippet}: {})});
    const decode = json => {
      const envelope = JSON.parse(json);
      if (!envelope.ok) throw Object.assign(new Error(envelope.error.message), envelope.error);
      const value = envelope.result;
      if (value.$numbers) {for (const key of ['width','height']) if (value.$numbers[key]) value[key] = number(value.$numbers[key]); delete value.$numbers;}
      return value;
    };
    for (const c of fixture.cases) {
      if (c.boundary) {
        check(c.boundary === 'runtime-null-now-outside-declared-packet' && c.values.now.kind === 'null', c.id, 'explicit null-now boundary ledger'); excluded++; continue;
      }
      for (const lane of ['source','immutable','mutable']) {
        const id = `${c.id}/${lane}`, calls = [], originalNow = Date.now, sentinel = new Error('render-numbers host clock stopped');
        const model = lane === 'mutable' ? engine.parse(c.source) : lane === 'immutable' ? wasm.parse_v2(c.source, '{}') : null;
        let clockThrew = false, result, error;
        const tasks = lane === 'mutable' ? model.tasks : null;
        if (lane === 'mutable') check(normalizeGraph(graph(model)) === normalizeGraph(c.expected.graphBefore), id, 'complete parsed graph');
        const options = {...c.legacyOptions}; for (const [key, tag] of Object.entries(c.values)) options[key] = decodeTag(tag);
        options.measurer = {width(text,size,style) {const width=text.length*size*(style&1?.71:.6);calls.push(['width',units(text),bits(size),style,bits(width)]);return width;}};
        Date.now = () => {calls.push(['clock',c.hostThrows?'throw':bits(c.hostNow?number(c.hostNow.bits):1704067200000)]);if(c.hostThrows){clockThrew=true;throw sentinel;}return c.hostNow?number(c.hostNow.bits):1704067200000;};
        try {
          if (lane === 'immutable') {
            const n=renderNumericOptions(options);
            const json=model.render_numbers_with_resources(n.json,n.mask,n.numbers,n.outputKind,n.output,(text,size,style)=>options.measurer.width(text,size,style),undefined,()=>Date.now());
            if(clockThrew) throw sentinel;
            result=decode(json);
          } else result=engine.render(lane==='source'?c.source:model,options);
        } catch (caught) {error=caught;}
        finally {Date.now=originalNow;}
        if (c.expected.error) {
          check(!!error && equal(errorValue(error),c.expected.error),id,'exact error');
          check(error===sentinel,id,'original clock exception identity');
        } else {
          check(!error,id,`successful render ${error?.message??''}`);
          check(equal(result,c.expected.result),id,'whole SVG and result');
          if(result) {check(normalizedBits(bits(result.width))===normalizedBits(c.expected.widthBits),id,'width IEEE');check(normalizedBits(bits(result.height))===normalizedBits(c.expected.heightBits),id,'height IEEE');}
        }
        check(equal(trace(calls),trace(c.expected.calls)),id,'complete ordered clock/measurement trace');
        let after;
        if(lane==='mutable') {
          after=graph(model);
          check(normalizeGraph(after)===normalizeGraph(c.expected.graphAfter),id,'published complete post-render graph');
          if(c.family==='gantt') {
            check(model.tasks===tasks&&model.tasks.every(task=>model.byId.get(task.id)===task),id,'tasks and byId aliases retained');
            if(error) {
              const original=Date.now;Date.now=()=>1704067200000;
              try {engine.render(model,{now:1704067200000,measurer:{width:(text,size,style)=>text.length*size*(style&1?.71:.6)}});}
              catch(caught){check(false,id,`recovery ${caught.message}`);}finally{Date.now=original;}
            }
          }
        }
        outputs.set(id,JSON.stringify({result,error:error?errorValue(error):null,calls:trace(calls),after:after?normalizeGraph(after):null,width:result?normalizedBits(bits(result.width)):null,height:result?normalizedBits(bits(result.height)):null}));
        if(lane==='mutable')engine.dispose(model);else if(model)model.free();
        executed++;
      }
    }
    for(const pair of fixture.pairs) for(const lane of ['source','immutable','mutable']) {
      check(pair.compared && outputs.get(`${pair.left}/${lane}`)===outputs.get(`${pair.right}/${lane}`),pair.left+'/'+lane,'Date versus TimeClip number equivalent');pairs++;
    }
    return {failures,failuresCount,executed,excluded,pairs,assertions};
  });
  expect(report.failures).toEqual([]);
  expect(report).toMatchObject({failuresCount:0,executed:861,excluded:3,pairs:81});
});

test('numeric options preserve five raw IEEE inputs and Date semantics without changing remaining JSON options', async ({page}) => {
  await page.goto('/examples/web/');
  const report=await page.evaluate(async()=>{
    const {loadRenderer}=await import('/dist/package-web/package/full.js');
    const {renderNumericOptions}=await import('/dist/package-web/package/render-numbers.js');
    const engine=await loadRenderer(),wasm=await import('/dist/package-web/package/pkg/pele.js');
    const data=new DataView(new ArrayBuffer(8));
    let seedBits;
    const read=hex=>{data.setBigUint64(0,BigInt('0x'+hex));const value=data.getFloat64(0);if(hex==='fff8000000000042')seedBits=data.getBigUint64(0).toString(16).padStart(16,'0');return value;};
    const bits=n=>{data.setFloat64(0,n);return data.getBigUint64(0).toString(16).padStart(16,'0');};
    const expected=['fff8000000000042','7ff0000000000000','fff0000000000000','8000000000000000','4278cc251f40000e'];
    const options={fontSize:read(expected[0]),maxWidth:read(expected[1]),directionBreakpoint:read(expected[2]),padding:read(expected[3]),now:read(expected[4]),config:{retained:{value:'kept'}},idPrefix:'kept',outputLimit:Infinity};
    // Some VMs canonicalize NaN when a DataView value becomes a JS Number.
    // The adapter must preserve the Number it receives, including its raw bits.
    const inputBits=['fontSize','maxWidth','directionBreakpoint','padding','now'].map(key=>bits(options[key]));
    const n=renderNumericOptions(options),originalDate=new Date(-.875),date=renderNumericOptions({now:originalDate});
    const source='pie\n"A":1',measurer={width:(text,size,style)=>text.length*size*(style&1?.71:.6)};
    const full=engine.render(source,{fontSize:13,padding:3,outputLimit:Infinity,measurer});
    const iframe=document.createElement('iframe');document.body.append(iframe);
    let crossRealm;
    try {
      const date=new iframe.contentWindow.Date(1704067200000.875),packet=renderNumericOptions({now:date});
      const gantt='gantt\ndateFormat YYYY-MM-DD\nA:active,1d';
      const a=engine.render(gantt,{now:date,measurer}),b=engine.render(gantt,{now:date.getTime(),measurer});
      crossRealm=packet.mask===16&&Object.is(packet.numbers[4],date.getTime())&&JSON.stringify(a)===JSON.stringify(b);
    } finally {iframe.remove();}

    const decode=json=>JSON.parse(json),values=new Float64Array([13,0,0,0,0]);
    const call=(json,mask=1,kind=3)=>decode(wasm.render_numbers_with_resources(source,json,mask,values,kind,0,undefined,undefined,undefined));
    const filtered=call('{"fontSize":{"ignored":true},"idPrefix":"safe"}');
    const duplicate=call('{"fontSize":1,"fontSize":2}');
    const malformed=call('{"fontSize":}');
    const other=call('{"fontSize":{},"maxWidth":{}}');
    const nested=call('{"fontSize":{},"config":{"retained":{"fontSize":4}}}');
    const trailing=call('{"fontSize":1} {}');
    const invalidMask=call('{}',32),invalidTag=call('{}',0,4);
    const budget=call('{"config":{"large":"'+'x'.repeat(1024*1024)+'"}}',0);
    // Inspect the packet bytes themselves: materializing a Number array may
    // canonicalize a NaN before the assertion observes the transport buffer.
    const packet=new DataView(n.numbers.buffer,n.numbers.byteOffset,n.numbers.byteLength);
    const packetBits=Array.from({length:5},(_,index)=>packet.getBigUint64(index*8,true).toString(16).padStart(16,'0'));
    return {mask:n.mask,bits:packetBits,inputBits,seedBits,json:JSON.parse(n.json),kind:n.outputKind,output:n.output===Infinity,original:Object.is(options.padding,-0)&&Number.isNaN(options.fontSize)&&options.outputLimit===Infinity,date:bits(date.numbers[4]),dateUnchanged:+originalDate===0,crossRealm,svg:full.svg.includes('font:13px'),filtered:filtered.ok,nested:nested.ok,rejected:[duplicate,malformed,other,trailing,invalidMask,invalidTag].every(value=>value.ok===false&&value.error.code==='semantic'),budget:budget.error.code};
  });
  const {bits,inputBits,...rest}=report;
  expect(bits).toEqual(inputBits);
  expect(inputBits.slice(1)).toEqual(['7ff0000000000000','fff0000000000000','8000000000000000','4278cc251f40000e']);
  expect(rest).toEqual({mask:31,seedBits:'fff8000000000042',json:{config:{retained:{value:'kept'}},idPrefix:'kept'},kind:2,output:true,original:true,date:'0000000000000000',dateUnchanged:true,crossRealm:true,svg:true,filtered:true,nested:true,rejected:true,budget:'limit'});
});

test('numeric output thresholds and Gantt callback failures publish state before recovery', async ({page}) => {
  test.setTimeout(90000);
  await page.goto('/examples/web/');
  const report=await page.evaluate(async()=>{
    const {loadRenderer,PeleError}=await import('/dist/package-web/package/full.js'),oracle=await import('/dist/evidence/pele-oracle.js');
    const wasm=await import('/dist/package-web/package/pkg/pele.js'),engine=await loadRenderer();
    const failures=[],check=(v,s)=>{if(!v)failures.push(s);};
    const measurer={width:(text,size,style)=>text.length*size*(style&1?.71:.6)};
    const source='gantt\ndateFormat YYYY-MM-DD\nsection S\nActive:active,1d\nNext:after task1,2d';
    const model=engine.parse(source),reference=oracle.parse(source),tasks=model.tasks;
    const oldNow=Date.now,clockError=new RangeError('clock identity'),widthError=new RangeError('width identity');
    const fields=error=>({name:error?.name,message:error?.message,code:error?.code,type:error?.type,line:error?.line,column:error?.column,snippet:error?.snippet});
    const guarded=(actual,expected)=>actual instanceof PeleError&&actual.name==='PeleError'&&actual.code==='limit'&&JSON.stringify(fields(actual))===JSON.stringify(fields(expected));
    let caught,pinned;
    Date.now=()=>{throw clockError;};
    try {
      try {engine.render(model,{now:NaN,measurer});}catch(error){caught=error;}
      try {oracle.render(reference,{now:NaN,measurer});}catch(error){pinned=error;}
    }finally{Date.now=oldNow;}
    check(guarded(caught,pinned)&&model.tasks===tasks,'pinned clock RangeError guard and retained tasks');
    Date.now=()=>1704067200000;
    try {
      try {engine.render(model,{now:Infinity,measurer:{width(){throw widthError;}}});}catch(error){caught=error;}
      try {oracle.render(reference,{now:Infinity,measurer:{width(){throw widthError;}}});}catch(error){pinned=error;}
    }finally{Date.now=oldNow;}
    check(guarded(caught,pinned)&&tasks.every(task=>task.processed)&&model.byId.get(tasks[0].id)===tasks[0],'scheduled state published before pinned width RangeError guard');
    const actual=engine.render(model,{now:1704067200000,measurer}),expected=oracle.render(reference,{now:1704067200000,measurer});
    check(JSON.stringify(actual)===JSON.stringify(expected),'complete recovery SVG');
    engine.dispose(model);
    const text='pie title Limit\n"A":1',full=engine.render(text,{outputLimit:Infinity,measurer});
    for(const limit of [undefined,null,NaN,Infinity,-Infinity,0,-0,full.svg.length-.5,full.svg.length,full.svg.length+.5]) {
      let a,b;
      try{a={result:engine.render(text,{outputLimit:limit,measurer})};}catch(error){a={error:{name:error.name,message:error.message,code:error.code,type:error.type,line:error.line,column:error.column,snippet:error.snippet}};}
      try{b={result:oracle.render(text,{outputLimit:limit,measurer})};}catch(error){b={error:{name:error.name,message:error.message,code:error.code,type:error.type,line:error.line,column:error.column,snippet:error.snippet}};}
      check(JSON.stringify(a)===JSON.stringify(b),'exact output threshold '+String(limit));
    }
    // Omitted outputLimit still runs the upstream post-render default check.
    // A huge emitted class ID must not hide an earlier host measurement throw.
    const classSource='classDiagram\nclass '+'A'.repeat(500100),classError=new Error('class default callback');
    for(const mutable of [false,true]) {
      const input=mutable?engine.parse(classSource,{limit:classSource.length}):classSource;
      let actual,expected,actualCalls=0,expectedCalls=0;
      try{engine.render(input,{limit:classSource.length,measurer:{width(){actualCalls++;throw classError;}}});}catch(error){actual=error;}
      try{oracle.render(classSource,{limit:classSource.length,measurer:{width(){expectedCalls++;throw classError;}}});}catch(error){expected=error;}
      check(actual===classError&&expected===classError&&actualCalls===expectedCalls&&actualCalls>0,'omitted default preserves Class callback precedence '+mutable);
      if(mutable)engine.dispose(input);
    }
    const tooLarge='pie\n'+'\0'.repeat(6*1024*1024),zeros=new Float64Array(5);
    const bounded=wasm.render_numbers_with_resources(tooLarge,'{"limit":16777216}',0,zeros,3,0,undefined,undefined,undefined);
    check(new TextEncoder().encode(bounded).length<=64*1024*1024&&JSON.parse(bounded).error.code==='limit','escaped oversized error bounded without source truncation');
    return failures;
  });
  expect(report).toEqual([]);
});
