// Pinned Pele 1.6.0 / 458322a5. Run with TZ=UTC; no browser required.
import {parse,render} from 'pele';
import {writeFileSync} from 'node:fs';
const source='gantt\ndateFormat YYYY-MM-DD\nsection S\nA:a,2024-01-01,1d\nB:b,after a,2d';
const cases=[];
for(const mode of ['wrapped','alternate','capturedFlags']){
 const model=parse(source), original=model.getTasks, calls=[]; let aliases;
 model.getTasks=function(now){calls.push(['host',now]);const tasks=original.call(this,now);aliases=tasks[0].endTime===tasks[1].startTime;tasks[0].task='Changed live';if(mode==='capturedFlags'){this.displayMode='compact';this.topAxis=true;}return mode==='alternate'?[tasks[0]]:tasks;};
 const drawing=render(model,{now:1704067200000,measurer:{width(text,size,style){calls.push(['width',text,size,style]);return text.length*size*0.6;}}});
 cases.push({mode,drawing,calls,aliases});
}
writeFileSync(new URL('../fixtures/public/gantt-host-functions.json', import.meta.url),JSON.stringify({upstream:'458322a5b8abb1ae57ddf127c32017ab9fb14a92',package:'Pele1.6.0',source,cases}));
console.log(JSON.stringify({cases:cases.length,aliasChecks:cases.every(c=>c.aliases),bytes:JSON.stringify(cases).length}));
