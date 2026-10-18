import pie from '../../bindings/web/diagrams/pie.js';
import gantt from '../../bindings/web/diagrams/gantt.js';
import flowchart from '../../bindings/web/diagrams/flowchart.js';
import {loadDiagramEntries,disposeDiagramModel} from '../../bindings/web/diagram-entries.js';
import {loadRegistry} from '../../bindings/web/registry.js';
async function feature(){
 await loadDiagramEntries();const registry=await loadRegistry();registry.register(pie,gantt,flowchart);
 const chart=pie.parse('pie\n"A":1\n',{},undefined,0,{maxEdges:5000});chart.sections.set('B',2);const drawing=pie.render(chart,{}, {outputLimit:1});
 const schedule=gantt.parse('gantt\nA:a,1d\n',{},undefined,0,{maxEdges:5000});schedule.getTasks(new Date());
 const graph=flowchart.parse('flowchart TD\nA-->B\n',{},'Prepared',7,{maxEdges:2.5});graph.addSingleLink('B','A',undefined);flowchart.render(graph,{flowchart:{useMaxWidth:false}}, {fontSize:13.25,maxWidth:300});
 disposeDiagramModel(chart);disposeDiagramModel(schedule);disposeDiagramModel(graph);registry.close();return drawing.svg;
}
void feature;
