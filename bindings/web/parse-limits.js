// Exact declared number-valued parse policies. Other runtime values retain the
// existing JSON decoder instead of being silently coerced or defaulted.
function threshold(value) {
  if (value === undefined) return [0, 0];
  if (value === null) return [1, 0];
  if (typeof value === 'number') return [2, value];
  return [3, 0];
}
export function parseLimitValues(options) {
  const [sourceKind, sourceValue] = threshold(options.limit);
  const [edgeKind, edgeValue] = threshold(options.maxEdges);
  return {sourceKind, sourceValue, edgeKind, edgeValue};
}
export function parseLimitOptions(options) {
  const legacy = {...options};
  const {sourceKind, sourceValue, edgeKind, edgeValue} = parseLimitValues(legacy);
  if (sourceKind !== 3) delete legacy.limit;
  if (edgeKind !== 3) delete legacy.maxEdges;
  return {json: JSON.stringify(legacy), sourceKind, sourceValue, edgeKind, edgeValue};
}
