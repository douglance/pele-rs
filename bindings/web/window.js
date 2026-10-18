// Browser host adapter port of Pele src/window.ts, MIT (kepano).
// Pinned revision 458322a5b8abb1ae57ddf127c32017ab9fb14a92.
// Source SHA256 48cc7abe6b5b496bdd8bcf2017bd251beef057c1a477d69eaeb18a7279d34d3a.
// Rendering, parsing and layout remain in Rust.
export function windowOf(element) {
    return element.ownerDocument?.defaultView ?? globalThis;
}
export function room(element, style) {
    const width = element.clientWidth - (parseFloat(style.paddingLeft) || 0) - (parseFloat(style.paddingRight) || 0);
    return width > 0 ? width : 0;
}
