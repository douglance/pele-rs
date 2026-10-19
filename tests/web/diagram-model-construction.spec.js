import {test,expect} from '@playwright/test';

test('all33 low-level entries render caller-authored declared graphs without parsing',async({page},testInfo)=>{
 test.setTimeout(120000);await page.goto('/examples/web/');
 const report=await page.evaluate(async()=>{
  const fixture=await(await fetch('/fixtures/pele/diagram-model-construction.json')).json();
  const ready=await import('/dist/package-web/package/diagram-entries.js');await ready.loadDiagramEntries();
  const view=new DataView(new ArrayBuffer(8)),bits=n=>{view.setFloat64(0,n);return view.getBigUint64(0).toString(16).padStart(16,'0');},number=hex=>{view.setBigUint64(0,BigInt('0x'+hex));return view.getFloat64(0);},units=s=>Array.from({length:s.length},(_,i)=>s.charCodeAt(i));
  const equal=(a,b)=>JSON.stringify(a)===JSON.stringify(b),failures=[],ledger=[];
  const check=(v,id,stage,detail)=>{if(!v&&failures.length<30)failures.push({id,stage,detail});};
  const diff=(a,b)=>{const x=JSON.stringify(a),y=JSON.stringify(b);let i=0;while(i<x.length&&x[i]===y[i])i++;return{offset:i,actual:x.slice(Math.max(0,i-80),i+200),expected:y.slice(Math.max(0,i-80),i+200)};};
    function graph(root){const ids=new Map(),queue=[],nodes=[],functions=[],prototypes=new Map(),builtin=new Set([Object.prototype,Array.prototype,Map.prototype,Set.prototype,Date.prototype,null]);
      const atom=v=>{if(v===undefined)return['undefined'];if(v===null)return['null'];if(typeof v==='boolean')return['bool',v];if(typeof v==='number')return['number',bits(v)];if(typeof v==='string')return['string',units(v)];if(typeof v==='function'){if(Object.hasOwn(v,'prototype'))void v.prototype;let i=functions.findIndex(f=>f.value===v);if(i<0){i=functions.length;functions.push({value:v,arity:v.length,ownKeys:Reflect.ownKeys(v)});}return['functionObservation',i];}if(!ids.has(v)){ids.set(v,queue.length);queue.push(v);}return['ref',ids.get(v)];};
      const r=atom(root);for(let id=0;id<queue.length;id++){const value=queue[id],proto=Object.getPrototypeOf(value),kind=Array.isArray(value)?'array':value instanceof Map?'map':value instanceof Set?'set':value instanceof Date?'date':builtin.has(proto)?'object':'instance',node={kind};if(kind==='instance'){if(!prototypes.has(proto))prototypes.set(proto,prototypes.size+1);node.prototype=prototypes.get(proto);}if(kind==='array')node.length=value.length;if(kind==='map')node.entries=[...value].map(([k,v])=>[atom(k),atom(v)]);if(kind==='set')node.values=[...value].map(atom);if(kind==='date')node.milliseconds=bits(+value);node.properties=Object.getOwnPropertyNames(value).filter(k=>!(kind==='array'&&k==='length')).map(k=>{const d=Object.getOwnPropertyDescriptor(value,k);return[units(k),atom(d.value),(d.enumerable?1:0)|(d.writable?2:0)|(d.configurable?4:0)];});nodes.push(node);}return{root:r,nodes,...(functions.length?{functionObservations:functions.map(({arity,ownKeys})=>({arity,ownKeys}))}:{})};}

  // General graph hydration only: no parser, scheduler or diagram algorithm.
  function hydrate(input,family){
   const objects=input.nodes.map(n=>n.kind==='array'?new Array(n.length):n.kind==='map'?new Map():n.kind==='set'?new Set():n.kind==='date'?new Date(number(n.milliseconds)):{});
   const functions=(input.functionObservations??[]).map(()=>family==='gantt'?function getTasks(_now){return this.tasks;}:function semanticModel(){});
   const atom=a=>a[0]==='undefined'?undefined:a[0]==='null'?null:a[0]==='bool'?a[1]:a[0]==='number'?number(a[1]):a[0]==='string'?String.fromCharCode(...a[1]):a[0]==='ref'?objects[a[1]]:functions[a[1]];
   input.nodes.forEach((n,i)=>{const o=objects[i];if(n.kind==='map')for(const[k,v]of n.entries)o.set(atom(k),atom(v));if(n.kind==='set')for(const v of n.values)o.add(atom(v));for(const[k,v,f]of n.properties)Object.defineProperty(o,String.fromCharCode(...k),{value:atom(v),enumerable:!!(f&1),writable:!!(f&2),configurable:!!(f&4)});});return atom(input.root);
  }
  const errorView=e=>({name:e.name,message:e.message,...(e.name==='PeleError'?{code:e.code,type:e.type,line:e.line,column:e.column,snippet:e.snippet}:{})});
  for(const c of fixture.cases){
   const entry=(await import('/dist/package-web/package/diagrams/'+c.family+'.js')).default,reference=(await import('/dist/evidence/pele-esm/diagrams/'+c.family+'.js')).default;
   const model=hydrate(c.expected.before,c.family),wanted=hydrate(c.expected.before,c.family),extra=model.callerExtra,shared=extra.shared,original=entry.parse;
   entry.parse=()=>{throw Error('Fresh model renderer must not parse');};const traces=[[],[]];
   const options=i=>({...c.options,measurer:{width(text,size,style){const width=text.length*size*(style&1?.71:.6);traces[i].push(['width',units(text),bits(size),style,bits(width)]);return width;}},icons:name=>{traces[i].push(['icon',units(name)]);return'<path d="M0,0"/>';}});
   let actual,expected,error,referenceError;
   // Function intrinsic key materialization belongs to the current VM. Observe
   // both caller closures identically; data graph order/flags remain untouched.
   const liveBefore=graph(wanted),fixtureBefore={...c.expected.before,...(liveBefore.functionObservations?{functionObservations:liveBefore.functionObservations}:{})};
   try{check(equal(graph(model),fixtureBefore)&&equal(graph(model),liveBefore),c.id,'literal fixture input');try{actual=entry.render(model,c.config,options(0));}catch(e){error=e;}try{expected=reference.render(wanted,c.config,options(1));}catch(e){referenceError=e;}
    if(c.expected.error){check(error!==undefined,c.id,'native error present',actual);check(equal(errorView(error),c.expected.error),c.id,'native canonical exception',errorView(error));check(referenceError?.name===error.name,c.id,'live oracle exception class',referenceError&&errorView(referenceError));ledger.push({id:c.id,native:errorView(error),oracle:referenceError&&errorView(referenceError)});if(error.name==='PeleError')check(equal(errorView(error),errorView(referenceError)),c.id,'all Pele error fields');}
    else{check(!error&&!referenceError,c.id,'successful raw render',{native:error&&errorView(error),oracle:referenceError&&errorView(referenceError)});check(equal(actual,expected)&&equal(actual,c.expected.result),c.id,'exact complete raw SVG/result',diff(actual,expected));check(bits(actual.width)===c.expected.widthBits&&bits(actual.height)===c.expected.heightBits,c.id,'dimension IEEEbits');}
    check(equal(traces[0],traces[1])&&equal(traces[0],c.expected.calls),c.id,'complete ordered host callbacks',{actual:traces[0],expected:traces[1]});
    const liveAfter=graph(wanted),fixtureAfter={...c.expected.after,...(liveAfter.functionObservations?{functionObservations:liveAfter.functionObservations}:{})};
    check(equal(graph(model),liveAfter)&&equal(graph(model),fixtureAfter),c.id,'complete post-render graph',diff(graph(model),liveAfter));
    check(Object.getPrototypeOf(model)===Object.prototype&&model.callerExtra===extra&&extra.self===extra&&extra.shared===shared&&extra.alias===shared,c.id,'original literal root/aliases/cycle retained');
   }catch(e){failures.push({id:c.id,stage:'unexpected harness exception',error:errorView(e)});}finally{entry.parse=original;try{ready.disposeDiagramModel(model);}catch(e){failures.push({id:c.id,stage:'dispose',error:errorView(e)});}}
  }
  return{failures,ledger,cases:fixture.cases.length};
 });await testInfo.attach('fresh-model-intrinsic-diagnostics.json',{body:JSON.stringify(report.ledger,null,2),contentType:'application/json'});expect(report.failures).toEqual([]);expect(report.cases).toBe(67);
});

test('literal low-level models retain host failures, recovery, ownership and disposal',async({page})=>{
 await page.goto('/examples/web/');const report=await page.evaluate(async()=>{
  const {loadDiagramEntries,disposeDiagramModel}=await import('/dist/package-web/package/diagram-entries.js');await loadDiagramEntries();const pie=(await import('/dist/package-web/package/diagrams/pie.js')).default;
  const {loadRegistry}=await import('/dist/package-web/package/registry.js'),registry=await loadRegistry();const shared=[],model={type:'pie',sections:new Map([['Literal',3]]),showData:true,extra:{shared,alias:shared}};model.extra.self=model.extra;
  const token={host:'same'},measurer={width(){throw token;}};let identity=false;try{pie.render(model,{}, {measurer});}catch(e){identity=e===token;}
  const svg=pie.render(model,{}, {measurer:{width:(text,size)=>text.length*size*.6}}).svg;
  model.sections.set('Edit',2);const edited=pie.render(model,{}, {measurer:{width:(text,size)=>text.length*size*.6}}).svg;
  let coreRejected=false;try{registry.render(model);}catch(e){coreRejected=e instanceof TypeError;}
  disposeDiagramModel(model);let disposed=false;try{pie.render(model,{},{});}catch(e){disposed=e instanceof TypeError&&e.message==='Public model document is disposed';}registry.close();
  return{identity,svg:svg.includes('Literal'),edited:edited.includes('Edit'),coreRejected,disposed,aliases:model.extra.shared===shared&&model.extra.alias===shared&&model.extra.self===model.extra};
 });expect(report).toEqual({identity:true,svg:true,edited:true,coreRejected:true,disposed:true,aliases:true});
});
