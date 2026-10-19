// Pinned Pele458322a5 eventmodeling/render.ts84/259 and railroad/render.ts218.
import {test,expect} from '@playwright/test';
for(const family of ['eventmodeling','railroad'])test('default mono font follows pinned '+family+' source, retained and raw dispatch',async({page})=>{
 await page.goto('/examples/web/');const report=await page.evaluate(async family=>{
  const {loadRenderer,forgetTextWidths}=await import('/dist/package-web/package/full.js');
  const {loadDiagramEntries,disposeDiagramModel}=await import('/dist/package-web/package/diagram-entries.js');await loadDiagramEntries();
  const oracle=await import('/dist/evidence/pele-esm/core.js'),entry=(await import('/dist/package-web/package/diagrams/'+family+'.js')).default,pinned=(await import('/dist/evidence/pele-esm/diagrams/'+family+'.js')).default;
  oracle.register(pinned);const engine=await loadRenderer();
  const originalCanvas=globalThis.OffscreenCanvas,traces=[[],[]],failures=[];let side=0;
  globalThis.OffscreenCanvas=class {getContext(){return{font:'10px sans-serif',measureText(text){traces[side].push([text,this.font]);return{width:text.length*(this.font.includes('AuditMono')?17:this.font.includes('monospace')?7:5)};}};}};
  const source=family==='eventmodeling'?'eventmodeling\ntf 01 cmd RunAction { identifier: 123 }\n':'railroad-abnf-beta\nrule = "wide" / "item" ;\n';
  const options={fontFamily:'AuditSans',fontFamilyMono:'AuditMono',maxWidth:200};
  const equal=(a,b)=>JSON.stringify(a)===JSON.stringify(b),check=(yes,stage,detail)=>{if(!yes)failures.push({stage,detail});};
  const compare=(actual,wanted,stage,mono='AuditMono')=>{traces[0]=[];traces[1]=[];forgetTextWidths();side=0;const a=actual();oracle.forgetTextWidths();side=1;const b=wanted();check(equal(a,b),stage+' exact SVG/dimensions');check(equal(traces[0],traces[1]),stage+' complete canvas measurement order',{native:traces[0],oracle:traces[1]});check(traces[0].some(([,font])=>font.includes(mono)),stage+' requested mono family used');};
  let model,reference,raw,rawReference;
  try{
   compare(()=>engine.render(source,options),()=>oracle.render(source,options),'source');
   const standard={...options,fontFamilyMono:undefined};compare(()=>engine.render(source,standard),()=>oracle.render(source,standard),'omitted mono default','monospace');
   model=engine.parse(source);reference=oracle.parse(source);model.type=reference.type='pie';model.title=reference.title='Retained';
   compare(()=>engine.render(model,options),()=>oracle.render(reference,options),'retained registration survives edited type');
   raw=entry.parse(source,{},undefined,0,{maxEdges:5000});rawReference=pinned.parse(source,{},undefined,0,{maxEdges:5000});
   compare(()=>entry.render(raw,{},options),()=>pinned.render(rawReference,{},options),'raw');
   for(const [text,extra]of [[source,{limit:-1}],['---\ninvalid: [\n---\n'+source,{limit:-1}],['---\ninvalid: [\n---\n'+source,{}]]){
    const errors=[];traces[0]=[];traces[1]=[];for(const [i,renderer]of [engine,oracle].entries()){side=i;try{renderer.render(text,{...options,...extra});}catch(e){errors.push({name:e.name,message:e.message,code:e.code,type:e.type,line:e.line,column:e.column,snippet:e.snippet});}}
    check(errors.length===2&&equal(errors[0],errors[1]),'source/preparation/semantic limit error ordering',errors);check(traces[0].length===0&&traces[1].length===0,'failed source never starts default measurement');
   }
   // Custom measurers bypass native font selection and retain the ordinary API.
   const custom={...options,measurer:{width:(text,size)=>text.length*size*.6}};
   check(equal(engine.render(source,custom),oracle.render(source,custom)),'explicit custom measurer preserved');
  }finally{if(model)engine.dispose(model);if(raw)disposeDiagramModel(raw);engine.close();forgetTextWidths();oracle.forgetTextWidths();globalThis.OffscreenCanvas=originalCanvas;}
  return{failures};
 },family);expect(report.failures).toEqual([]);
});
