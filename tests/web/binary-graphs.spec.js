import {test,expect} from '@playwright/test';

test('binary WASM consumers retain edited graph identities and reject malformed exchanges',async({page})=>{
  await page.goto('/examples/web/');
  const report=await page.evaluate(async()=>{
    const {loadRenderer,metricsMeasurer}=await import('/dist/package-web/package/full.js');await loadRenderer();
    const wasm=await import('/dist/package-web/package/pkg/pele.js');
    const {decodeGraphBinary,encodeGraphBinary}=await import('/dist/package-web/package/graph-binary.js');
    const parsed=wasm.parse_v2('sankey\nA,B,5','{}'),document=parsed.to_public();parsed.free();
    const envelope=json=>JSON.parse(json),read=()=>decodeGraphBinary(document.snapshot_binary());
    try{
      const first=read(),graph=first.graph,root=graph.nodes[graph.root.value];
      const intern=units=>{graph.strings.push(units);return graph.strings.length-1;};
      const key=intern([101,120,116,114,97]),raw=intern([0xd800,65,0xdc00]),date=graph.nodes.length;
      graph.nodes.push({kind:'date',milliseconds:'7ff8000000000042',properties:[]});
      const object=graph.nodes.length,next=intern([110,101,120,116]);
      graph.nodes.push({kind:'object',properties:[{key:next,flags:7,value:{kind:'ref',value:object}},{key:raw,flags:5,value:{kind:'ref',value:date}}]});
      root.properties.push({key,flags:7,value:{kind:'ref',value:object}});
      const upload=encodeGraphBinary(first.revision,graph);
      const malformed=envelope(document.apply_binary(upload.subarray(0,20)));
      const before=read().revision,applied=envelope(document.apply_binary(upload)),stale=envelope(document.apply_binary(upload));
      const after=read(),result=decodeGraphBinary(document.render_operation_binary(after.revision,'{}',metricsMeasurer.width,undefined));
      const rendered=result.metadata.Ok,failed=decodeGraphBinary(document.render_operation_binary(result.revision,'{"outputLimit":1}',metricsMeasurer.width,undefined));
      return{type:first.metadata.type,before,applied:applied.result,stale:stale.error.message,malformed:malformed.error.message,revision:result.revision,raw:after.graph.strings[raw],cycle:after.graph.nodes[object].properties[0].value.value===object,date:after.graph.nodes[date].milliseconds,rendered:rendered.type,failed:failed.metadata.Err.value.code};
    }finally{document.free();}
  });
  expect(report).toEqual({type:'sankey',before:'0',applied:'1',stale:'Stale public model revision',malformed:'Invalid public binary graph transport',revision:'2',raw:[0xd800,65,0xdc00],cycle:true,date:'7ff8000000000042',rendered:'sankey',failed:'limit'});
});
