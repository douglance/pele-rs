//! Pele sequence/actors.ts participant shape geometry, 458322a5 (MIT, kepano).
use crate::compat::number_math::NumberMath;
use crate::compat::{
    svg::{label_svg_with_resources, num},
    text::Label,
    PeleError, RenderResources,
};
const SHAPE: &str = " fill=\"var(--_s)\" stroke=\"var(--_b)\"";
const LINE: &str = " fill=\"none\" stroke=\"var(--_b)\"";
const RADIUS: &str = "var(--pele-radius,4px)";
#[derive(Clone, Copy)]
pub(super) struct ActorSize {
    pub w: f64,
    pub h: f64,
}
pub(super) fn actor_kind(kind: &str) -> &str {
    if matches!(
        kind,
        "participant"
            | "actor"
            | "boundary"
            | "control"
            | "entity"
            | "database"
            | "collections"
            | "queue"
    ) {
        kind
    } else {
        "participant"
    }
}
pub(super) fn is_figure(kind: &str) -> bool {
    matches!(
        kind,
        "actor" | "boundary" | "control" | "entity" | "database"
    )
}
pub(super) fn actor_size(
    kind: &str,
    label: &Label,
    min_w: f64,
    min_h: f64,
    padding_x: f64,
) -> ActorSize {
    if is_figure(kind) {
        ActorSize {
            w: 40_f64.js_max(label.width + 8.),
            h: 34.
                + if label.height > 0. {
                    4. + label.height
                } else {
                    0.
                },
        }
    } else {
        let extra = if kind == "collections" {
            5.
        } else if kind == "queue" {
            12.
        } else {
            0.
        };
        ActorSize {
            w: min_w.js_max(label.width + 2. * padding_x) + extra,
            h: min_h.js_max(label.height + 20.) + if kind == "collections" { 5. } else { 0. },
        }
    }
}
fn figure(kind: &str, cx: f64, top: f64) -> String {
    let x = num(cx);
    let y = |dy: f64| num(top + dy);
    match kind{"actor"=>format!("<circle cx=\"{x}\" cy=\"{}\" r=\"6\"{SHAPE}/><path d=\"M{x},{}V{}M{},{}H{}M{},{}L{x},{}L{},{}\"{LINE}/>",y(7.),y(13.),y(24.),num(cx-10.),y(17.),num(cx+10.),num(cx-8.),y(34.),y(24.),num(cx+8.),y(34.)),"boundary"=>format!("<path d=\"M{},{}V{}M{},{}H{}\"{LINE}/><circle cx=\"{}\" cy=\"{}\" r=\"11\"{SHAPE}/>",num(cx-19.),y(6.),y(28.),num(cx-19.),y(17.),num(cx-8.),num(cx+3.),y(17.)),"control"=>format!("<circle cx=\"{x}\" cy=\"{}\" r=\"11\"{SHAPE}/><path d=\"M{},{}L{},{}L{},{}\"{LINE}/>",y(21.),num(cx+5.),y(5.),num(cx-1.),y(10.),num(cx+5.),y(15.)),"entity"=>format!("<circle cx=\"{x}\" cy=\"{}\" r=\"11\"{SHAPE}/><path d=\"M{},{}H{}\"{LINE}/>",y(15.),num(cx-11.),y(30.),num(cx+11.)),_=>{let l=num(cx-14.);let r=num(cx+14.);format!("<path d=\"M{l},{}V{}A14,5 0 0 0 {r},{}V{}A14,5 0 0 0 {l},{}Z\"{SHAPE}/><path d=\"M{l},{}A14,5 0 0 0 {r},{}\"{LINE}/>",y(7.),y(27.),y(27.),y(7.),y(7.),y(7.),y(7.))}}
}
pub(super) fn draw_actor(
    kind: &str,
    cx: f64,
    top: f64,
    size: ActorSize,
    label: &Label,
    resources: &mut dyn RenderResources,
) -> Result<String, PeleError> {
    let ActorSize { w, h } = size;
    let attrs = " class=\"pele-label\"";
    if is_figure(kind) {
        return Ok(figure(kind, cx, top)
            + &label_svg_with_resources(
                label,
                cx,
                top + 38. + label.height / 2.,
                attrs,
                resources,
            )?);
    }
    let left = cx - w / 2.;
    if kind == "collections" {
        let bw = w - 5.;
        let bh = h - 5.;
        return Ok(format!("<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{RADIUS}\"{SHAPE}/><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{RADIUS}\"{SHAPE}/>{}",num(left+5.),num(top),num(bw),num(bh),num(left),num(top+5.),num(bw),num(bh),label_svg_with_resources(label,left+bw/2.,top+5.+bh/2.,attrs,resources)?));
    }
    if kind == "queue" {
        let l = num(left + 6.);
        let r = num(left + w - 6.);
        let b = num(top + h);
        let ry = num(h / 2.);
        let t = num(top);
        return Ok(format!("<path d=\"M{l},{t}H{r}A6,{ry} 0 0 1 {r},{b}H{l}A6,{ry} 0 0 1 {l},{t}Z\"{SHAPE}/><path d=\"M{r},{t}A6,{ry} 0 0 0 {r},{b}\"{LINE}/>{}",label_svg_with_resources(label,cx-3.,top+h/2.,attrs,resources)?));
    }
    Ok(format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{RADIUS}\"{SHAPE}/>{}",
        num(left),
        num(top),
        num(w),
        num(h),
        label_svg_with_resources(label, cx, top + h / 2., attrs, resources)?
    ))
}
