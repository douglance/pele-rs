use super::*;
use super::{
    placement::display,
    validation::{limit, Budget},
};
use crate::compat::PeleError;
use std::fmt::Write;
pub(super) fn svg(s: &Scene, o: &Options, b: &mut Budget) -> Result<String, PeleError> {
    b.spend(s.edges.iter().map(|e| e.points.len()).sum::<usize>().pow(2))?;
    let mut out=format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\" role=\"img\" aria-label=\"Directed graph\"><style>.graph-edge{{fill:none;stroke:var(--pele-line,#667085);stroke-width:1.2}}.graph-label{{font:11px Arial,sans-serif;fill:var(--pele-fg,#182230)}}.graph-title{{font:14px Arial,sans-serif;fill:var(--pele-fg,#182230)}}.graph-detail{{font:11px Arial,sans-serif;fill:var(--pele-muted,#475467)}}</style>",s.width,s.height,s.width,s.height);
    for (i, e) in s.edges.iter().enumerate() {
        edge(
            &mut out,
            e,
            &s.edges
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, e)| e.points.as_slice())
                .collect::<Vec<_>>(),
        );
        bound(&out, o)?;
    }
    for n in &s.nodes {
        node(&mut out, n);
        bound(&out, o)?;
    }
    out.push_str("</svg>");
    bound(&out, o)?;
    Ok(out)
}
fn bound(s: &str, o: &Options) -> Result<(), PeleError> {
    if s.len() > o.output_limit {
        Err(limit("graph SVG output limit exceeded"))
    } else {
        Ok(())
    }
}
fn edge(out: &mut String, e: &Route, others: &[&[Point]]) {
    let dashed = if e.edge.label == "contains" || e.edge.label == "refs" {
        " stroke-dasharray=\"4 3\""
    } else {
        ""
    };
    let _=write!(out,"<g class=\"pele-edge\" data-id=\"{}\" data-source=\"{}\" data-target=\"{}\"><title>{}</title><path class=\"graph-edge\"{} d=\"{}\"/>",esc(&e.edge.id),esc(&e.edge.source),esc(&e.edge.target),esc(&e.edge.label),dashed,super::path::path(&e.points,others));
    let z = *e.points.last().unwrap();
    let a = e.points[e.points.len() - 2];
    let distance = a.distance(z);
    let (dx, dy) = ((z.x - a.x) / distance, (z.y - a.y) / distance);
    let p = Point {
        x: z.x - dx * 7. - dy * 3.,
        y: z.y - dy * 7. + dx * 3.,
    };
    let q = Point {
        x: z.x - dx * 7. + dy * 3.,
        y: z.y - dy * 7. - dx * 3.,
    };
    let _ = write!(
        out,
        "<path d=\"M{},{}L{},{}L{},{}Z\" fill=\"var(--pele-line,#667085)\"/>",
        z.x, z.y, p.x, p.y, q.x, q.y
    );
    if e.label_box.width > 0. {
        let r = e.label_box;
        let _=write!(out,"<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" fill=\"var(--pele-bg,#fff)\"/><text class=\"graph-label\" x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>",r.x,r.y,r.width,r.height,r.x+r.width/2.,r.y+14.,esc(&display(&e.edge.label,24)));
    }
    out.push_str("</g>");
}
fn node(out: &mut String, n: &PlacedNode) {
    let r = n.rect;
    let dash = if n.node.external {
        " stroke-dasharray=\"4 3\""
    } else {
        ""
    };
    let _=write!(out,"<g class=\"pele-node\" data-id=\"{}\"><title>{}</title><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" fill=\"var(--pele-surface,#f9fafb)\" stroke=\"var(--pele-border,#667085)\"{}/><text class=\"graph-title\" x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text><text class=\"graph-detail\" x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text></g>",esc(&n.node.id),esc(&n.node.label),r.x,r.y,r.width,r.height,dash,r.x+r.width/2.,r.y+27.,esc(&display(&n.node.label,36)),r.x+r.width/2.,r.y+46.,esc(&display(&n.node.detail,28)));
}

fn esc(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c if (c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
                || matches!(c, '\u{fffe}' | '\u{ffff}') =>
            {
                out.push(' ')
            }
            _ => out.push(c),
        }
    }
    out
}
