//! Real parse/render pipelines exercise the optional raster substitution boundary.
use pele::compat::{public::PublicEngine, *};
const SOURCE: &str =
    "flowchart LR\nA@{img: \"https://example.test/image.png\", label: \"Image\", w:80,h:60} --> B";
fn png() -> Vec<u8> {
    let mut p = vec![137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82];
    p.extend_from_slice(&1u32.to_be_bytes());
    p.extend_from_slice(&1u32.to_be_bytes());
    p.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0, 0]);
    p
}
#[derive(Default)]
struct Host {
    uris: Vec<String>,
    embedded: bool,
    fail: bool,
}
impl RenderResources for Host {
    fn text_width(&mut self, r: TextRequest<'_>) -> Result<f64, PeleError> {
        Ok(r.text.encode_utf16().count() as f64 * r.size * 0.6)
    }
    fn image(&mut self, uri: &str) -> Result<ImageResource, PeleError> {
        self.uris.push(uri.into());
        if self.fail {
            return Err(PeleError::new("image rejected", ErrorCode::Semantic));
        }
        if self.embedded {
            Ok(ImageResource::Embedded(EmbeddedImage::from_encoded(
                &png(),
                1024,
                1,
            )?))
        } else {
            Ok(ImageResource::Passthrough)
        }
    }
}
#[test]
fn source_retained_and_mutable_lanes_use_approved_embedded_images_without_changing_geometry() {
    let e = Engine::with_builtins();
    let d = e.parse(SOURCE, &ParseOptions::default()).unwrap();
    let opts = RenderOptions::default();
    let old = e
        .render_source(SOURCE, &opts, &mut Host::default())
        .unwrap();
    assert!(old
        .rendered
        .svg
        .contains("href=\"https://example.test/image.png\""));
    let mut h = Host {
        embedded: true,
        ..Default::default()
    };
    let got = e
        .render_document_bounded(&d, &opts, Some(OutputLimit::Undefined), &mut h)
        .unwrap();
    assert!(got.rendered.svg.contains("href=\"data:image/png;base64,"));
    assert_eq!(
        (got.rendered.width, got.rendered.height),
        (old.rendered.width, old.rendered.height)
    );
    assert_eq!(h.uris, ["https://example.test/image.png"]);
    let mut raw_host = Host {
        embedded: true,
        ..Default::default()
    };
    let raw = e
        .render_document_raw(&d, &Object::default(), &opts.render, &mut raw_host)
        .unwrap();
    assert!(raw.svg.contains("data:image/png;base64,"));
    assert_eq!(raw_host.uris, ["https://example.test/image.png"]);

    let mut d = PublicEngine::with_builtins()
        .parse(SOURCE, &ParseOptions::default())
        .unwrap();
    let mut h = Host {
        embedded: true,
        ..Default::default()
    };
    let mut capture = PresentationCapture::new(&mut h, PresentationBudget::default());
    let outcome = d
        .render_operation_bounded(0, &opts, Some(OutputLimit::Undefined), &mut capture)
        .unwrap();
    assert_eq!(outcome.revision, 1);
    assert_eq!(outcome.result.unwrap(), got);
    assert!(matches!(
        capture.outcome(),
        Some(PresentationOutcome::Available(_))
    ));
}
#[test]
fn uri_and_images_policy_runs_before_host_approval_and_failures_recover() {
    let e = Engine::with_builtins();
    let mut opts = RenderOptions::default();
    let mut h = Host {
        fail: true,
        ..Default::default()
    };
    opts.render.images = Some(false);
    e.render_source(SOURCE, &opts, &mut h).unwrap();
    assert!(h.uris.is_empty());
    opts.render.images = None;
    for uri in ["javascript:alert(1)", "file:///private/test.png"] {
        let source = SOURCE.replace("https://example.test/image.png", uri);
        e.render_source(&source, &opts, &mut h).unwrap();
    }
    assert!(h.uris.is_empty());
    let mut d = PublicEngine::with_builtins()
        .parse(SOURCE, &ParseOptions::default())
        .unwrap();
    let before = d.snapshot().unwrap();
    let out = d
        .render_operation_bounded(0, &opts, Some(OutputLimit::Undefined), &mut h)
        .unwrap();
    assert!(out.result.is_err());
    assert_eq!(out.revision, 1);
    assert_eq!(
        serde_json::to_value(out.graph).unwrap(),
        serde_json::to_value(before).unwrap()
    );
    h.fail = false;
    h.embedded = true;
    assert!(d
        .render_operation_bounded(1, &opts, Some(OutputLimit::Undefined), &mut h)
        .unwrap()
        .result
        .is_ok());
}
#[test]
fn encoded_raster_limits_reject_invalid_types_axes_and_pixels_through_public_construction() {
    for bytes in [b"<svg/>".to_vec(), vec![], vec![255, 216, 255, 217]] {
        assert!(EmbeddedImage::from_encoded(&bytes, 1024, 100).is_err());
    }
    assert_eq!(
        EmbeddedImage::from_encoded(&png(), 1, 100)
            .unwrap_err()
            .code,
        ErrorCode::Limit
    );
    assert_eq!(
        EmbeddedImage::from_encoded(&png(), 1024, 0)
            .unwrap_err()
            .code,
        ErrorCode::Limit
    );
    let mut p = png();
    p[16..20].copy_from_slice(&8193u32.to_be_bytes());
    assert_eq!(
        EmbeddedImage::from_encoded(&p, 1024, u64::MAX)
            .unwrap_err()
            .code,
        ErrorCode::Limit
    );
    let jpeg = [255, 216, 255, 192, 0, 8, 8, 0, 1, 0, 2, 1, 255, 217];
    let image = EmbeddedImage::from_encoded(&jpeg, 1024, 2).unwrap();
    assert_eq!(image.dimensions(), (2, 1));
    assert!(image.data_uri().starts_with("data:image/jpeg;base64,"));
}
