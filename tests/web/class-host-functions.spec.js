import {test,expect} from '@playwright/test';

const source='classDiagram\nclass A {\n +value$\n +run(x) int*\n}\nclass B {\n +other\n}\nA --> B';

test('Class live member callbacks preserve nested originals, shared return objects and exact drawing order',async({page})=>{
 await page.goto('/examples/web/');
 const result=await page.evaluate(async source=>{
  const {loadRenderer}=await import('/dist/package-web/package/full.js'),oracle=await import('/dist/evidence/pele-oracle.js'),native=await loadRenderer();
  function exercise(api){const model=api.parse(source),member=model.classes.get('A').members[0],original=member.getDisplayDetails,classifier=member.parseClassifier,trace=[],checks=[];
   member.parseClassifier=function(){trace.push(['classifier',this===member]);this.classifier='*';return classifier.call(this);};
   member.getDisplayDetails=function(){trace.push(['display',this===member]);const display=original.call(this);checks.push(display.cssStyle==='font-style:italic;');model.savedMember=this;model.savedDisplay=display;display.displayText='Live label';return display;};
   const options={measurer:{width(text,size,style){trace.push(['width',text,size,style]);return text.length*size*.6;}}};
   const first=api.render(model,options);checks.push(model.savedMember===member,model.savedDisplay.displayText==='Live label');
   // The captured original remains callable outside rendering and performs the
   // current receiver's live parseClassifier lookup, rather than a cached copy.
   const direct=original.call(member);checks.push(direct.cssStyle==='font-style:italic;',direct.displayText==='+value');
   const second=api.render(model,options);checks.push(first.svg===second.svg);api.dispose?.(model);api.close?.();return{first,second,direct,trace,checks};
  }return{native:exercise(native),oracle:exercise(oracle)};
 },source);
 expect(result.native).toEqual(result.oracle);expect(result.native.checks.every(Boolean)).toBe(true);
});

test('Class callbacks retain exact thrown identity, partial objects, readonly errors and recovery before output policy',async({page})=>{
 await page.goto('/examples/web/');
 const result=await page.evaluate(async source=>{
  const {loadRenderer}=await import('/dist/package-web/package/full.js'),oracle=await import('/dist/evidence/pele-oracle.js'),native=await loadRenderer();
  function exercise(api){const model=api.parse(source),member=model.classes.get('A').members[0],original=member.getDisplayDetails,classifier=member.parseClassifier,checks=[],trace=[],states=[];
   const options={outputLimit:0,measurer:{width(text,size,style){trace.push(['width',text,size,style]);return text.length*size*.6;}}};
   for(const marker of [undefined,null,false,-0,'Class throw',new Error('Class error'),{marker:true}]){
    member.getDisplayDetails=function(){const value=original.call(this);model.savedMember=this;model.savedDisplay=value;value.displayText='Before throw';throw marker;};
    let caught=false;try{api.render(model,options);}catch(error){caught=true;checks.push(Object.is(error,marker));}
    checks.push(caught,model.savedMember===member,model.savedDisplay.displayText==='Before throw');states.push([member.id,member.classifier,model.savedDisplay.displayText]);
   }
   member.getDisplayDetails=original;member.parseClassifier=function(){'use strict';this.partial='Kept';this.classifier='*';return classifier.call(this);};
   Object.defineProperty(member,'classifier',{writable:false,configurable:true});
   try{original.call(member);checks.push(false);}catch(error){checks.push(error instanceof TypeError,member.partial==='Kept');}
   Object.defineProperty(member,'classifier',{writable:true,configurable:true});
   const drawing=api.render(model,{...options,outputLimit:undefined});checks.push(drawing.svg.includes('value'));
   api.dispose?.(model);api.close?.();return{checks,states,trace,drawing};
  }return{native:exercise(native),oracle:exercise(oracle)};
 },source);
 expect(result.native).toEqual(result.oracle);expect(result.native.checks.every(Boolean)).toBe(true);
});

test('prepared Class raw rendering uses the same live methods and retains raw output policy',async({page})=>{
 await page.goto('/examples/web/');
 const result=await page.evaluate(async source=>{
  const entries=await import('/dist/package-web/package/diagram-entries.js');await entries.loadDiagramEntries();
  const native=(await import('/dist/package-web/package/diagrams/class.js')).default,oracle=(await import('/dist/evidence/pele-esm/diagrams/class.js')).default;
  function exercise(api,dispose){const model=api.parse(source,{},undefined,0,{maxEdges:undefined}),member=model.classes.get('A').members[0],original=member.getDisplayDetails,trace=[],checks=[];
   member.getDisplayDetails=function(){trace.push('display');const value=original.call(this);value.displayText='Raw live label';model.savedDisplay=value;return value;};
   const drawing=api.render(model,{}, {outputLimit:0,limit:-1,maxEdges:-1,measurer:{width(text,size,style){trace.push(['width',text,size,style]);return text.length*size*.6;}}});
   checks.push(drawing.svg.includes('Raw live label'),model.savedDisplay.displayText==='Raw live label');dispose?.(model);return{drawing,trace,checks};
  }return{native:exercise(native,entries.disposeDiagramModel),oracle:exercise(oracle)};
 },source);
 expect(result.native).toEqual(result.oracle);expect(result.native.checks.every(Boolean)).toBe(true);
});
