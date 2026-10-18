// Host values only: diagram state transitions and rendering remain in Rust.
const bits = new DataView(new ArrayBuffer(8));
function numberBits(value) {
  bits.setFloat64(0, value);
  return bits.getBigUint64(0).toString(16).padStart(16, '0');
}
function numberValue(hex) {
  bits.setBigUint64(0, BigInt('0x' + hex));
  return bits.getFloat64(0);
}
function units(value) {
  const output = new Array(value.length);
  for (let i = 0; i < value.length; i++) output[i] = value.charCodeAt(i);
  return output;
}
export function stringValue(value) {
  let output = '';
  for (let i = 0; i < value.length; i += 8192) output += String.fromCharCode(...value.slice(i, i + 8192));
  return output;
}
/** Schema functions/prototypes share identity across documents of one renderer. */
export class ModelRegistry {
  constructor() { this.schemas = new Map(); this.owners = new WeakMap(); this.sharedFunctions = new Map(); }
  registration(schema) {
    const key = `${schema.id}:${schema.version}`;
    if (this.schemas.has(key)) {
      const known=this.schemas.get(key);
      if (JSON.stringify(known.schema)!==JSON.stringify(schema)) throw new TypeError('Conflicting native model schema identity');
      return known;
    }
    const registration = {schema, functions: new Map(), functionIds: new WeakMap(), prototypes: new Map(), prototypeIds: new WeakMap(), documents: [], sharedIds: new Map((schema.sharedFunctions??[]).map(entry=>[entry.identity,entry.method]))};
    this.schemas.set(key, registration);
    const aliases=new Map((schema.sharedFunctions??[]).map(entry=>[entry.method,entry.identity]));
    const shapes=new Map((schema.functionShapes??[]).map(entry=>[entry.method,entry.constructible]));
    const live=registration=>{
      registration.documents=registration.documents.filter(reference=>{const value=reference.deref();return value&&!value.disposed;});
      return registration.documents.at(-1)?.deref();
    };
    for (const method of schema.methods) {
      const registry = this, identity=aliases.get(method.id), constructible=shapes.get(method.id)??false;
      let shared=identity&&this.sharedFunctions.get(identity);
      if(shared&&(shared.name!==method.name||shared.arity!==method.arity||shared.constructible!==constructible))throw new TypeError('Conflicting shared native function descriptor');
      if(identity&&!shared){shared={name:method.name,arity:method.arity,constructible,registrations:new Set(),callable:null};this.sharedFunctions.set(identity,shared);}
      if(shared)shared.registrations.add(registration);
      const dispatch=(receiver,argumentsValue)=>{
        const direct=receiver&&registry.owners.get(receiver);
        let owner;
        if(shared) {
          if(direct&&!direct.disposed&&direct.registration.sharedIds.has(identity))owner=direct;
          if(!owner)for(const candidate of [...shared.registrations].reverse()){owner=live(candidate);if(owner)break;}
        } else owner=direct?.registration===registration&&!direct.disposed?direct:live(registration);
        if (!owner) throw new TypeError('Public method has no live document');
        return owner.invoke(shared?owner.registration.sharedIds.get(identity):method.id, receiver, argumentsValue);
      };
      // Keep the outline helper's intrinsic arity: rewriting it changes
      // Firefox's observed ordering of ordinary function properties.
      const callable=(shared&&shared.callable)||(constructible?
        {[method.name]:function(_first,_second){return dispatch(this,Array.from(arguments));}}[method.name]:
        {[method.name](...argumentsValue){return dispatch(this,argumentsValue);}}[method.name]);
      if(!constructible||method.arity!==2)Object.defineProperty(callable, 'length', {value: method.arity, configurable: true});
      if(shared)shared.callable=callable;
      registration.functions.set(method.id, callable);
      registration.functionIds.set(callable, method.id);
    }
    for (const prototype of schema.prototypes) {
      const value = Object.create(null);
      registration.prototypes.set(prototype.id, value);
      registration.prototypeIds.set(value, prototype.id);
    }
    for (const prototype of schema.prototypes) {
      const value = registration.prototypes.get(prototype.id), parent = prototype.parent;
      Object.setPrototypeOf(value, parent.kind === 'null' ? null : parent.kind === 'prototype' ? registration.prototypes.get(parent.prototype) : Object.prototype);
      // Construction remains an explicit native capability gap. The descriptor
      // still exposes the correct class-shaped prototype and member flags.
      const constructor = function () { throw new TypeError('Public model construction is unavailable'); };
      Object.defineProperty(constructor, 'name', {value: prototype.name, configurable: true});
      Object.defineProperty(constructor, 'prototype', {value, writable: false});
      Object.defineProperty(value, 'constructor', {value: constructor, writable: true, configurable: true});
      for (const member of prototype.members) {
        const descriptor = {enumerable: !!(member.flags & 1), configurable: !!(member.flags & 4)};
        if (member.kind === 'method') {
          descriptor.value = registration.functions.get(member.method);
          descriptor.writable = !!(member.flags & 2);
        } else {
          descriptor.get = member.get == null ? undefined : registration.functions.get(member.get);
          descriptor.set = member.set == null ? undefined : registration.functions.get(member.set);
        }
        Object.defineProperty(value, member.name, descriptor);
      }
    }
    registration.prototypeBaseline = [...registration.prototypes.values()].map(value => ({value, parent:Object.getPrototypeOf(value), descriptors:Object.getOwnPropertyDescriptors(value)}));
    return registration;
  }
}
export class ModelBridge {
  constructor(graph, schema, registry, invoke) {
    this.registry = registry;
    this.registration = registry.registration(schema);
    if(this.registration.documents.length>256)this.registration.documents=this.registration.documents.filter(reference=>{const value=reference.deref();return value&&!value.disposed;});
    this.registration.documents.push(new WeakRef(this));
    this.disposed=false;
    this.invoke = invoke;
    this.ids = new WeakMap();
    this.hostIds = new Map();
    this.hostValues = [];
    this.objects = [];
    this.reconcile(graph);
  }
  reconcile(graph) {
    this.graph = graph;
    this.strings = graph.strings.map(stringValue);
    this.stringIds = new Map(this.strings.map((value, index) => [value, index]));
    while (this.objects.length < graph.nodes.length) {
      const id = this.objects.length, node = graph.nodes[id];
      let value;
      switch (node.kind) {
        case 'instance': value = Object.create(this.registration.prototypes.get(node.prototype)); break;
        case 'object': value = {}; break;
        case 'array': value = new Array(node.length); break;
        case 'map': value = new Map(); break;
        case 'set': value = new Set(); break;
        case 'date': value = new Date(numberValue(node.milliseconds)); break;
        default: throw new TypeError('Unsupported public object kind');
      }
      this.objects.push(value); this.ids.set(value, id); this.registry.owners.set(value, this);
    }
    graph.nodes.forEach((node, id) => {
      const object = this.objects[id], keys = new Set(node.properties.map(property => this.strings[property.key]));
      for (const key of Object.getOwnPropertyNames(object)) {
        if (node.kind === 'array' && key === 'length') continue;
        if (!keys.has(key) && !Reflect.deleteProperty(object, key)) throw new TypeError('Native graph removed a non-configurable property');
      }
      if (node.kind === 'map') {
        Map.prototype.clear.call(object);
        for (const [key, value] of node.entries) Map.prototype.set.call(object, this.value(key), this.value(value));
      }
      if (node.kind === 'set') {
        Set.prototype.clear.call(object);
        for (const value of node.values) Set.prototype.add.call(object, this.value(value));
      }
      if (node.kind === 'date') Date.prototype.setTime.call(object, numberValue(node.milliseconds));
      // Set length before entries, preserving holes and existing non-index keys.
      if (node.kind === 'array' && object.length !== node.length) object.length = node.length;
      for (const property of node.properties) Object.defineProperty(object, this.strings[property.key], {
        value: this.value(property.value), enumerable: !!(property.flags & 1),
        writable: !!(property.flags & 2), configurable: !!(property.flags & 4)
      });
    });
    this.root = this.value(graph.root);
    return this.root;
  }
  value(atom) {
    switch (atom.kind) {
      case 'undefined': return undefined;
      case 'null': return null;
      case 'bool': return atom.value;
      case 'number': return numberValue(atom.value);
      case 'string': return this.strings[atom.value];
      case 'ref': return this.objects[atom.value];
      case 'builtinFunction': return this.registration.functions.get(atom.value);
      case 'hostReference': return this.hostValues[atom.value];
      default: throw new TypeError('Unknown public atom kind');
    }
  }
  // All retained nodes are scanned, including externally held detached arrays.
  // Extra arguments join the same arena; calls never serialize nested objects.
  snapshot(values = []) {
    for (const baseline of this.registration.prototypeBaseline) {
      const descriptors = Object.getOwnPropertyDescriptors(baseline.value), original = baseline.descriptors;
      if (Object.getPrototypeOf(baseline.value) !== baseline.parent || Reflect.ownKeys(descriptors).length !== Reflect.ownKeys(original).length || Reflect.ownKeys(original).some(key => {
        const a=original[key],b=descriptors[key];
        return !b || a.value!==b.value || a.get!==b.get || a.set!==b.set || a.enumerable!==b.enumerable || a.writable!==b.writable || a.configurable!==b.configurable;
      })) throw new TypeError('Prototype edits require an explicit native prototype mutation codec');
    }
    const pending = [], seen = new Set(), nodes = this.graph.nodes.slice();
    let codeUnits = this.strings.reduce((n,s)=>n+s.length,0), propertiesCount = 0;
    const budget = () => { throw new RangeError('Public model graph exceeds host exchange budget'); };
    const string = value => {
      if (this.stringIds.has(value)) return this.stringIds.get(value);
      const id = this.strings.length;
      codeUnits += value.length;
      if (id >= 1_000_000 || codeUnits > 16_000_000) budget();
      this.strings.push(value); this.stringIds.set(value, id);
      return id;
    };
    const host = value => {
      if (!this.hostIds.has(value)) {
        this.hostIds.set(value, this.hostValues.length); this.hostValues.push(value);
      }
      return {kind: 'hostReference', value: this.hostIds.get(value)};
    };
    const atom = value => {
      if (value === undefined) return {kind: 'undefined'};
      if (value === null) return {kind: 'null'};
      switch (typeof value) {
        case 'boolean': return {kind: 'bool', value};
        case 'number': return {kind: 'number', value: numberBits(value)};
        case 'string': return {kind: 'string', value: string(value)};
        case 'function': {
          const id = this.registration.functionIds.get(value);
          return id === undefined ? host(value) : {kind: 'builtinFunction', value: id};
        }
        case 'object': {
          const prototype = Object.getPrototypeOf(value);
          if (!(Array.isArray(value) || value instanceof Map || value instanceof Set || value instanceof Date || prototype === Object.prototype || this.registration.prototypeIds.has(prototype))) return host(value);
          let id = this.ids.get(value);
          if (id === undefined) {
            id = this.objects.length; if (id >= 1_000_000) budget(); this.objects.push(value); this.ids.set(value, id);
          }
          if (!seen.has(id)) { seen.add(id); pending.push([id, value]); }
          return {kind: 'ref', value: id};
        }
        default: return host(value);
      }
    };
    const root = atom(this.root), argumentsValue = values.map(atom);
    // Detaching an array does not detach references that the caller still holds.
    for (const object of this.objects) atom(object);
    while (pending.length) {
      const [id, value] = pending.pop(), properties = [];
      for (const [key, descriptor] of Object.entries(Object.getOwnPropertyDescriptors(value))) {
        if (Array.isArray(value) && key === 'length') {
          if (descriptor.writable) continue;
        }
        if (!Object.hasOwn(descriptor, 'value')) throw new TypeError('Accessor properties require an explicit public model codec');
        if (++propertiesCount > 8_000_000) budget();
        properties.push({key: string(key), value: atom(descriptor.value), flags:
          (descriptor.enumerable ? 1 : 0) | (descriptor.writable ? 2 : 0) | (descriptor.configurable ? 4 : 0)});
      }
      if (value instanceof Map) {
        const entries = [];
        for (const [key, item] of Map.prototype.entries.call(value)) {
          propertiesCount += 2; if (propertiesCount > 8_000_000) budget();
          entries.push([atom(key), atom(item)]);
        }
        nodes[id] = {kind: 'map', entries, properties};
      } else if (value instanceof Set) {
        const values = [];
        for (const item of Set.prototype.values.call(value)) {
          if (++propertiesCount > 8_000_000) budget(); values.push(atom(item));
        }
        nodes[id] = {kind:'set', values, properties};
      }
      else if (value instanceof Date) nodes[id] = {kind: 'date', milliseconds: numberBits(Date.prototype.getTime.call(value)), properties};
      else if (Array.isArray(value)) nodes[id] = {kind: 'array', length: value.length, properties};
      else {
        const prototype = this.registration.prototypeIds.get(Object.getPrototypeOf(value));
        nodes[id] = prototype === undefined ? {kind: 'object', properties} : {kind: 'instance', prototype, properties};
      }
    }
    this.graph = {version: 1, root, strings: this.strings.map(units), nodes};
    return {graph: this.graph, values: argumentsValue};
  }
}
