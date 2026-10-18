// Development-only pinned Pele low-level entry oracle. No JS parser/renderer ships.
import {readFile,writeFile} from 'node:fs/promises';
import {gunzipSync} from 'node:zlib';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {resolve} from 'node:path';
import assert from 'node:assert/strict';
process.env.TZ='UTC';
const args=process.argv.slice(2),arg=(key,fallback)=>{const i=args.indexOf(key);return i<0?fallback:args[i+1];};
const repo=resolve(arg('--repo',fileURLToPath(new URL('../../',import.meta.url)))),upstream=resolve(arg('--upstream','/tmp/pele-upstream-458322a5'));
const output=resolve(arg('--out',repo+'/fixtures/pele/diagram-model-construction.json'));
const revision='458322a5b8abb1ae57ddf127c32017ab9fb14a92',sha=b=>createHash('sha256').update(b).digest('hex');
const env={PATH:process.env.PATH??'/usr/bin:/bin',TZ:'UTC',LANG:'C',LC_ALL:'C'};
assert.equal(execFileSync('git',['rev-parse','HEAD'],{cwd:upstream,env,encoding:'utf8'}).trim(),revision);
execFileSync('git',['diff','--quiet',revision,'--','src'],{cwd:upstream,env});
const dist=await readFile(repo+'/node_modules/pele/dist/pele.js');assert.equal(sha(dist),'d1ebc1889f3069d7759fe5db34c4ad4b4823285edbee2b537d0ef3d505a76d8d');
const numericPath=repo+'/fixtures/pele/all-render-numbers.json';let numeric;try{numeric=await readFile(numericPath);}catch{numeric=gunzipSync(await readFile(numericPath+'.gz'));}
const baselines=JSON.parse(numeric).cases.filter(c=>c.id.startsWith('baseline/'));
const bits=n=>{const b=Buffer.alloc(8);b.writeDoubleBE(n);return b.toString('hex');},units=s=>Array.from({length:s.length},(_,i)=>s.charCodeAt(i));
function graph(root){const ids=new Map(),queue=[],nodes=[],functions=[],prototypes=new Map(),builtin=new Set([Object.prototype,Array.prototype,Map.prototype,Set.prototype,Date.prototype,null]);
 const atom=v=>{if(v===undefined)return['undefined'];if(v===null)return['null'];if(typeof v==='boolean')return['bool',v];if(typeof v==='number')return['number',bits(v)];if(typeof v==='string')return['string',units(v)];if(typeof v==='function'){if(Object.hasOwn(v,'prototype'))void v.prototype;let i=functions.findIndex(f=>f.value===v);if(i<0){i=functions.length;functions.push({value:v,arity:v.length,ownKeys:Reflect.ownKeys(v)});}return['functionObservation',i];}if(!ids.has(v)){ids.set(v,queue.length);queue.push(v);}return['ref',ids.get(v)];};
 const r=atom(root);for(let id=0;id<queue.length;id++){const value=queue[id],proto=Object.getPrototypeOf(value),kind=Array.isArray(value)?'array':value instanceof Map?'map':value instanceof Set?'set':value instanceof Date?'date':builtin.has(proto)?'object':'instance',node={kind};if(kind==='instance'){if(!prototypes.has(proto))prototypes.set(proto,prototypes.size+1);node.prototype=prototypes.get(proto);}if(kind==='array')node.length=value.length;if(kind==='map')node.entries=[...value].map(([k,v])=>[atom(k),atom(v)]);if(kind==='set')node.values=[...value].map(atom);if(kind==='date')node.milliseconds=bits(+value);node.properties=Object.getOwnPropertyNames(value).filter(k=>!(kind==='array'&&k==='length')).map(k=>{const d=Object.getOwnPropertyDescriptor(value,k);assert.ok('value'in d);return[units(k),atom(d.value),(d.enumerable?1:0)|(d.writable?2:0)|(d.configurable?4:0)];});nodes.push(node);}return{root:r,nodes,...(functions.length?{functionObservations:functions.map(({arity,ownKeys})=>({arity,ownKeys}))}:{})};}
const declared={
  "FlowchartModel": [
    {
      "name": "type",
      "callable": false,
      "private": false
    },
    {
      "name": "direction",
      "callable": false,
      "private": false
    },
    {
      "name": "nodes",
      "callable": false,
      "private": false
    },
    {
      "name": "edges",
      "callable": false,
      "private": false
    },
    {
      "name": "subgraphs",
      "callable": false,
      "private": false
    },
    {
      "name": "classes",
      "callable": false,
      "private": false
    },
    {
      "name": "tooltips",
      "callable": false,
      "private": false
    },
    {
      "name": "defaultEdgeStyle",
      "callable": false,
      "private": false
    },
    {
      "name": "defaultInterpolate",
      "callable": false,
      "private": false
    },
    {
      "name": "title",
      "callable": false,
      "private": false
    },
    {
      "name": "accTitle",
      "callable": false,
      "private": false
    },
    {
      "name": "accDescr",
      "callable": false,
      "private": false
    }
  ],
  "PieModel": [
    {
      "name": "type",
      "callable": false,
      "private": false
    },
    {
      "name": "title",
      "callable": false,
      "private": false
    },
    {
      "name": "accTitle",
      "callable": false,
      "private": false
    },
    {
      "name": "accDescr",
      "callable": false,
      "private": false
    },
    {
      "name": "showData",
      "callable": false,
      "private": false
    },
    {
      "name": "sections",
      "callable": false,
      "private": false
    }
  ],
  "ErModel": [
    {
      "name": "type",
      "callable": false,
      "private": false
    },
    {
      "name": "direction",
      "callable": false,
      "private": false
    },
    {
      "name": "entities",
      "callable": false,
      "private": false
    },
    {
      "name": "relationships",
      "callable": false,
      "private": false
    },
    {
      "name": "classes",
      "callable": false,
      "private": false
    },
    {
      "name": "subgraphs",
      "callable": false,
      "private": false
    },
    {
      "name": "title",
      "callable": false,
      "private": false
    },
    {
      "name": "accTitle",
      "callable": false,
      "private": false
    },
    {
      "name": "accDescr",
      "callable": false,
      "private": false
    }
  ],
  "MindmapModel": [
    {
      "name": "type",
      "callable": false,
      "private": false
    },
    {
      "name": "title",
      "callable": false,
      "private": false
    },
    {
      "name": "nodes",
      "callable": false,
      "private": false
    }
  ],
  "KanbanModel": [
    {
      "name": "type",
      "callable": false,
      "private": false
    },
    {
      "name": "title",
      "callable": false,
      "private": false
    },
    {
      "name": "sections",
      "callable": false,
      "private": false
    }
  ],
  "TimelineModel": [
    {
      "name": "type",
      "callable": false,
      "private": false
    },
    {
      "name": "title",
      "callable": false,
      "private": false
    },
    {
      "name": "accTitle",
      "callable": false,
      "private": false
    },
    {
      "name": "accDescr",
      "callable": false,
      "private": false
    },
    {
      "name": "direction",
      "callable": false,
      "private": false
    },
    {
      "name": "sections",
      "callable": false,
      "private": false
    },
    {
      "name": "periods",
      "callable": false,
      "private": false
    }
  ],
  "JourneyModel": [
    {
      "name": "type",
      "callable": false,
      "private": false
    },
    {
      "name": "title",
      "callable": false,
      "private": false
    },
    {
      "name": "accTitle",
      "callable": false,
      "private": false
    },
    {
      "name": "accDescr",
      "callable": false,
      "private": false
    },
    {
      "name": "sections",
      "callable": false,
      "private": false
    },
    {
      "name": "tasks",
      "callable": false,
      "private": false
    }
  ],
  "ClassModel": [
    {
      "name": "type",
      "callable": false,
      "private": false
    },
    {
      "name": "direction",
      "callable": false,
      "private": false
    },
    {
      "name": "classes",
      "callable": false,
      "private": false
    },
    {
      "name": "relations",
      "callable": false,
      "private": false
    },
    {
      "name": "notes",
      "callable": false,
      "private": false
    },
    {
      "name": "namespaces",
      "callable": false,
      "private": false
    },
    {
      "name": "interfaces",
      "callable": false,
      "private": false
    },
    {
      "name": "styleClasses",
      "callable": false,
      "private": false
    },
    {
      "name": "title",
      "callable": false,
      "private": false
    },
    {
      "name": "accTitle",
      "callable": false,
      "private": false
    },
    {
      "name": "accDescr",
      "callable": false,
      "private": false
    }
  ],
  "QuadrantModel": [
    {
      "name": "type",
      "callable": false,
      "private": false
    },
    {
      "name": "title",
      "callable": false,
      "private": false
    },
    {
      "name": "accTitle",
      "callable": false,
      "private": false
    },
    {
      "name": "accDescr",
      "callable": false,
      "private": false
    },
    {
      "name": "quadrants",
      "callable": false,
      "private": false
    },
    {
      "name": "xAxisLeft",
      "callable": false,
      "private": false
    },
    {
      "name": "xAxisRight",
      "callable": false,
      "private": false
    },
    {
      "name": "yAxisBottom",
      "callable": false,
      "private": false
    },
    {
      "name": "yAxisTop",
      "callable": false,
      "private": false
    },
    {
      "name": "points",
      "callable": false,
      "private": false
    },
    {
      "name": "classes",
      "callable": false,
      "private": false
    }
  ],
  "XyChartModel": [
    {
      "name": "type",
      "callable": false,
      "private": false
    },
    {
      "name": "title",
      "callable": false,
      "private": false
    },
    {
      "name": "accTitle",
      "callable": false,
      "private": false
    },
    {
      "name": "accDescr",
      "callable": false,
      "private": false
    },
    {
      "name": "orientation",
      "callable": false,
      "private": false
    },
    {
      "name": "xAxis",
      "callable": false,
      "private": false
    },
    {
      "name": "yAxis",
      "callable": false,
      "private": false
    },
    {
      "name": "hasSetYAxis",
      "callable": false,
      "private": false
    },
    {
      "name": "plots",
      "callable": false,
      "private": false
    }
  ],
  "StateModel": [
    {
      "name": "type",
      "callable": false,
      "private": false
    },
    {
      "name": "direction",
      "callable": false,
      "private": false
    },
    {
      "name": "rootDoc",
      "callable": false,
      "private": false
    },
    {
      "name": "states",
      "callable": false,
      "private": false
    },
    {
      "name": "relations",
      "callable": false,
      "private": false
    },
    {
      "name": "classes",
      "callable": false,
      "private": false
    },
    {
      "name": "links",
      "callable": false,
      "private": false
    },
    {
      "name": "title",
      "callable": false,
      "private": false
    },
    {
      "name": "accTitle",
      "callable": false,
      "private": false
    },
    {
      "name": "accDescr",
      "callable": false,
      "private": false
    }
  ],
  "GanttModel": [
    {
      "name": "type",
      "callable": false,
      "private": false
    },
    {
      "name": "title",
      "callable": false,
      "private": false
    },
    {
      "name": "accTitle",
      "callable": false,
      "private": false
    },
    {
      "name": "accDescr",
      "callable": false,
      "private": false
    },
    {
      "name": "dateFormat",
      "callable": false,
      "private": false
    },
    {
      "name": "axisFormat",
      "callable": false,
      "private": false
    },
    {
      "name": "tickInterval",
      "callable": false,
      "private": false
    },
    {
      "name": "todayMarker",
      "callable": false,
      "private": false
    },
    {
      "name": "includes",
      "callable": false,
      "private": false
    },
    {
      "name": "excludes",
      "callable": false,
      "private": false
    },
    {
      "name": "inclusiveEndDates",
      "callable": false,
      "private": false
    },
    {
      "name": "topAxis",
      "callable": false,
      "private": false
    },
    {
      "name": "displayMode",
      "callable": false,
      "private": false
    },
    {
      "name": "weekday",
      "callable": false,
      "private": false
    },
    {
      "name": "weekend",
      "callable": false,
      "private": false
    },
    {
      "name": "sections",
      "callable": false,
      "private": false
    },
    {
      "name": "links",
      "callable": false,
      "private": false
    },
    {
      "name": "tasks",
      "callable": false,
      "private": false
    },
    {
      "name": "getTasks",
      "callable": true,
      "private": false
    }
  ],
  "ArchitectureModel": [
    {
      "name": "type",
      "callable": false,
      "private": false
    },
    {
      "name": "title",
      "callable": false,
      "private": false
    },
    {
      "name": "accTitle",
      "callable": false,
      "private": false
    },
    {
      "name": "accDescr",
      "callable": false,
      "private": false
    },
    {
      "name": "nodes",
      "callable": false,
      "private": false
    },
    {
      "name": "groups",
      "callable": false,
      "private": false
    },
    {
      "name": "edges",
      "callable": false,
      "private": false
    },
    {
      "name": "layoutHints",
      "callable": false,
      "private": false
    }
  ],
  "RequirementModel": [
    {
      "name": "type",
      "callable": false,
      "private": false
    },
    {
      "name": "direction",
      "callable": false,
      "private": false
    },
    {
      "name": "requirements",
      "callable": false,
      "private": false
    },
    {
      "name": "elements",
      "callable": false,
      "private": false
    },
    {
      "name": "relations",
      "callable": false,
      "private": false
    },
    {
      "name": "classes",
      "callable": false,
      "private": false
    },
    {
      "name": "title",
      "callable": false,
      "private": false
    },
    {
      "name": "accTitle",
      "callable": false,
      "private": false
    },
    {
      "name": "accDescr",
      "callable": false,
      "private": false
    }
  ],
  "SankeyModel": [
    {
      "name": "type",
      "callable": false,
      "private": false
    },
    {
      "name": "nodes",
      "callable": false,
      "private": false
    },
    {
      "name": "links",
      "callable": false,
      "private": false
    },
    {
      "name": "title",
      "callable": false,
      "private": false
    },
    {
      "name": "accTitle",
      "callable": false,
      "private": false
    },
    {
      "name": "accDescr",
      "callable": false,
      "private": false
    }
  ],
  "PacketModel": [
    {
      "name": "type",
      "callable": false,
      "private": false
    },
    {
      "name": "title",
      "callable": false,
      "private": false
    },
    {
      "name": "accTitle",
      "callable": false,
      "private": false
    },
    {
      "name": "accDescr",
      "callable": false,
      "private": false
    },
    {
      "name": "bitsPerRow",
      "callable": false,
      "private": false
    },
    {
      "name": "rows",
      "callable": false,
      "private": false
    }
  ],
  "RadarModel": [
    {
      "name": "type",
      "callable": false,
      "private": false
    },
    {
      "name": "title",
      "callable": false,
      "private": false
    },
    {
      "name": "accTitle",
      "callable": false,
      "private": false
    },
    {
      "name": "accDescr",
      "callable": false,
      "private": false
    },
    {
      "name": "axes",
      "callable": false,
      "private": false
    },
    {
      "name": "curves",
      "callable": false,
      "private": false
    },
    {
      "name": "options",
      "callable": false,
      "private": false
    }
  ],
  "TreemapModel": [
    {
      "name": "type",
      "callable": false,
      "private": false
    },
    {
      "name": "title",
      "callable": false,
      "private": false
    },
    {
      "name": "accTitle",
      "callable": false,
      "private": false
    },
    {
      "name": "accDescr",
      "callable": false,
      "private": false
    },
    {
      "name": "roots",
      "callable": false,
      "private": false
    },
    {
      "name": "classes",
      "callable": false,
      "private": false
    }
  ],
  "InfoModel": [
    {
      "name": "type",
      "callable": false,
      "private": false
    },
    {
      "name": "title",
      "callable": false,
      "private": false
    },
    {
      "name": "accTitle",
      "callable": false,
      "private": false
    },
    {
      "name": "accDescr",
      "callable": false,
      "private": false
    }
  ],
  "SequenceModel": [
    {
      "name": "type",
      "callable": false,
      "private": false
    },
    {
      "name": "actors",
      "callable": false,
      "private": false
    },
    {
      "name": "createdActors",
      "callable": false,
      "private": false
    },
    {
      "name": "destroyedActors",
      "callable": false,
      "private": false
    },
    {
      "name": "boxes",
      "callable": false,
      "private": false
    },
    {
      "name": "messages",
      "callable": false,
      "private": false
    },
    {
      "name": "notes",
      "callable": false,
      "private": false
    },
    {
      "name": "title",
      "callable": false,
      "private": false
    },
    {
      "name": "accTitle",
      "callable": false,
      "private": false
    },
    {
      "name": "accDescr",
      "callable": false,
      "private": false
    }
  ],
  "GitModel": [
    {
      "name": "type",
      "callable": false,
      "private": false
    },
    {
      "name": "title",
      "callable": false,
      "private": false
    },
    {
      "name": "accTitle",
      "callable": false,
      "private": false
    },
    {
      "name": "accDescr",
      "callable": false,
      "private": false
    },
    {
      "name": "direction",
      "callable": false,
      "private": false
    },
    {
      "name": "commits",
      "callable": false,
      "private": false
    },
    {
      "name": "branches",
      "callable": false,
      "private": false
    },
    {
      "name": "lanes",
      "callable": false,
      "private": false
    },
    {
      "name": "currentBranch",
      "callable": false,
      "private": false
    },
    {
      "name": "head",
      "callable": false,
      "private": false
    },
    {
      "name": "warnings",
      "callable": false,
      "private": false
    }
  ],
  "C4Model": [
    {
      "name": "type",
      "callable": false,
      "private": false
    },
    {
      "name": "c4Type",
      "callable": false,
      "private": false
    },
    {
      "name": "title",
      "callable": false,
      "private": false
    },
    {
      "name": "accTitle",
      "callable": false,
      "private": false
    },
    {
      "name": "accDescr",
      "callable": false,
      "private": false
    },
    {
      "name": "shapes",
      "callable": false,
      "private": false
    },
    {
      "name": "boundaries",
      "callable": false,
      "private": false
    },
    {
      "name": "rels",
      "callable": false,
      "private": false
    },
    {
      "name": "shapeInRow",
      "callable": false,
      "private": false
    },
    {
      "name": "boundaryInRow",
      "callable": false,
      "private": false
    }
  ],
  "BlockModel": [
    {
      "name": "type",
      "callable": false,
      "private": false
    },
    {
      "name": "root",
      "callable": false,
      "private": false
    },
    {
      "name": "blocks",
      "callable": false,
      "private": false
    },
    {
      "name": "edges",
      "callable": false,
      "private": false
    },
    {
      "name": "classes",
      "callable": false,
      "private": false
    },
    {
      "name": "byId",
      "callable": false,
      "private": false
    },
    {
      "name": "title",
      "callable": false,
      "private": false
    },
    {
      "name": "accTitle",
      "callable": false,
      "private": false
    },
    {
      "name": "accDescr",
      "callable": false,
      "private": false
    }
  ],
  "RailroadModel": [
    {
      "name": "type",
      "callable": false,
      "private": false
    },
    {
      "name": "notation",
      "callable": false,
      "private": false
    },
    {
      "name": "title",
      "callable": false,
      "private": false
    },
    {
      "name": "accTitle",
      "callable": false,
      "private": false
    },
    {
      "name": "accDescr",
      "callable": false,
      "private": false
    },
    {
      "name": "rules",
      "callable": false,
      "private": false
    }
  ],
  "AgentflowModel": [
    {
      "name": "type",
      "callable": false,
      "private": false
    },
    {
      "name": "direction",
      "callable": false,
      "private": false
    },
    {
      "name": "nodes",
      "callable": false,
      "private": false
    },
    {
      "name": "edges",
      "callable": false,
      "private": false
    },
    {
      "name": "subgraphs",
      "callable": false,
      "private": false
    },
    {
      "name": "connectors",
      "callable": false,
      "private": false
    },
    {
      "name": "classes",
      "callable": false,
      "private": false
    },
    {
      "name": "tooltips",
      "callable": false,
      "private": false
    },
    {
      "name": "defaultEdgeStyle",
      "callable": false,
      "private": false
    },
    {
      "name": "defaultInterpolate",
      "callable": false,
      "private": false
    },
    {
      "name": "mappings",
      "callable": false,
      "private": false
    },
    {
      "name": "diagnostics",
      "callable": false,
      "private": false
    },
    {
      "name": "title",
      "callable": false,
      "private": false
    },
    {
      "name": "accTitle",
      "callable": false,
      "private": false
    },
    {
      "name": "accDescr",
      "callable": false,
      "private": false
    },
    {
      "name": "semanticModel",
      "callable": true,
      "private": false
    }
  ],
  "TreeViewModel": [
    {
      "name": "type",
      "callable": false,
      "private": false
    },
    {
      "name": "title",
      "callable": false,
      "private": false
    },
    {
      "name": "accTitle",
      "callable": false,
      "private": false
    },
    {
      "name": "accDescr",
      "callable": false,
      "private": false
    },
    {
      "name": "root",
      "callable": false,
      "private": false
    },
    {
      "name": "count",
      "callable": false,
      "private": false
    }
  ],
  "CynefinModel": [
    {
      "name": "type",
      "callable": false,
      "private": false
    },
    {
      "name": "title",
      "callable": false,
      "private": false
    },
    {
      "name": "accTitle",
      "callable": false,
      "private": false
    },
    {
      "name": "accDescr",
      "callable": false,
      "private": false
    },
    {
      "name": "domains",
      "callable": false,
      "private": false
    },
    {
      "name": "transitions",
      "callable": false,
      "private": false
    }
  ],
  "WardleyModel": [
    {
      "name": "type",
      "callable": false,
      "private": false
    },
    {
      "name": "title",
      "callable": false,
      "private": false
    },
    {
      "name": "accTitle",
      "callable": false,
      "private": false
    },
    {
      "name": "accDescr",
      "callable": false,
      "private": false
    },
    {
      "name": "nodes",
      "callable": false,
      "private": false
    },
    {
      "name": "links",
      "callable": false,
      "private": false
    },
    {
      "name": "trends",
      "callable": false,
      "private": false
    },
    {
      "name": "pipelines",
      "callable": false,
      "private": false
    },
    {
      "name": "annotations",
      "callable": false,
      "private": false
    },
    {
      "name": "notes",
      "callable": false,
      "private": false
    },
    {
      "name": "accelerators",
      "callable": false,
      "private": false
    },
    {
      "name": "deaccelerators",
      "callable": false,
      "private": false
    },
    {
      "name": "annotationsBox",
      "callable": false,
      "private": false
    },
    {
      "name": "axes",
      "callable": false,
      "private": false
    },
    {
      "name": "size",
      "callable": false,
      "private": false
    }
  ],
  "EventModelingModel": [
    {
      "name": "type",
      "callable": false,
      "private": false
    },
    {
      "name": "title",
      "callable": false,
      "private": false
    },
    {
      "name": "accTitle",
      "callable": false,
      "private": false
    },
    {
      "name": "accDescr",
      "callable": false,
      "private": false
    },
    {
      "name": "boxes",
      "callable": false,
      "private": false
    },
    {
      "name": "lanes",
      "callable": false,
      "private": false
    },
    {
      "name": "relations",
      "callable": false,
      "private": false
    },
    {
      "name": "notes",
      "callable": false,
      "private": false
    },
    {
      "name": "specifications",
      "callable": false,
      "private": false
    },
    {
      "name": "entities",
      "callable": false,
      "private": false
    }
  ],
  "UsecaseModel": [
    {
      "name": "type",
      "callable": false,
      "private": false
    },
    {
      "name": "actors",
      "callable": false,
      "private": false
    },
    {
      "name": "useCases",
      "callable": false,
      "private": false
    },
    {
      "name": "systemBoundaries",
      "callable": false,
      "private": false
    },
    {
      "name": "relationships",
      "callable": false,
      "private": false
    },
    {
      "name": "notes",
      "callable": false,
      "private": false
    },
    {
      "name": "jsonNodes",
      "callable": false,
      "private": false
    },
    {
      "name": "classDefs",
      "callable": false,
      "private": false
    },
    {
      "name": "direction",
      "callable": false,
      "private": false
    },
    {
      "name": "title",
      "callable": false,
      "private": false
    },
    {
      "name": "accTitle",
      "callable": false,
      "private": false
    },
    {
      "name": "accDescr",
      "callable": false,
      "private": false
    },
    {
      "name": "accDescription",
      "callable": false,
      "private": false
    },
    {
      "name": "source",
      "callable": false,
      "private": false
    },
    {
      "name": "headerSpan",
      "callable": false,
      "private": false
    },
    {
      "name": "statements",
      "callable": false,
      "private": false
    }
  ],
  "VennModel": [
    {
      "name": "type",
      "callable": false,
      "private": false
    },
    {
      "name": "title",
      "callable": false,
      "private": false
    },
    {
      "name": "subsets",
      "callable": false,
      "private": false
    },
    {
      "name": "textNodes",
      "callable": false,
      "private": false
    },
    {
      "name": "styles",
      "callable": false,
      "private": false
    }
  ],
  "IshikawaModel": [
    {
      "name": "type",
      "callable": false,
      "private": false
    },
    {
      "name": "title",
      "callable": false,
      "private": false
    },
    {
      "name": "root",
      "callable": false,
      "private": false
    }
  ],
  "SwimlaneModel": [
    {
      "name": "type",
      "callable": false,
      "private": false
    },
    {
      "name": "direction",
      "callable": false,
      "private": false
    },
    {
      "name": "nodes",
      "callable": false,
      "private": false
    },
    {
      "name": "edges",
      "callable": false,
      "private": false
    },
    {
      "name": "subgraphs",
      "callable": false,
      "private": false
    },
    {
      "name": "classes",
      "callable": false,
      "private": false
    },
    {
      "name": "tooltips",
      "callable": false,
      "private": false
    },
    {
      "name": "defaultEdgeStyle",
      "callable": false,
      "private": false
    },
    {
      "name": "defaultInterpolate",
      "callable": false,
      "private": false
    },
    {
      "name": "title",
      "callable": false,
      "private": false
    },
    {
      "name": "accTitle",
      "callable": false,
      "private": false
    },
    {
      "name": "accDescr",
      "callable": false,
      "private": false
    }
  ]
};
const typeNames=['FlowchartModel','PieModel','ErModel','MindmapModel','KanbanModel','TimelineModel','JourneyModel','ClassModel','QuadrantModel','XyChartModel','StateModel','GanttModel','ArchitectureModel','RequirementModel','SankeyModel','PacketModel','RadarModel','TreemapModel','InfoModel','SequenceModel','GitModel','C4Model','BlockModel','RailroadModel','AgentflowModel','TreeViewModel','CynefinModel','WardleyModel','EventModelingModel','UsecaseModel','VennModel','IshikawaModel','SwimlaneModel'];
const families=['flowchart','pie','er','mindmap','kanban','timeline','journey','class','quadrantChart','xychart','state','gantt','architecture','requirement','sankey','packet','radar','treemap','info','sequence','gitGraph','c4','block','railroad','agentflow','treeView','cynefin','wardley','eventmodeling','usecase','venn','ishikawa','swimlane'];
const fieldNames=Object.fromEntries(families.map((f,i)=>[f,declared[typeNames[i]].map(v=>v.name)]));
function cloneData(value,memo=new Map()){
 if(value===null||typeof value!=='object')return value;
 if(memo.has(value))return memo.get(value);
 const out=Array.isArray(value)?[]:value instanceof Map?new Map():value instanceof Set?new Set():value instanceof Date?new Date(+value):{};memo.set(value,out);
 if(value instanceof Map)for(const[k,v]of value)out.set(cloneData(k,memo),cloneData(v,memo));
 if(value instanceof Set)for(const v of value)out.add(cloneData(v,memo));
 for(const key of Object.getOwnPropertyNames(value)){if(Array.isArray(value)&&key==='length')continue;const d=Object.getOwnPropertyDescriptor(value,key);assert.ok('value'in d);Object.defineProperty(out,key,{...d,value:cloneData(d.value,memo)});}return out;
}
const now=1704067200000;Date.now=()=>now;const cases=[],provenance={};
for(const path of ['src/models.ts','src/diagrams/agentflow/graph.ts','src/diagrams/gantt/render.ts'])provenance[path]=sha(await readFile(upstream+'/'+path));
for(const base of baselines){
 const family=base.family,entryPath=repo+'/node_modules/pele/dist/esm/diagrams/'+family+'.js',entry=(await import(pathToFileURL(entryPath).href)).default;
 provenance['dist/esm/diagrams/'+family+'.js']=sha(await readFile(entryPath));
 let source=base.source.replace(/^---\n[\s\S]*?\n---\n/,'')+'\n';
 if(family==='class')source='classDiagram\nclass Engine\nEngine --> Document : renders\n';
 for(const variant of ['literal','edited',...(family==='gantt'?['missing-date-filter']:[])]){
  const config={},parsed=entry.parse(source,config,undefined,0,{maxEdges:undefined});if(family==='gantt')parsed.getTasks(now);
  const values={};for(const key of fieldNames[family])values[key]=parsed[key];const model=cloneData(values);
  if(family==='gantt')model.getTasks=function getTasks(_now){return this.tasks;};
  if(family==='agentflow'){const semantic=parsed.semanticModel();model.semanticModel=function semanticModel(){return semantic;};}
  const shared=[];model.callerExtra={raw:'\ud800',ieee:NaN,shared,alias:shared};model.callerExtra.self=model.callerExtra;
  if(variant==='edited')model.title='Fresh '+family;
  if(variant==='missing-date-filter')model.excludes=['weekends'];
  const before=graph(model),calls=[],options={now,outputLimit:1,limit:-1,maxEdges:-1,measurer:{width(text,size,style){if(calls.length>=2000)throw Error('Oracle callback harness budget');const width=text.length*size*(style&1?.71:.6);calls.push(['width',units(text),bits(size),style,bits(width)]);return width;}},icons:name=>{calls.push(['icon',units(name)]);return'<path d="M0,0"/>';}};
  let expected;try{const result=entry.render(model,config,options);assert.ok(result.svg.length<1_000_000);expected={before,after:graph(model),result,widthBits:bits(result.width),heightBits:bits(result.height),calls};}catch(e){expected={before,after:graph(model),error:{name:e.name,message:e.message,...(e.name==='PeleError'?{code:e.code,type:e.type,line:e.line,column:e.column,snippet:e.snippet}:{})},calls};}
  cases.push({id:family+'/'+variant,family,source,config,fields:fieldNames[family],variant,options:{now,outputLimit:1,limit:-1,maxEdges:-1},expected});
 }
}
const fixture={revision,oracleSha256:sha(dist),provenance,host:{measure:'fixedUTF16',now,timezone:'UTC'},scope:'Fresh ordinary declared-model roots with data-only nested records; Gantt getTasks is a caller method returning already scheduled literal tasks. No DB prototype, private parser history, or graph cache is injected. Undeclared Agentflow/Gantt helper reads retain pinned errors. Caller extras/aliases/cycles/UTF16 remain authoritative.',cases};
await writeFile(output,JSON.stringify(fixture,null,2)+'\n');console.log(JSON.stringify({cases:cases.length,errors:cases.filter(c=>c.expected.error).map(c=>({id:c.id,error:c.expected.error,calls:c.expected.calls.length})),out:output}));
