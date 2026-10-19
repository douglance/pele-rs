// Numeric render fields cross WASM as raw f64 values, never JSON numbers.
// Other options retain their existing JSON contract (including config).
const fields = ['fontSize', 'maxWidth', 'directionBreakpoint', 'padding', 'now'];
export function renderNumericOptions(options) {
  const legacy = {...options}, numbers = new Float64Array(5);
  let mask = 0;
  for (let index = 0; index < fields.length; index++) {
    const field = fields[index];
    let value = legacy[field];
    if (field === 'now' && value !== null && typeof value === 'object') {
      // Internal Date branding accepts other realms without invoking coercion.
      try { value = Date.prototype.getTime.call(value); } catch { /* Remaining objects retain JSON semantics. */ }
    }
    if (typeof value === 'number') {
      numbers[index] = value; mask |= 1 << index; delete legacy[field];
    }
  }
  // Pele defaults to the exact post-render threshold even when omitted.
  let outputKind = 0, output = 0;
  if (Object.hasOwn(legacy, 'outputLimit')) {
    const value = legacy.outputLimit;
    if (value === undefined) outputKind = 0;
    else if (value === null) outputKind = 1;
    else if (typeof value === 'number') {outputKind = 2; output = value;}
    else outputKind = 3;
    if (outputKind !== 3) delete legacy.outputLimit;
  }
  return {json: JSON.stringify(legacy), mask, numbers, outputKind, output};
}
