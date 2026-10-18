import {test,expect} from '@playwright/test';

test('all 33 packed low-level entries preserve prepared models, raw rendering and native registration',async({page})=>{
  test.setTimeout(120000);await page.goto('/examples/web/');
  const report=await page.evaluate(async()=>{
    const fixture=await(await fetch('/fixtures/pele/diagram-entries.json')).json();
    const ready=await import('/dist/package-web/package/diagram-entries.js');await ready.loadDiagramEntries();
    const {loadRegistry}=await import('/dist/package-web/package/registry.js'),core=await loadRegistry();
    const oracleCore=await import('/dist/evidence/pele-esm/core.js');
    const view=new DataView(new ArrayBuffer(8)),bits=n=>{view.setFloat64(0,n);return view.getBigUint64(0).toString(16).padStart(16,'0');},units=s=>Array.from({length:s.length},(_,i)=>s.charCodeAt(i));
    const equal=(a,b)=>JSON.stringify(a)===JSON.stringify(b),failures=[],check=(v,id,stage)=>{if(!v&&failures.length<30)failures.push({id,stage});};
    function graph(root){const ids=new Map(),queue=[],nodes=[],functions=[],prototypes=new Map(),builtin=new Set([Object.prototype,Array.prototype,Map.prototype,Set.prototype,Date.prototype,null]);
      const atom=v=>{if(v===undefined)return['undefined'];if(v===null)return['null'];if(typeof v==='boolean')return['bool',v];if(typeof v==='number')return['number',bits(v)];if(typeof v==='string')return['string',units(v)];if(typeof v==='function'){if(Object.hasOwn(v,'prototype'))void v.prototype;let i=functions.findIndex(f=>f.value===v);if(i<0){i=functions.length;functions.push({value:v,arity:v.length,ownKeys:Reflect.ownKeys(v)});}return['functionObservation',i];}if(!ids.has(v)){ids.set(v,queue.length);queue.push(v);}return['ref',ids.get(v)];};
      const r=atom(root);for(let id=0;id<queue.length;id++){const value=queue[id],proto=Object.getPrototypeOf(value),kind=Array.isArray(value)?'array':value instanceof Map?'map':value instanceof Set?'set':value instanceof Date?'date':builtin.has(proto)?'object':'instance',node={kind};if(kind==='instance'){if(!prototypes.has(proto))prototypes.set(proto,prototypes.size+1);node.prototype=prototypes.get(proto);}if(kind==='array')node.length=value.length;if(kind==='map')node.entries=[...value].map(([k,v])=>[atom(k),atom(v)]);if(kind==='set')node.values=[...value].map(atom);if(kind==='date')node.milliseconds=bits(+value);node.properties=Object.getOwnPropertyNames(value).filter(k=>!(kind==='array'&&k==='length')).map(k=>{const d=Object.getOwnPropertyDescriptor(value,k);return[units(k),atom(d.value),(d.enumerable?1:0)|(d.writable?2:0)|(d.configurable?4:0)];});nodes.push(node);}return{root:r,nodes,...(functions.length?{functionObservations:functions.map(({arity,ownKeys})=>({arity,ownKeys}))}:{})};}
    const entries=new Map(),references=new Map();
    for(const metadata of fixture.entries){const entry=(await import('/dist/package-web/package/diagrams/'+metadata.type+'.js')).default,reference=(await import('/dist/evidence/pele-esm/diagrams/'+metadata.type+'.js')).default;entries.set(metadata.type,entry);references.set(metadata.type,reference);
      const inspect=d=>({type:d.type,...(Object.hasOwn(d,'section')?{section:d.section}:{}),...(Object.hasOwn(d,'keepComments')?{keepComments:d.keepComments}:{}),ownKeys:Object.keys(d),parseArity:d.parse.length,renderArity:d.render.length});
      check(equal(inspect(entry),inspect(reference)),metadata.type,'exact entry metadata/arity/presence');
      for(const key of Object.keys(reference)){const a=Object.getOwnPropertyDescriptor(entry,key),b=Object.getOwnPropertyDescriptor(reference,key);check(a.enumerable===b.enumerable&&a.writable===b.writable&&a.configurable===b.configurable,metadata.type,'entry descriptor '+key);}
      core.register(entry);oracleCore.register(reference);check(core.registered(entry.type),metadata.type,'real native entry registration');
    }
    let executed=0;
    for(const c of fixture.cases){const entry=entries.get(c.family),reference=references.get(c.family),title=c.title??undefined,limits={maxEdges:undefined};let model,expected;
      try{model=entry.parse(c.source,c.config,title,c.lineOffset,limits);expected=reference.parse(c.source,c.config,title,c.lineOffset,limits);
        check(equal(graph(model),graph(expected)),c.id,'complete prepared graph');
        let rejected=false;try{core.render(model);}catch(e){rejected=e instanceof TypeError;}check(rejected,c.id,'low-level model has separate core provenance');
        if(c.edit){Object.assign(model,c.edit);Object.assign(expected,c.edit);}
        const traces=[[],[]],options=index=>({...c.options,measurer:{width(text,size,style){const width=text.length*size*(style&1?.71:.6);traces[index].push(['width',units(text),bits(size),style,bits(width)]);return width;}},icons:name=>{traces[index].push(['icon',units(name)]);return'<path d="M0,0"/>';}});
        const actual=entry.render(model,c.config,options(0)),wanted=reference.render(expected,c.config,options(1));
        check(!Object.hasOwn(actual,'type'),c.id,'raw rendered result has no core type');if(!equal(actual,wanted)||!equal(actual,c.expected.result)){const snippet=(a,b)=>{const x=JSON.stringify(a),y=JSON.stringify(b);let i=0;while(i<x.length&&x[i]===y[i])i++;return{index:i,actual:x.slice(Math.max(0,i-70),i+150),expected:y.slice(Math.max(0,i-70),i+150)};};failures.push({id:c.id,stage:'exact raw SVG/result',live:snippet(actual,wanted),fixture:snippet(actual,c.expected.result)});} check(bits(actual.width)===c.expected.widthBits&&bits(actual.height)===c.expected.heightBits,c.id,'raw dimension bits');check(equal(traces[0],traces[1])&&equal(traces[0],c.expected.calls),c.id,'raw callback order');check(equal(graph(model),graph(expected)),c.id,'complete post-render graph');
        if(c.id.endsWith('/baseline')){const rendered=core.render(c.source,{now:fixture.host.now,measurer:options(0).measurer});check(rendered.type===c.family,c.id,'registered builtin uses core source path');}
        executed++;
      }catch(error){check(false,c.id,error.name+': '+error.message);}finally{if(model)ready.disposeDiagramModel(model);}
    }
    for(const c of fixture.coreIconCases){const traces=[[],[]],options=i=>({...c.options,measurer:{width(text,size,style){const width=text.length*size*(style&1?.71:.6);traces[i].push(['width',units(text),bits(size),style,bits(width)]);return width;}},icons:name=>{traces[i].push(['icon',units(name)]);return'<path d="M0,0"/>';}});const actual=core.render(c.source,options(0)),wanted=oracleCore.render(c.source,options(1));check(equal(actual,wanted)&&equal(actual,c.expected.result),c.id,'high-level SVG after direct-icon correction');check(equal(traces[0],traces[1])&&equal(traces[0],c.expected.calls),c.id,'high-level direct/global icon callback order');}
    core.close();return{failures,executed,entries:entries.size};
  });expect(report.failures).toEqual([]);expect(report.executed).toBe(99);expect(report.entries).toBe(33);
});

test('prepared entries keep raw parse/render policies, methods, partial state and host recovery',async({page})=>{
  await page.goto('/examples/web/');const failures=await page.evaluate(async()=>{
    const{loadDiagramEntries,disposeDiagramModel}=await import('/dist/package-web/package/diagram-entries.js');await loadDiagramEntries();
    const pie=(await import('/dist/package-web/package/diagrams/pie.js')).default,pinnedPie=(await import('/dist/evidence/pele-esm/diagrams/pie.js')).default;
    const flow=(await import('/dist/package-web/package/diagrams/flowchart.js')).default,pinnedFlow=(await import('/dist/evidence/pele-esm/diagrams/flowchart.js')).default;
    const gantt=(await import('/dist/package-web/package/diagrams/gantt.js')).default,pinnedGantt=(await import('/dist/evidence/pele-esm/diagrams/gantt.js')).default;
    const failures=[],check=(v,s)=>{if(!v)failures.push(s);},equal=(a,b)=>JSON.stringify(a)===JSON.stringify(b),error=e=>({name:e.name,message:e.message,...(e.name==='PeleError'?{code:e.code,type:e.type,line:e.line,column:e.column,snippet:e.snippet}:{})});
    const result=run=>{try{return{value:run()};}catch(e){return{error:error(e)};}};
    const options={now:1704067200000,measurer:{width:(text,size,style)=>text.length*size*(style&1?.71:.6)},outputLimit:1,limit:-1,maxEdges:-1,config:{unused:true}};
    for(const source of ['pie\n"Literal #35;":1','pie\n"ﬂ°°65¶ß":1','---\ntitle: No preprocessing\n---\npie\n"A":1']){
      const actual=result(()=>pie.parse(source,{},undefined,0,{maxEdges:undefined})),expected=result(()=>pinnedPie.parse(source,{},undefined,0,{maxEdges:undefined}));
      check(equal(actual.error,expected.error),'prepared parser errors and no preprocessing');if(actual.value){check(equal(pie.render(actual.value,{},options),pinnedPie.render(expected.value,{},options)),'raw entities/newline and ignored policies');disposeDiagramModel(actual.value);}
    }
    const text='flowchart TD\nA\nB\nC\nD\n',model=flow.parse(text,{},undefined,0,{maxEdges:2.5}),reference=pinnedFlow.parse(text,{},undefined,0,{maxEdges:2.5}),edges=model.edges;
    const link={type:'arrow_point',stroke:'normal',length:1};check(equal(result(()=>model.addLink(['A','B'],['C','D'],link)),result(()=>reference.addLink(['A','B'],['C','D'],link))),'raw fractional method error');check(model.edges===edges&&edges.length===3&&edges.every(edge=>model.edgeById.get(edge.id)===edge),'raw partial edges/aliases');model.options.maxEdges=reference.options.maxEdges=undefined;model.addSingleLink('B','D');reference.addSingleLink('B','D');check(equal(flow.render(model,{},options),pinnedFlow.render(reference,{},options)),'raw builder recovery');disposeDiagramModel(model);
    const source='gantt\ndateFormat YYYY-MM-DD\nsection S\nA:a,2024-01-01,1d\nB:b,after a,1d\n',tasks=gantt.parse(source,{},undefined,0,{maxEdges:undefined}),pinned=pinnedGantt.parse(source,{},undefined,0,{maxEdges:undefined}),sentinel=new RangeError('raw host failure');let nativeError,oracleError;
    try{gantt.render(tasks,{}, {...options,measurer:{width(){throw sentinel;}}});}catch(e){nativeError=e;}
    try{pinnedGantt.render(pinned,{}, {...options,measurer:{width(){throw sentinel;}}});}catch(e){oracleError=e;}
    check(nativeError===sentinel&&oracleError===sentinel,'raw host RangeError identity is not core guarded');check(tasks.tasks[0].processed&&tasks.tasks[1].startTime===tasks.tasks[0].endTime,'raw failed render publishes scheduling/Date aliases');check(equal(gantt.render(tasks,{},options),pinnedGantt.render(pinned,{},options)),'raw render recovery');disposeDiagramModel(tasks);
    return failures;
  });expect(failures).toEqual([]);
});

test('canonical built-in objects retain live handlers and sections across registry replacement and close',async({page})=>{
  await page.goto('/examples/web/');const report=await page.evaluate(async()=>{
    const {loadDiagramEntries,disposeDiagramModel}=await import('/dist/package-web/package/diagram-entries.js');await loadDiagramEntries();
    const pie=(await import('/dist/package-web/package/diagrams/pie.js')).default,pinned=(await import('/dist/evidence/pele-esm/diagrams/pie.js')).default;
    const {loadRegistry}=await import('/dist/package-web/package/registry.js'),{loadRenderer}=await import('/dist/package-web/package/full.js'),oracle=await import('/dist/evidence/pele-esm/core.js');
    const native=await loadRegistry(),trace=[[],[]],configs=[new WeakMap(),new WeakMap()],configCounts=[0,0],original=[pie.render,pinned.render],hadSection=[Object.hasOwn(pie,'section'),Object.hasOwn(pinned,'section')],sections=[pie.section,pinned.section],failures=[];
    const equal=(a,b)=>JSON.stringify(a)===JSON.stringify(b),check=(v,stage)=>{if(!v)failures.push(stage);};
    const source='pie\n"fas:fa-user A":1\n"B":2\n',settings={now:1704067200000,config:{pie:{useMaxWidth:false},other:{useMaxWidth:true}}};
    const hosts=i=>({now:settings.now,measurer:{width(text,size,style){const width=text.length*size*(style&1?.71:.6);trace[i].push(['width',text,size,style,width]);return width;}},icons:name=>{trace[i].push(['icon',name]);return'<path d="M0,0"/>';}});
    const wrap=(provider,i,label)=>function(model,config,options){if(!configs[i].has(config))configs[i].set(config,++configCounts[i]);trace[i].push(['handler',label,this===provider,configs[i].get(config),options.responsive,Object.keys(config)]);config.seen=(config.seen??0)+1;const result=original[i].call(this,model,config,options);return{...result,svg:result.svg.replace('<svg','<svg data-handler="'+label+'"')};};
    native.register(pie);oracle.register(pinned);const old=native.parse(source,settings),reference=oracle.parse(source,settings);
    check(equal(native.render(old,hosts(0)),oracle.render(reference,hosts(1))),'initial native entry render');check(equal(trace[0],trace[1]),'initial native direct/inline icons');
    pie.section=pinned.section='other';pie.render=wrap(pie,0,'old');pinned.render=wrap(pinned,1,'old');old.sections.set('C',3);reference.sections.set('C',3);
    const compare=(a,b,stage)=>{trace[0]=[];trace[1]=[];check(equal(native.render(a,hosts(0)),oracle.render(b,hosts(1))),stage+' SVG');check(equal(trace[0],trace[1]),stage+' complete host order/config identity');};
    compare(old,reference,'live old handler and changed section');compare(old,reference,'retained config mutation');
    const replacement={...pie},pinnedReplacement={...pinned};replacement.render=wrap(replacement,0,'new');pinnedReplacement.render=wrap(pinnedReplacement,1,'new');native.register(replacement);oracle.register(pinnedReplacement);
    const next=native.parse(source,settings),nextReference=oracle.parse(source,settings);compare(old,reference,'old registration survives replacement');compare(next,nextReference,'future parse uses replacement');
    pie.render=wrap(pie,0,'old-again');pinned.render=wrap(pinned,1,'old-again');native.close();compare(old,reference,'live old handler after owner close');compare(next,nextReference,'replacement model after owner close');
    oracle.register(pinned);const full=await loadRenderer();trace[0]=[];trace[1]=[];check(equal(full.render(source,{...settings,...hosts(0)}),oracle.render(source,{...settings,...hosts(1)})),'full entry uses canonical subpath object');check(equal(trace[0],trace[1]),'full canonical provider callback trace');
    pie.render=original[0];pinned.render=original[1];for(const [i,entry] of [pie,pinned].entries()){if(hadSection[i])entry.section=sections[i];else delete entry.section;}
    trace[0]=[];trace[1]=[];check(equal(full.render(source,{...settings,...hosts(0)}),oracle.render(source,{...settings,...hosts(1)})),'restored canonical entry returns native source path');check(equal(trace[0],trace[1]),'restored source callbacks');
    native.dispose(old);native.dispose(next);disposeDiagramModel(next);full.close();return{failures};
  });expect(report.failures).toEqual([]);
});
