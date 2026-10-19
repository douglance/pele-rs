import {test,expect} from '@playwright/test';

test('public PeleError constructors and native failures retain the pinned prototype contract',async({page})=>{
  await page.goto('/examples/web/');
  const failures=await page.evaluate(async()=>{
    const full=await import('/dist/package-web/package/full.js'),core=await import('/dist/package-web/package/core.js'),oracle=await import('/dist/evidence/pele-oracle.js');
    const engine=await full.loadRenderer(),inspection=await core.loadCore(),failures=[];
    if(full.PeleError!==core.PeleError||full.PeleCompatibilityError!==full.PeleError||core.PeleCompatibilityError!==full.PeleError)failures.push('one public error constructor');
    if(full.PeleError.length!==oracle.PeleError.length)failures.push('constructor arity');
    if(Object.getPrototypeOf(full.PeleError.prototype)!==Error.prototype)failures.push('genuine Error inheritance');
    const compare=(actual,expected,stage)=>{
      if(!(actual instanceof full.PeleError)||!(actual instanceof Error)||Object.getPrototypeOf(actual)!==full.PeleError.prototype)failures.push(stage+' prototype');
      if(JSON.stringify(Reflect.ownKeys(actual))!==JSON.stringify(Reflect.ownKeys(expected)))failures.push(stage+' own keys');
      for(const key of Reflect.ownKeys(expected)) {
        const a=Object.getOwnPropertyDescriptor(actual,key),b=Object.getOwnPropertyDescriptor(expected,key);
        if(!a||a.enumerable!==b.enumerable||a.writable!==b.writable||a.configurable!==b.configurable)failures.push(stage+' flags '+key);
        if(!['stack','fileName','lineNumber','columnNumber','sourceURL'].includes(key)&&!Object.is(a?.value,b.value))failures.push(stage+' value '+key);
      }
      if(actual.toString()!==expected.toString())failures.push(stage+' Error.toString');
    };
    const vectors=[[],['Message'],['Detailed','semantic',{type:'flowchart',line:12,column:3,snippet:'A\n--^'}],['',undefined,{line:NaN,column:-0,type:null,snippet:''}],['\ud800','limit',{line:Infinity,column:-Infinity,type:'\udc00',snippet:'\ud800'}]];
    for(const args of vectors)compare(new full.PeleError(...args),new oracle.PeleError(...args),'constructor '+args.length);
    for(const [stage,actualCall,expectedCall]of[
      ['parse',()=>engine.parse('not a diagram'),()=>oracle.parse('not a diagram')],
      ['inspect',()=>inspection.inspect('pie\n"A":1',{limit:1}),()=>oracle.parse('pie\n"A":1',{limit:1})],
      ['output limit',()=>engine.render('pie\n"A":1',{outputLimit:1}),()=>oracle.render('pie\n"A":1',{outputLimit:1})],
      ['RangeError guard',()=>engine.render('pie\n"A":1',{measurer:{width(){throw new RangeError('host')}}}),()=>oracle.render('pie\n"A":1',{measurer:{width(){throw new RangeError('host')}}})]
    ]){
      let actual,expected;try{actualCall()}catch(e){actual=e}try{expectedCall()}catch(e){expected=e}
      if(!actual||!expected)failures.push(stage+' failed to throw');else compare(actual,expected,stage);
    }
    const model=engine.parse('timeline\n2025: Rust');
    let actual,expected;try{model.addEvent('\ud800')}catch(e){actual=e} // Valid storage; no forced scalar conversion.
    if(actual)failures.push('unused UTF16 storage');
    engine.dispose(model);
    return failures;
  });
  expect(failures).toEqual([]);
});
