//! One pinned font drives both measurement and drawing. No host font lookup.
use crate::{Error, Rect};
use rustybuzz::{
    ttf_parser::{self, OutlineBuilder},
    Direction, Face, UnicodeBuffer,
};
use std::fmt::Write;

const FONT: &[u8] = include_bytes!("../assets/fonts/NotoSans-Regular.ttf");
pub(crate) const MAX_GLYPHS: usize = 65536;

#[derive(Clone, Debug)]
struct Glyph {
    id: u16,
    x: f64,
    y: f64,
}
#[derive(Clone, Debug)]
pub(crate) struct Text {
    pub width: f64,
    pub height: f64,
    size: f64,
    lines: Vec<Vec<Glyph>>,
}

impl Text {
    pub fn shape(text: &str, size: f64, budget: &mut usize) -> Result<Self, Error> {
        let face = Face::from_slice(FONT, 0)
            .ok_or_else(|| Error::new("internal", "Bundled font cannot load"))?;
        let scale = size / face.units_per_em() as f64;
        let mut lines = vec![];
        let mut width: f64 = 0.;
        for line in text.split('\n') {
            for c in line.chars() {
                if matches!(c as u32,0x0590..=0x08ff|0xfb1d..=0xfdff|0xfe70..=0xfefc|0x200e..=0x200f|0x202a..=0x202e|0x2066..=0x2069|0x1e900..=0x1e95f)
                {
                    return Err(Error::new(
                        "unsupported_text",
                        "Right-to-left text and bidi controls are not supported",
                    ));
                }
                if face.glyph_index(c).is_none() {
                    return Err(Error::new(
                        "unsupported_glyph",
                        format!("Bundled font does not cover U+{:04X}", c as u32),
                    ));
                }
            }
            let mut buffer = UnicodeBuffer::new();
            buffer.push_str(line);
            buffer.guess_segment_properties();
            buffer.set_direction(Direction::LeftToRight);
            let shaped = rustybuzz::shape(&face, &[], buffer);
            *budget = budget
                .checked_add(shaped.len())
                .ok_or_else(|| Error::new("graph_limit", "Glyph budget exceeded"))?;
            if *budget > MAX_GLYPHS {
                return Err(Error::new(
                    "graph_limit",
                    "Diagram exceeds 65536 shaped glyphs",
                ));
            }
            let mut glyphs = vec![];
            let mut pen = 0.;
            let mut left: f64 = 0.;
            let mut right: f64 = 0.;
            for (info, pos) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
                if info.glyph_id == 0 {
                    return Err(Error::new(
                        "unsupported_glyph",
                        "Shaping produced a missing glyph",
                    ));
                }
                let x = pen + pos.x_offset as f64 * scale;
                if let Some(bounds) =
                    face.glyph_bounding_box(ttf_parser::GlyphId(info.glyph_id as u16))
                {
                    left = left.min(x + bounds.x_min as f64 * scale);
                    right = right.max(x + bounds.x_max as f64 * scale);
                }
                glyphs.push(Glyph {
                    id: info.glyph_id as u16,
                    x,
                    y: pos.y_offset as f64 * scale,
                });
                pen += pos.x_advance as f64 * scale;
            }
            for glyph in &mut glyphs {
                glyph.x -= left;
            }
            width = width.max(pen.max(right) - left);
            lines.push(glyphs);
        }
        let height = lines.len() as f64 * size * 1.5;
        Ok(Self {
            width,
            height,
            size,
            lines,
        })
    }
    pub fn paths(&self, bounds: Rect) -> Result<String, Error> {
        let face = ttf_parser::Face::parse(FONT, 0)
            .map_err(|_| Error::new("internal", "Bundled font cannot load"))?;
        let scale = self.size / face.units_per_em() as f64;
        let mut out = String::new();
        for (line_index, line) in self.lines.iter().enumerate() {
            let advance = line.iter().map(|g| g.x).last().unwrap_or(0.);
            // Shape() already accounts for overhang. Center each line using its glyph bounds.
            let line_width = line
                .iter()
                .filter_map(|g| {
                    face.glyph_bounding_box(ttf_parser::GlyphId(g.id))
                        .map(|b| g.x + b.x_max as f64 * scale)
                })
                .fold(advance, f64::max);
            let left = bounds.x + (bounds.width - line_width) / 2.;
            let baseline =
                bounds.y + line_index as f64 * self.size * 1.5 + face.ascender() as f64 * scale;
            for glyph in line {
                let mut path = Path {
                    data: String::new(),
                    x: left + glyph.x,
                    y: baseline - glyph.y,
                    scale,
                };
                if face
                    .outline_glyph(ttf_parser::GlyphId(glyph.id), &mut path)
                    .is_some()
                {
                    write!(out, "<path d=\"{}\"/>", path.data).unwrap();
                }
                if out.len() > 8 * 1024 * 1024 {
                    return Err(Error::new("output_limit", "Glyph paths exceed SVG budget"));
                }
            }
        }
        Ok(out)
    }
}
struct Path {
    data: String,
    x: f64,
    y: f64,
    scale: f64,
}
impl Path {
    fn pair(&self, x: f32, y: f32) -> (f64, f64) {
        (
            crate::model::q(self.x + x as f64 * self.scale),
            crate::model::q(self.y - y as f64 * self.scale),
        )
    }
}
impl OutlineBuilder for Path {
    fn move_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.pair(x, y);
        write!(self.data, "M{x} {y}").unwrap();
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.pair(x, y);
        write!(self.data, "L{x} {y}").unwrap();
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (x1, y1) = self.pair(x1, y1);
        let (x, y) = self.pair(x, y);
        write!(self.data, "Q{x1} {y1} {x} {y}").unwrap();
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (x1, y1) = self.pair(x1, y1);
        let (x2, y2) = self.pair(x2, y2);
        let (x, y) = self.pair(x, y);
        write!(self.data, "C{x1} {y1} {x2} {y2} {x} {y}").unwrap();
    }
    fn close(&mut self) {
        self.data.push('Z');
    }
}
