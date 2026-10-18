//! Pele sequence/colors.ts CSS color recognition, 458322a5 (MIT, kepano).
use regex::Regex;
use std::sync::OnceLock;
const KEYWORDS:&str="aliceblue antiquewhite aqua aquamarine azure beige bisque black blanchedalmond blue blueviolet brown burlywood cadetblue chartreuse chocolate coral cornflowerblue cornsilk crimson cyan darkblue darkcyan darkgoldenrod darkgray darkgreen darkgrey darkkhaki darkmagenta darkolivegreen darkorange darkorchid darkred darksalmon darkseagreen darkslateblue darkslategray darkslategrey darkturquoise darkviolet deeppink deepskyblue dimgray dimgrey dodgerblue firebrick floralwhite forestgreen fuchsia gainsboro ghostwhite gold goldenrod gray green greenyellow grey honeydew hotpink indianred indigo ivory khaki lavender lavenderblush lawngreen lemonchiffon lightblue lightcoral lightcyan lightgoldenrodyellow lightgray lightgreen lightgrey lightpink lightsalmon lightseagreen lightskyblue lightslategray lightslategrey lightsteelblue lightyellow lime limegreen linen magenta maroon mediumaquamarine mediumblue mediumorchid mediumpurple mediumseagreen mediumslateblue mediumspringgreen mediumturquoise mediumvioletred midnightblue mintcream mistyrose moccasin navajowhite navy oldlace olive olivedrab orange orangered orchid palegoldenrod palegreen paleturquoise palevioletred papayawhip peachpuff peru pink plum powderblue purple rebeccapurple red rosybrown royalblue saddlebrown salmon sandybrown seagreen seashell sienna silver skyblue slateblue slategray slategrey snow springgreen steelblue tan teal thistle tomato turquoise violet wheat white whitesmoke yellow yellowgreen transparent currentcolor inherit initial unset revert accentcolor accentcolortext activetext buttonborder buttonface buttontext canvas canvastext field fieldtext graytext highlight highlighttext linktext mark marktext selecteditem selecteditemtext visitedtext activeborder activecaption appworkspace background buttonhighlight buttonshadow captiontext inactiveborder inactivecaption inactivecaptiontext infobackground infotext menu menutext scrollbar threeddarkshadow threedface threedhighlight threedlightshadow threedshadow window windowframe windowtext";
#[derive(Clone, Copy, PartialEq)]
enum K {
    Bad,
    Number,
    Percent,
    Angle,
    None,
}
fn kind(term: &str) -> K {
    if term.eq_ignore_ascii_case("none") {
        return K::None;
    }
    static RE: OnceLock<Regex> = OnceLock::new();
    let Some(m)=RE.get_or_init(||Regex::new(r"(?i)^[+-]?(?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)(?:e[+-]?[0-9]+)?(%|deg|grad|rad|turn)?$").unwrap()).captures(term)else{return K::Bad};
    match m.get(1).map(|m| m.as_str()) {
        None => K::Number,
        Some("%") => K::Percent,
        _ => K::Angle,
    }
}
pub(super) fn is_css_color(color: &str) -> bool {
    if !color.contains('(') {
        return KEYWORDS.split(' ').any(|s| s.eq_ignore_ascii_case(color));
    }
    let hsl = color.starts_with("hsl");
    let open = color.find('(').unwrap();
    if !matches!(&color[..open], "rgb" | "rgba" | "hsl" | "hsla") {
        return false;
    }
    if color.encode_utf16().count() > 120 || color.find(')') != Some(color.len() - 1) {
        return false;
    }
    let inner = color[open + 1..color.len() - 1].trim_matches(crate::compat::source::space);
    let legacy = inner.contains(',');
    let (mut parts, alpha) = if legacy {
        let mut parts: Vec<_> = inner
            .split(',')
            .map(|s| s.trim_matches(crate::compat::source::space))
            .collect();
        let alpha = if parts.len() == 4 { parts.pop() } else { None };
        (parts, alpha)
    } else {
        let slash: Vec<_> = inner.split('/').collect();
        if slash.len() > 2 {
            return false;
        }
        let parts = slash[0]
            .trim_matches(crate::compat::source::space)
            .split(crate::compat::source::space)
            .filter(|s| !s.is_empty())
            .collect();
        (
            parts,
            slash
                .get(1)
                .map(|s| s.trim_matches(crate::compat::source::space)),
        )
    };
    if parts.len() != 3 {
        return false;
    }
    if let Some(alpha) = alpha {
        let k = kind(alpha);
        if !(matches!(k, K::Number | K::Percent) || !legacy && k == K::None) {
            return false;
        }
    }
    let kinds: Vec<_> = parts.drain(..).map(kind).collect();
    if legacy {
        if hsl {
            return matches!(kinds[0], K::Number | K::Angle)
                && kinds[1] == K::Percent
                && kinds[2] == K::Percent;
        }
        return matches!(kinds[0], K::Number | K::Percent)
            && kinds[1] == kinds[0]
            && kinds[2] == kinds[0];
    }
    kinds.iter().enumerate().all(|(i, k)| {
        matches!(k, K::Number | K::None)
            || if hsl && i == 0 {
                *k == K::Angle
            } else {
                *k == K::Percent
            }
    })
}
