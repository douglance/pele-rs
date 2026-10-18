import {loadRenderer, PeleError, PeleCompatibilityError, type PeleErrorCode, type DiagramModel} from '../../bindings/web/full.js';

// A consumer slice typechecks the installed entry's model narrowing, mutable
// collections and method signatures; no private codec types are imported.
async function consume(source: string): Promise<string> {
  const renderer = await loadRenderer();
  const model: DiagramModel = renderer.parse(source);
  if (model.type === 'pie') model.sections.set('Swift', 2);
  if (model.type === 'sankey') model.addLink(model.findOrCreateNode('A'),model.findOrCreateNode('B'),2);
  if (model.type === 'treemap') model.classes.set('active', ['fill:red']);
  if (model.type === 'quadrantChart') {
    model.classes.set('active', {radius: 7});
    model.setQuadrant1Text({text:'Top',type:'text'});
    model.addPoint({text:'Added',type:'text'},'active','0.4','0.7',[]);
  }
  if (model.type === 'journey') {
    model.addSection('Core');model.addTask('Build', ':5: Swift');
    const actors: string[] = model.getActors();
    model.setDiagramTitle(actors.join(', '));
  }
  if (model.type === 'xychart') {
    model.setXAxisBand([{text:'Swift',type:'text'}]);
    model.setBarData({text:'Builds',type:'text'},[{value:2,label:''}]);
    const data: [string,number|undefined][] = model.plotData([2]);
    model.setAccTitle(String(data.length));
  }
  if (model.type === 'kanban') {model.addNode(2,'task','Build',0);model.decorateNode({class:'active'});model.getType('[',']');}
  if (model.type === 'mindmap') {model.addNode(2,'child','Swift',0);model.decorateNode({icon:'code'});}
  if (model.type === 'timeline') {const common=model.getCommonDb();common.addSection('Core');common.addTask('2026',1,'Swift');common.addEvent('Web');}
  if (model.type === 'flowchart') {model.addVertex('C',{text:'Core',type:'text'});model.addLink(['A'],['C'],{type:'arrow_point',stroke:'normal'});model.members.add('A');}
  if (model.type === 'er') {model.addEntity('TEAM');model.addAttributes('TEAM',[{type:'int',name:'id'}]);model.claimed.add('TEAM');}
  if (model.type === 'architecture') {model.addService({id:'web',title:'Web'});model.addLayoutHint({direction:'row',members:['web']});model.nodes.set('web',{id:'web',type:'service'});}
  const rendered = renderer.render(model);
  renderer.dispose(model);
  return rendered.svg;
}
void consume;

const code:PeleErrorCode='semantic';
const error:PeleCompatibilityError=new PeleError('Native',code,{type:'flowchart',line:2,column:3,snippet:'A'});
void error;
