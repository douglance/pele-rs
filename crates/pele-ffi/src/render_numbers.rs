//! Version9 native IEEE number transport. Existing exports keep their dispatch.
use super::{calendar, compatibility_input, public, PeleHostResourcesV3, PeleResult};
use pele::compat::{ErrorCode, OutputLimit, PeleError, RenderOptions};
use serde::{
    de::{
        self,
        value::{MapAccessDeserializer, StringDeserializer},
        DeserializeSeed, IgnoredAny, MapAccess, Visitor,
    },
    Deserialize,
};

pub const PELE_RENDER_NUMBER_FONT_SIZE_V9: u64 = 1 << 0;
pub const PELE_RENDER_NUMBER_MAX_WIDTH_V9: u64 = 1 << 1;
pub const PELE_RENDER_NUMBER_DIRECTION_BREAKPOINT_V9: u64 = 1 << 2;
pub const PELE_RENDER_NUMBER_PADDING_V9: u64 = 1 << 3;
pub const PELE_RENDER_NUMBER_NOW_V9: u64 = 1 << 4;
const KNOWN_NUMBERS: u64 = 0x1f;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PeleRenderNumbersV9 {
    pub version: u32,
    pub struct_size: u32,
    pub present: u64,
    pub font_size_bits: u64,
    pub max_width_bits: u64,
    pub direction_breakpoint_bits: u64,
    pub padding_bits: u64,
    pub now_bits: u64,
}
#[derive(Clone, Copy, Default)]
pub(super) struct ValidatedRenderNumbers {
    present: u64,
    values: [u64; 5],
}
unsafe fn packet(numbers: *const PeleRenderNumbersV9) -> Result<ValidatedRenderNumbers, PeleError> {
    if numbers.is_null() {
        return Ok(ValidatedRenderNumbers::default());
    }
    // Read only the fixed eight-byte header before trusting the declared size.
    let header = numbers.cast::<u32>();
    let version = unsafe { std::ptr::read_unaligned(header) };
    let size = unsafe { std::ptr::read_unaligned(header.add(1)) };
    if version != 9 {
        return Err(public::invalid(
            "Native render numbers: unsupported version",
        ));
    }
    if size != 56 {
        return Err(public::invalid(
            "Native render numbers: invalid structure size",
        ));
    }
    let numbers = unsafe { std::ptr::read_unaligned(numbers) };
    if numbers.present & !KNOWN_NUMBERS != 0 {
        return Err(public::invalid(
            "Native render numbers: unknown numeric mask",
        ));
    }
    Ok(ValidatedRenderNumbers {
        present: numbers.present,
        values: [
            numbers.font_size_bits,
            numbers.max_width_bits,
            numbers.direction_breakpoint_bits,
            numbers.padding_bits,
            numbers.now_bits,
        ],
    })
}
fn selected(key: &str) -> u64 {
    match key {
        "fontSize" => PELE_RENDER_NUMBER_FONT_SIZE_V9,
        "maxWidth" => PELE_RENDER_NUMBER_MAX_WIDTH_V9,
        "directionBreakpoint" => PELE_RENDER_NUMBER_DIRECTION_BREAKPOINT_V9,
        "padding" => PELE_RENDER_NUMBER_PADDING_V9,
        "now" => PELE_RENDER_NUMBER_NOW_V9,
        "outputLimit" => 1 << 5,
        _ => 0,
    }
}
struct FilteredMap<A> {
    inner: A,
    mask: u64,
    seen: u64,
}
impl<'de, A: MapAccess<'de>> MapAccess<'de> for FilteredMap<A> {
    type Error = A::Error;
    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        loop {
            let Some(key) = self.inner.next_key::<String>()? else {
                return Ok(None);
            };
            let bit = selected(&key);
            if self.mask & bit != 0 {
                if self.seen & bit != 0 {
                    return Err(de::Error::custom(format!("duplicate field `{key}`")));
                }
                self.seen |= bit;
                self.inner.next_value::<IgnoredAny>()?;
            } else {
                return seed
                    .deserialize(StringDeserializer::<A::Error>::new(key))
                    .map(Some);
            }
        }
    }
    fn next_value_seed<V: DeserializeSeed<'de>>(
        &mut self,
        seed: V,
    ) -> Result<V::Value, Self::Error> {
        self.inner.next_value_seed(seed)
    }
}
struct FilterOptions {
    mask: u64,
}
impl<'de> DeserializeSeed<'de> for FilterOptions {
    type Value = RenderOptions;
    fn deserialize<D: de::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<RenderOptions, D::Error> {
        struct OptionsVisitor {
            mask: u64,
        }
        impl<'de> Visitor<'de> for OptionsVisitor {
            type Value = RenderOptions;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("render options object")
            }
            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<RenderOptions, A::Error> {
                RenderOptions::deserialize(MapAccessDeserializer::new(FilteredMap {
                    inner: map,
                    mask: self.mask,
                    seen: 0,
                }))
            }
        }
        deserializer.deserialize_map(OptionsVisitor { mask: self.mask })
    }
}
fn render_number_options(
    json: &str,
    numbers: ValidatedRenderNumbers,
    output: Option<OutputLimit>,
) -> Result<RenderOptions, PeleError> {
    if numbers.present == 0 && output.is_none() {
        return pele::compat::render_options(json);
    }
    let mask = numbers.present | if output.is_some() { 1 << 5 } else { 0 };
    let mut options = if json.trim().is_empty() {
        RenderOptions::default()
    } else {
        let mut decoder = serde_json::Deserializer::from_str(json);
        let value = FilterOptions { mask }
            .deserialize(&mut decoder)
            .and_then(|value| decoder.end().map(|_| value));
        value.map_err(|error| {
            PeleError::new(
                format!("Invalid render options: {error}"),
                ErrorCode::Semantic,
            )
        })?
    };
    let fields = [
        &mut options.render.font_size,
        &mut options.render.max_width,
        &mut options.render.direction_breakpoint,
        &mut options.render.padding,
        &mut options.render.now,
    ];
    for (index, field) in fields.into_iter().enumerate() {
        if numbers.present & (1 << index) != 0 {
            *field = Some(f64::from_bits(numbers.values[index]));
        }
    }
    Ok(options)
}
pub(super) fn check_lengths(
    options_len: usize,
    source_len: Option<usize>,
) -> Result<(), PeleError> {
    if options_len > 1024 * 1024
        || source_len.is_some_and(|len| len > pele::compat::MAX_PUBLIC_SOURCE_BYTES)
    {
        return Err(PeleError::new(
            "Native transport: declared input exceeds budget",
            ErrorCode::Limit,
        ));
    }
    Ok(())
}
pub(super) unsafe fn options(
    bytes: *const u8,
    len: usize,
    numbers: *const PeleRenderNumbersV9,
    kind: u32,
    bits: u64,
) -> Result<(RenderOptions, Option<OutputLimit>), PeleError> {
    check_lengths(len, None)?;
    let numbers = unsafe { packet(numbers) }?;
    let limit = match kind {
        0 => Some(OutputLimit::Undefined),
        1 => Some(OutputLimit::Null),
        2 => Some(OutputLimit::Number(f64::from_bits(bits))),
        3 => None,
        _ => {
            return Err(public::invalid(
                "Native public transport: unknown output-limit kind",
            ))
        }
    };
    let json = unsafe { compatibility_input(bytes, len, 1024 * 1024) }?;
    Ok((render_number_options(json, numbers, limit)?, limit))
}
/// # Safety
/// Inputs, optional packet and callbacks remain readable for the synchronous call.
#[no_mangle]
pub unsafe extern "C" fn pele_render_with_numbers_v9(
    source: *const u8,
    source_len: usize,
    bytes: *const u8,
    len: usize,
    numbers: *const PeleRenderNumbersV9,
    kind: u32,
    bits: u64,
    resources: *const PeleHostResourcesV3,
) -> *mut PeleResult {
    public::bounded_call(|| {
        check_lengths(len, Some(source_len))?;
        let (options, limit) = unsafe { options(bytes, len, numbers, kind, bits) }?;
        let source = unsafe {
            compatibility_input(source, source_len, pele::compat::MAX_PUBLIC_SOURCE_BYTES)
        }?;
        let result = unsafe {
            calendar::with_resources(resources, |host| {
                pele::compat::Engine::with_builtins()
                    .render_source_bounded(source, &options, limit, host)
            })
        }?;
        public::success(pele::compat::render_result_transport(&result))
    })
}
/// # Safety
/// Document is a live result; packet/options/resources are synchronously borrowed.
#[no_mangle]
pub unsafe extern "C" fn pele_render_document_with_numbers_v9(
    document: *const PeleResult,
    bytes: *const u8,
    len: usize,
    numbers: *const PeleRenderNumbersV9,
    kind: u32,
    bits: u64,
    resources: *const PeleHostResourcesV3,
) -> *mut PeleResult {
    public::bounded_call(|| {
        let (options, limit) = unsafe { options(bytes, len, numbers, kind, bits) }?;
        let document = unsafe { document.as_ref() }
            .and_then(|value| value.document.as_ref())
            .ok_or_else(|| {
                public::invalid("render() takes diagram text or a model that parse() returned.")
            })?;
        let result = unsafe {
            calendar::with_resources(resources, |host| {
                pele::compat::Engine::empty()
                    .render_document_bounded(document, &options, limit, host)
            })
        }?;
        public::success(pele::compat::render_result_transport(&result))
    })
}
