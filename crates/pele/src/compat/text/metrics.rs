//! Pele text/measurer.ts and text/metrics.ts, 458322a5 (MIT, kepano).
use crate::compat::{PeleError, RenderResources, TextRequest};
pub const REGULAR:&str="7q7q9vfgfgopij5b9999atg87q997q7qfgfgfgfgfgfgfgfgfgfg7q7qg8g8g8fgs7ijijk2k2ijgzlmk27qdwijfgn5k2lmijlmk2ijgzk2ijq8ijijgz7q7q7qd1fg99fgfgdwfgfg7qfgfg6666dw66n5fgfgfgfg99dw7qfgdwk2dwdwdw9a789ag8";
pub const BOLD:&str="7q99d6fgfgopk26m9999atg87q997q7qfgfgfgfgfgfgfgfgfgfg9999g8g8g8gzr3k2k2k2k2ijgzlmk27qfgk2gzn5k2lmijlmk2ijgzk2ijq8ijijgz997q99g8fg99fggzfggzfg99gzgz7q7qfg7qopgzgzgzgzatfg99gzfglmfgfgdwat7satg8";
#[derive(Default)]
pub struct MetricsResources;
impl RenderResources for MetricsResources {
    fn text_width(&mut self, request: TextRequest<'_>) -> Result<f64, PeleError> {
        Ok(width(request.text, request.size, request.style))
    }
}
pub fn width(text: &str, size: f64, style: u8) -> f64 {
    use std::sync::OnceLock;
    static TABLES: OnceLock<([u16; 95], [u16; 95])> = OnceLock::new();
    let (regular, bold) = TABLES.get_or_init(|| {
        fn unpack(s: &str) -> [u16; 95] {
            let mut table = [0; 95];
            for (i, v) in table.iter_mut().enumerate() {
                *v = u16::from_str_radix(&s[i * 2..i * 2 + 2], 36).unwrap()
            }
            table
        }
        (unpack(REGULAR), unpack(BOLD))
    });
    let mono = style & 4 != 0;
    let table = if style & 1 != 0 { bold } else { regular };
    let mut units = 0u64;
    for c in text.encode_utf16() {
        if c < 32 {
            continue;
        }
        if c < 127 {
            units += if mono {
                600
            } else {
                table[(c - 32) as usize] as u64
            }
        } else if matches!(c,0x300..=0x36f|0x200b..=0x200f|0xdc00..=0xdfff|0xfe00..=0xfe0f|0xfeff) {
            continue;
        } else if matches!(c,0x1100..=0x115f|0x2e80..=0xa4cf|0xac00..=0xd7a3|0xf900..=0xfaff|0xfe30..=0xfe4f|0xff00..=0xff60|0xffe0..=0xffe6|0xd800..=0xdbff)
        {
            units += if mono { 1200 } else { 1000 }
        } else {
            units += if mono { 600 } else { 580 }
        }
    }
    units as f64 * size / 1000.
}
