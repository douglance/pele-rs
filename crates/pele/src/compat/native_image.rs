//! Explicit host-owned raster substitution. The portable default never fetches resources.
use super::{ErrorCode, PeleError};
use std::sync::Arc;
pub const MAX_NATIVE_IMAGE_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_NATIVE_IMAGE_PIXELS: u64 = 16_777_216;
pub const MAX_NATIVE_IMAGE_AXIS: u32 = 8192;
pub const MAX_NATIVE_IMAGE_TOTAL_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_NATIVE_IMAGE_COUNT: usize = 128;
#[derive(Clone, Debug)]
pub enum ImageResource {
    /// Preserve the engine-approved URI. Existing hosts retain their SVG contract.
    Passthrough,
    /// Self-contained PNG/JPEG data. No SVG, file or network dependency.
    Embedded(EmbeddedImage),
}
#[derive(Clone, Debug)]
pub struct EmbeddedImage {
    data_uri: Arc<str>,
    encoded_len: usize,
    width: u32,
    height: u32,
}
impl EmbeddedImage {
    /// Validates bounded raster headers; native adapters additionally validate with
    /// their platform decoder before submitting bytes. This is not a pixel decoder.
    pub fn from_encoded(
        bytes: &[u8],
        max_bytes: usize,
        max_pixels: u64,
    ) -> Result<Self, PeleError> {
        if bytes.len() > max_bytes.min(MAX_NATIVE_IMAGE_BYTES) {
            return Err(failure("Native image exceeds encoded byte budget", true));
        }
        let (mime, width, height) = dimensions(bytes)?;
        if width == 0
            || height == 0
            || width > MAX_NATIVE_IMAGE_AXIS
            || height > MAX_NATIVE_IMAGE_AXIS
            || u64::from(width) * u64::from(height) > max_pixels.min(MAX_NATIVE_IMAGE_PIXELS)
        {
            return Err(failure("Native image exceeds decoded pixel budget", true));
        }
        let mut data = String::with_capacity(32 + bytes.len().div_ceil(3) * 4);
        data.push_str("data:");
        data.push_str(mime);
        data.push_str(";base64,");
        const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        for chunk in bytes.chunks(3) {
            let n = (u32::from(chunk[0]) << 16)
                | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
                | u32::from(*chunk.get(2).unwrap_or(&0));
            data.push(ABC[((n >> 18) & 63) as usize] as char);
            data.push(ABC[((n >> 12) & 63) as usize] as char);
            data.push(if chunk.len() > 1 {
                ABC[((n >> 6) & 63) as usize] as char
            } else {
                '='
            });
            data.push(if chunk.len() > 2 {
                ABC[(n & 63) as usize] as char
            } else {
                '='
            });
        }
        Ok(Self {
            data_uri: data.into(),
            encoded_len: bytes.len(),
            width,
            height,
        })
    }
    pub fn data_uri(&self) -> &str {
        &self.data_uri
    }
    pub fn encoded_len(&self) -> usize {
        self.encoded_len
    }
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}
fn failure(message: &str, limit: bool) -> PeleError {
    PeleError::new(
        message,
        if limit {
            ErrorCode::Limit
        } else {
            ErrorCode::Semantic
        },
    )
}
fn dimensions(bytes: &[u8]) -> Result<(&'static str, u32, u32), PeleError> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        && bytes.len() >= 33
        && bytes[8..16] == [0, 0, 0, 13, b'I', b'H', b'D', b'R']
    {
        return Ok((
            "image/png",
            u32::from_be_bytes(bytes[16..20].try_into().unwrap()),
            u32::from_be_bytes(bytes[20..24].try_into().unwrap()),
        ));
    }
    if bytes.starts_with(&[255, 216]) {
        let mut i = 2;
        while i < bytes.len() {
            if bytes[i] != 255 {
                break;
            }
            while i < bytes.len() && bytes[i] == 255 {
                i += 1
            }
            if i >= bytes.len() {
                break;
            }
            let marker = bytes[i];
            i += 1;
            if marker == 217 || marker == 218 {
                break;
            }
            if marker == 1 || (208..=215).contains(&marker) {
                continue;
            }
            if i + 2 > bytes.len() {
                break;
            }
            let len = u16::from_be_bytes([bytes[i], bytes[i + 1]]) as usize;
            if len < 2 || len > bytes.len() - i {
                break;
            }
            if matches!(
                marker,
                192 | 193 | 194 | 195 | 197 | 198 | 199 | 201 | 202 | 203 | 205 | 206 | 207
            ) && len >= 8
            {
                return Ok((
                    "image/jpeg",
                    u32::from(u16::from_be_bytes([bytes[i + 5], bytes[i + 6]])),
                    u32::from(u16::from_be_bytes([bytes[i + 3], bytes[i + 4]])),
                ));
            }
            i += len;
        }
    }
    Err(failure(
        "Native image must contain a valid PNG or JPEG header",
        false,
    ))
}
