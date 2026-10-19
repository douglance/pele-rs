import {test, expect} from '@playwright/test';

test('mounted Rust drawings match pinned last-good, hidden, detached and callback lifecycle', async ({page}) => {
  await page.goto('/examples/web/');
  const failures = await page.evaluate(async () => {
    const {loadRenderer} = await import('/dist/package-web/package/full.js');
    const oracle = await import('/dist/evidence/pele-oracle.js');
    const engine = await loadRenderer(), failures = [];
    const measurer = {width: (text, size, style) => text.length * size * (style & 1 ? .71 : .6)};
    const FLOW = 'flowchart LR\naccTitle: Mounted steps\nA[Start]-->B[Middle]-->C[End]';
    const SANKEY = 'sankey\nA,B,10\nB,C,6';
    const bad = 'flowchart LR\nA -->';
    const hosts = [0, 1].map(() => {
      const host = document.createElement('div');
      host.style.cssText = 'width:520px;padding:10px;font-family:Arial';
      document.body.append(host); return host;
    });
    const events = [[], []];
    const options = index => ({measurer, idPrefix: 'mount-lifecycle', zoom: false,
      onRender: result => events[index].push(['render', result.svg]),
      onError: error => events[index].push(['error', error.name, error.message])});
    const handles = [engine.mount(hosts[0], FLOW, options(0)), oracle.mount(hosts[1], FLOW, options(1))];
    const error = value => value === undefined ? null : {name: value?.name, message: value?.message, code: value?.code, type: value?.type};
    const snapshot = index => ({source: handles[index].source, result: handles[index].result,
      error: error(handles[index].error), html: hosts[index].innerHTML, zoom: handles[index].zoom !== undefined});
    const check = (condition, stage) => {if (!condition) failures.push(stage);};
    const compare = stage => {
      check(JSON.stringify(snapshot(0)) === JSON.stringify(snapshot(1)), `${stage}: complete state/DOM`);
      check(JSON.stringify(events[0]) === JSON.stringify(events[1]), `${stage}: callback order`);
    };
    const refresh = () => handles.forEach(handle => handle.refresh());
    const update = (source, nextOptions) => handles.forEach((handle, i) => handle.update(source, nextOptions?.(i)));
    compare('initial');
    update(bad); compare('invalid update');
    check(hosts[0].firstElementChild.style.opacity === '0.4' && hosts[0].querySelector('.pele-error')?.title === handles[0].error.message, 'last-good dimmed with complete error tooltip');
    hosts.forEach(host => {host.style.width = '230px';}); refresh(); compare('resize while source invalid');
    check(handles[0].source === bad && handles[0].result.type === 'flowchart', 'failed source stays paired with last-good model');
    update(SANKEY); compare('valid replacement');
    check(handles[0].error === undefined && !hosts[0].querySelector('.pele-error'), 'error presentation removed');
    const beforeHidden = handles[0].result;
    hosts.forEach(host => {host.style.display = 'none';});
    update(FLOW); refresh(); compare('hidden update deferred');
    check(handles[0].result === beforeHidden && handles[0].source === FLOW, 'hidden source updates before drawing');
    hosts.forEach(host => {host.style.display = '';}); refresh(); compare('shown update');
    hosts.forEach(host => {host.style.display = 'none';});
    update('sankey\nA,B,10\nB,A,10');
    hosts.forEach(host => {host.style.display = '';}); refresh(); compare('hidden render failure');
    check(handles[0].error !== undefined && handles[0].result.type === 'flowchart', 'hidden render failure retains last-good');
    update(SANKEY); compare('hidden failure recovery');
    hosts.forEach(host => host.remove()); update(FLOW); compare('detached best-effort drawing');
    const detached = handles[0].result; refresh();
    check(handles[0].result === detached, 'detached refresh quiet');
    hosts.forEach(host => {host.style.fontFamily = 'Georgia'; document.body.append(host);});
    refresh(); compare('reattached font refresh');

    // A sanitizer fails before commit. onRender fails after commit; onError's
    // own exception propagates while error retains the original render failure.
    const sentinel = new Error('host callback sentinel');
    const caught = action => {try {action();return undefined;} catch (error) {return error;}};
    update(SANKEY, i => ({...options(i), sanitize() {throw sentinel;}})); compare('sanitizer failure');
    check(handles[0].error === sentinel, 'sanitizer exception identity retained');
    update(SANKEY, options); compare('sanitizer recovery');
    const throws = handles.map((handle, i) => caught(() => handle.update(FLOW, {...options(i), onRender() {throw sentinel;}})));
    check(throws.every(error => error === sentinel) && handles[0].result.type === 'flowchart' && handles[0].error === undefined, 'onRender commits before original exception');
    compare('onRender exception');
    const underlying = handles.map((handle, i) => caught(() => handle.update(bad, {...options(i), onError() {throw sentinel;}})));
    check(underlying.every(error => error === sentinel) && handles[0].error !== sentinel && handles[0].error?.code === 'syntax', 'onError throw preserves original parse failure');
    compare('onError exception');
    update(SANKEY, options); compare('callback recovery');
    const callbacks = events[0].length, shown = hosts[0].innerHTML;
    handles.forEach(handle => handle.destroy()); handles.forEach(handle => handle.destroy());
    hosts.forEach(host => {host.style.width = '800px';}); refresh();
    check(events[0].length === callbacks && hosts[0].innerHTML === shown, 'idempotent destroy leaves drawing and stops refresh');
    compare('destroy');
    hosts.forEach(host => host.remove());

    const hidden = document.createElement('div'); hidden.style.display = 'none'; document.body.append(hidden);
    const initiallyHidden = engine.mount(hidden, FLOW, {measurer, zoom: false});
    check(initiallyHidden.result?.type === 'flowchart' && !!hidden.querySelector('svg'), 'initial zero-width mount still draws naturally');
    initiallyHidden.destroy(); hidden.remove();
    const invalid = document.createElement('div'); invalid.style.width = '400px'; document.body.append(invalid);
    const noGood = engine.mount(invalid, bad, {measurer, zoom: false});
    check(noGood.result === undefined && invalid.children.length === 1 && invalid.firstElementChild.classList.contains('pele-error') && invalid.firstElementChild.textContent === noGood.error.message, 'initial failure displays complete error alone');
    noGood.update(SANKEY); check(noGood.result?.type === 'sankey', 'initial failure recovers');
    noGood.destroy(); invalid.remove();
    return failures;
  });
  expect(failures).toEqual([]);
});

test('mount retains native parsed models across refresh, replacement and foreign documents', async ({page}) => {
  await page.goto('/examples/web/');
  const failures = await page.evaluate(async () => {
    const {loadRenderer} = await import('/dist/package-web/package/full.js');
    const {mountWith} = await import('/dist/package-web/package/mount.js');
    const {windowOf, room} = await import('/dist/package-web/package/window.js');
    const engine = await loadRenderer(), failures = [], parsed = [], renders = [];
    const wrapped = {
      parse(source, options) {const model = engine.parse(source, options); parsed.push(model); return model;},
      render(model, options) {renders.push(model); return engine.render(model, options);},
    };
    const host = document.createElement('div'); host.style.cssText = 'width:320px;padding:11px;font-family:Arial'; document.body.append(host);
    const source = 'flowchart LR\nA[One]-->B[Two]-->C[Three]';
    const options = {zoom: false, idPrefix: 'owned-mount', measurer: {width: (text, size) => text.length * size * .6}};
    let notifications = 0;
    const handle = mountWith(wrapped, host, source, {...options, onRender() {notifications++;}});
    const first = parsed[0], original = handle.result.svg;
    host.style.width = '220px'; handle.refresh();
    host.style.width = '700px'; handle.refresh();
    if (parsed.length !== 1 || !renders.every(model => model === first)) failures.push('resize reparsed or replaced native model');
    // Ordinary native methods remain available inside the retained host model.
    first.addVertex('D', {text: 'Native retained', type: 'text'});
    host.style.fontFamily = 'Georgia'; handle.refresh();
    if (!handle.result.svg.includes('Native retained') || parsed.length !== 1) failures.push('font refresh discarded native mutation');
    const frame = document.createElement('iframe'); document.body.append(frame);
    await new Promise(resolve => {frame.onload = resolve; frame.srcdoc = '<!doctype html><body style="font-family:monospace"></body>';});
    frame.contentDocument.body.append(host); host.style.fontFamily = 'monospace'; handle.refresh();
    if (windowOf(host) !== frame.contentWindow || room(host, frame.contentWindow.getComputedStyle(host)) !== 700 || handle.result.svg === original) failures.push('foreign-document window/style/room binding');
    host.style.width = '240px'; handle.refresh();
    if (parsed.length !== 1 || !handle.result.svg.includes('Native retained')) failures.push('foreign resize lost retained model');
    const oldNotifications = notifications;
    const replacement = mountWith(wrapped, host, 'pie\n"Replacement":2', options);
    handle.refresh();
    if (notifications !== oldNotifications || replacement.result.type !== 'pie') failures.push('replacement did not stop old mount');
    replacement.destroy(); handle.destroy();
    parsed.forEach(model => engine.dispose(model)); frame.remove();
    return failures;
  });
  expect(failures).toEqual([]);
});

test('zoom controls, keyboard, modified wheel and viewport clamps match pinned DOM behavior', async ({page}) => {
  await page.goto('/examples/web/');
  const failures = await page.evaluate(async () => {
    const {loadRenderer, enableZoom} = await import('/dist/package-web/package/full.js');
    const oracle = await import('/dist/evidence/pele-oracle.js');
    const engine = await loadRenderer(), failures = [];
    const source = 'flowchart LR\nA[Start]-->B[Middle]-->C[Another]-->D[Long]-->E[Finish]';
    const rendered = engine.render(source, {idPrefix: 'zoom-test', measurer: {width: (text, size) => text.length * size * .6}});
    const hosts = [0, 1].map(() => {const host = document.createElement('div');host.style.cssText = 'width:300px;height:140px;padding:12px';host.innerHTML = rendered.svg;document.body.append(host);return host;});
    const options = {maxScale: 2.5, labels: {zoomIn: 'Enlarge', zoomOut: 'Reduce', reset: 'Fit'}};
    const zooms = [enableZoom(hosts[0], options), oracle.enableZoom(hosts[1], options)];
    const snapshot = i => ({scale: zooms[i].scale, fit: zooms[i].fit, svg: hosts[i].firstElementChild.outerHTML,
      position: hosts[i].style.position, controls: hosts[i].querySelector('.pele-zoom')?.outerHTML});
    const compare = stage => {if (JSON.stringify(snapshot(0)) !== JSON.stringify(snapshot(1))) failures.push(stage);};
    const both = action => {action(zooms[0], hosts[0]);action(zooms[1], hosts[1]);};
    const event = (host, Type, type, values) => {const bounds = host.firstElementChild.getBoundingClientRect();return host.dispatchEvent(new Type(type, {bubbles: true, cancelable: true, clientX: bounds.left + bounds.width / 2, clientY: bounds.top + bounds.height / 2, ...values}));};
    compare('initial fit and localized controls');
    if (!hosts[0].querySelector('[aria-label="Enlarge"]') || !hosts[0].querySelector('.pele-zoom-out').disabled || hosts[0].firstElementChild.getAttribute('tabindex') !== '0') failures.push('rest focus and disabled reduction');
    both(zoom => zoom.zoomBy(2)); compare('programmatic zoom');
    both((zoom, host) => host.querySelector('.pele-zoom-in').click()); compare('button zoom');
    both(zoom => zoom.zoomTo(100)); compare('maximum clamp');
    if (zooms[0].scale !== 2.5) failures.push('maxScale not enforced');
    for (const key of ['ArrowRight', 'ArrowDown', 'ArrowLeft', 'ArrowUp', '-', '+', '0', '=']) {
      both((zoom, host) => event(host, KeyboardEvent, 'keydown', {key})); compare('keyboard '+key);
    }
    const before = zooms[0].scale;
    const unmodified = hosts.map(host => event(host, WheelEvent, 'wheel', {deltaY: -100}));
    if (!unmodified.every(Boolean) || zooms[0].scale !== before) failures.push('ordinary wheel consumed');
    both((zoom, host) => event(host, WheelEvent, 'wheel', {deltaY: -100, ctrlKey: true})); compare('modified wheel');
    both((zoom, host) => event(host, MouseEvent, 'dblclick', {})); compare('double click reset');
    both((zoom, host) => event(host, MouseEvent, 'dblclick', {})); compare('double click enlarge');
    both((zoom, host) => event(host, KeyboardEvent, 'keydown', {key: 'Escape'})); compare('escape reset');
    both((zoom, host) => {host.style.width = '450px';zoom.refresh();}); compare('resize refresh');
    both(zoom => zoom.destroy()); compare('destroy restores styles and controls');
    const scales = zooms.map(zoom => zoom.scale);
    both((zoom, host) => event(host, KeyboardEvent, 'keydown', {key: '+'}));
    if (zooms.some((zoom, i) => zoom.scale !== scales[i]) || hosts[0].querySelector('.pele-zoom') || hosts[0].firstElementChild.hasAttribute('tabindex')) failures.push('destroy left input/controls/focus');
    hosts.forEach(host => host.remove());
    return failures;
  });
  expect(failures).toEqual([]);
});

test('real pointer drag stays inside drawing and suppresses link clicks until reset', async ({page}) => {
  await page.goto('/examples/web/');
  await page.evaluate(async () => {
    const {loadRenderer, enableZoom} = await import('/dist/package-web/package/full.js');
    const oracle = await import('/dist/evidence/pele-oracle.js'), engine = await loadRenderer();
    const svg = engine.render('flowchart LR\nA-->B-->C-->D-->E\nclick A "https://example.com"', {idPrefix: 'drag-test'}).svg;
    window.dragClicks = [0, 0];
    window.dragHosts = [0, 1].map(i => {const host = document.createElement('div');host.id = 'drag-'+i;host.style.cssText = 'width:260px;height:120px;margin:20px';host.innerHTML = svg;document.body.append(host);return host;});
    window.dragHosts.forEach((host, i) => host.addEventListener('click', event => {event.preventDefault();window.dragClicks[i]++;}));
    window.dragZooms = [enableZoom(window.dragHosts[0]), oracle.enableZoom(window.dragHosts[1])];
    window.dragZooms.forEach(zoom => zoom.zoomTo(1));
  });
  for (const id of ['drag-0', 'drag-1']) {
    const bounds = await page.locator('#'+id+' > svg').boundingBox();
    await page.mouse.move(bounds.x + bounds.width / 2, bounds.y + bounds.height / 2);
    await page.mouse.down();
    await page.mouse.move(bounds.x + bounds.width / 2 - 65, bounds.y + bounds.height / 2 - 10, {steps: 5});
    await page.mouse.up();
  }
  const result = await page.evaluate(() => {
    const views = window.dragHosts.map(host => host.firstElementChild.getAttribute('viewBox'));
    const cursors = window.dragHosts.map(host => host.firstElementChild.style.cursor);
    window.dragZooms.forEach(zoom => zoom.reset());
    const reset = window.dragHosts.map(host => host.firstElementChild.getAttribute('viewBox'));
    return {views, cursors, reset, suppressedClicks: [...window.dragClicks]};
  });
  for (const id of ['drag-0', 'drag-1']) await page.locator('#'+id+' a').first().click();
  const clicks = await page.evaluate(() => {
    const clicks = [...window.dragClicks];
    window.dragZooms.forEach(zoom => zoom.destroy());window.dragHosts.forEach(host => host.remove());
    delete window.dragHosts;delete window.dragZooms;delete window.dragClicks;
    return clicks;
  });
  expect(result.views[0]).toEqual(result.views[1]);
  expect(result.cursors).toEqual(['grab', 'grab']);
  expect(result.reset[0]).toEqual(result.reset[1]);
  expect(result.suppressedClicks).toEqual([0, 0]);
  expect(clicks).toEqual([1, 1]);
});
