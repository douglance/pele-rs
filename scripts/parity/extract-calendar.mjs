// Dev-only public scheduling oracle. Rust consumers receive no JS implementation.
import {writeFile} from 'node:fs/promises';
import {render} from 'pele';
const options={config:{},now:Date.UTC(2024,0,15,12),fontSize:15,padding:8,maxWidth:700};
const measurer={width:(text,size,style)=>text.length*size*(style&1?.71:.6)};
const cases=[];
const sources=[
 ['spring-gap','gantt\ntitle Spring gap\ndateFormat YYYY-MM-DD HH:mm\naxisFormat %Y-%m-%d %H:%M %Z\nA :a,2024-03-10 01:30,1d\nB :b,2024-03-10 01:30,24h\nGap :g,2024-03-10 02:30,1h'],
 ['fall-fold','gantt\ntitle Fall fold\ndateFormat YYYY-MM-DD HH:mm\naxisFormat %Y-%m-%d %H:%M %Z\nA :a,2024-11-03 01:30,1d\nB :b,2024-11-03 01:30,24h\nC :c,after a,1d'],
 ['half-hour-gap','gantt\ndateFormat YYYY-MM-DD HH:mm\naxisFormat %Y-%m-%d %H:%M %Z\nA :a,2024-10-06 01:45,1d\nGap :b,2024-10-06 02:15,1h'],
 ['half-hour-fold','gantt\ndateFormat YYYY-MM-DD HH:mm\naxisFormat %Y-%m-%d %H:%M %Z\nA :a,2024-04-07 01:45,1d\nB :b,2024-04-07 01:45,24h'],
 ['excluded-weekend','gantt\ndateFormat YYYY-MM-DD\naxisFormat %Y-%m-%d %H:%M %Z\nexcludes weekends\nA :a,2024-03-08,5d\nB :b,after a,2d'],
 ['month-leap','gantt\ndateFormat YYYY-MM-DD HH:mm\naxisFormat %Y-%m-%d %H:%M %Z\nA :a,2024-01-31 09:15,1M\nB :b,2024-02-29 09:15,1y'],
 ['skip-day','gantt\ndateFormat YYYY-MM-DD HH:mm\naxisFormat %Y-%m-%d %H:%M %Z\nA :a,2011-12-29 10:15,1d\nGap :g,2011-12-30 10:15,1h'],
 ['historic-year','gantt\ndateFormat YYYY-MM-DD HH:mm\naxisFormat %Y-%m-%d %H:%M %Z\nA :a,0099-03-01 01:30,1d']
];
for(const zone of ['UTC','America/New_York','Asia/Kolkata','Australia/Lord_Howe','Pacific/Apia']){
 process.env.TZ=zone;
 for(const[id,source]of sources){let expected;try{expected={result:render(source,{...options,measurer})}}catch(e){expected={error:{name:e.name,message:e.message,code:e.code,type:e.type,line:e.line,column:e.column,snippet:e.snippet}}}cases.push({id:`${zone}/${id}`,zone,source,options,expected});}
}
await writeFile('fixtures/pele/calendar.json',JSON.stringify({upstream:'pele@1.6.0',host:{node:process.version,v8:process.versions.v8},cases},null,2)+'\n');
console.log(`${cases.length} public Gantt calendar scenarios`);
