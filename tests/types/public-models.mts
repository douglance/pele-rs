import {loadRenderer, type DiagramModel} from '../../bindings/web/full.js';

// A consumer slice typechecks the installed entry's model narrowing, mutable
// collections and method signatures; no private codec types are imported.
async function consume(source: string): Promise<string> {
  const renderer = await loadRenderer();
  const model: DiagramModel = renderer.parse(source);
  if (model.type === 'pie') model.sections.set('Swift', 2);
  if (model.type === 'sankey') model.lookup.get('A');
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
  const rendered = renderer.render(model);
  renderer.dispose(model);
  return rendered.svg;
}
void consume;
