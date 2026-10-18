use crate::{layout::Layout, model::q, Error, LineStyle, Point, Rect, Shape};
use std::fmt::Write;

const MAX_SVG: usize = 8 * 1024 * 1024;
struct Output(String);
impl Output {
    fn push(&mut self, s: &str) -> Result<(), Error> {
        if self.0.len().saturating_add(s.len()) > MAX_SVG {
            return Err(Error::new("output_limit", "SVG exceeds 8 MiB"));
        }
        self.0.push_str(s);
        Ok(())
    }
}
pub fn render_svg(scene: &Layout) -> Result<String, Error> {
    let mut out = Output(String::new());
    let options = &scene.options;
    let theme = &options.theme;
    out.push(&format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\" role=\"img\" aria-label=\"{}\"><title>{}</title><desc>Flowchart with {} nodes and {} edges. Labels are outlined using bundled Noto Sans.</desc><rect width=\"{}\" height=\"{}\" fill=\"{}\"/>",scene.width(),scene.height(),scene.width(),scene.height(),escape(&options.title),escape(&options.title),scene.geometry.nodes.len(),scene.geometry.edges.len(),scene.width(),scene.height(),theme.background))?;
    for e in &scene.geometry.edges {
        let mut points = e.points.clone();
        let mut arrow = None;
        if e.arrow && points.len() > 1 {
            let tip = *points.last().unwrap();
            let prior = points[points.len() - 2];
            let length = ((tip.x - prior.x).powi(2) + (tip.y - prior.y).powi(2)).sqrt();
            if length > 0. {
                let ux = (tip.x - prior.x) / length;
                let uy = (tip.y - prior.y) / length;
                let size = 8_f64.min(length * 0.75);
                let base = Point {
                    x: tip.x - ux * size,
                    y: tip.y - uy * size,
                };
                *points.last_mut().unwrap() = base;
                arrow = Some(format!(
                    "{},{} {},{} {},{}",
                    q(tip.x),
                    q(tip.y),
                    q(base.x - uy * 4.),
                    q(base.y + ux * 4.),
                    q(base.x + uy * 4.),
                    q(base.y - ux * 4.)
                ));
            }
        }
        let mut path = String::new();
        for (i, p) in points.iter().enumerate() {
            write!(
                path,
                "{}{} {}",
                if i == 0 { "M" } else { "L" },
                q(p.x),
                q(p.y)
            )
            .unwrap();
        }
        let dash = if e.style == LineStyle::Dashed {
            " stroke-dasharray=\"6 4\""
        } else {
            ""
        };
        let stroke = q(options.stroke_width * if e.style == LineStyle::Thick { 2.5 } else { 1. });
        out.push(&format!("<g data-edge=\"{}\"><title>{}</title><path d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" stroke-linejoin=\"round\"{}/>",e.index,escape(&e.label),path,theme.line,stroke,dash))?;
        if let Some(points) = arrow {
            out.push(&format!(
                "<polygon points=\"{}\" fill=\"{}\"/>",
                points, theme.line
            ))?;
        }
        out.push("</g>")?;
    }
    for (i, n) in scene.geometry.nodes.iter().enumerate() {
        let r = n.bounds;
        let center = r.center();
        out.push(&format!(
            "<g data-node=\"{}\" aria-label=\"{}\"><title>{}</title>",
            escape(&n.id),
            escape(&n.label),
            escape(&n.label)
        ))?;
        let common = format!(
            "fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"",
            theme.surface, theme.border, options.stroke_width
        );
        match n.shape {
            Shape::Circle => out.push(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" {}/>",
                q(center.x),
                q(center.y),
                q(r.width / 2.),
                common
            ))?,
            Shape::Diamond => out.push(&format!(
                "<polygon points=\"{},{} {},{} {},{} {},{}\" {}/>",
                q(center.x),
                r.y,
                q(r.x + r.width),
                q(center.y),
                q(center.x),
                q(r.y + r.height),
                r.x,
                q(center.y),
                common
            ))?,
            _ => {
                let radius = match n.shape {
                    Shape::Rounded => options.radius.min(r.width / 2.).min(r.height / 2.),
                    Shape::Stadium => r.width.min(r.height) / 2.,
                    _ => 0.,
                };
                out.push(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" {}/>",
                    r.x,
                    r.y,
                    r.width,
                    r.height,
                    q(radius),
                    common
                ))?;
            }
        }
        let text = &scene.node_text[i];
        let label = Rect {
            x: center.x - text.width / 2.,
            y: center.y - text.height / 2.,
            width: text.width,
            height: text.height,
        };
        out.push(&format!("<g fill=\"{}\">", theme.foreground))?;
        out.push(&text.paths(label)?)?;
        out.push("</g></g>")?;
    }
    for (i, e) in scene.geometry.edges.iter().enumerate() {
        if let Some(r) = e.label_bounds {
            out.push(&format!("<g data-label=\"{}\"><title>{}</title><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"2\" fill=\"{}\"/><g fill=\"{}\">",e.index,escape(&e.label),r.x,r.y,r.width,r.height,theme.background,theme.foreground))?;
            let text = &scene.edge_text[i];
            let center = r.center();
            out.push(&text.paths(Rect {
                x: center.x - text.width / 2.,
                y: center.y - text.height / 2.,
                width: text.width,
                height: text.height,
            })?)?;
            out.push("</g></g>")?;
        }
    }
    out.push("</svg>")?;
    Ok(out.0)
}
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
