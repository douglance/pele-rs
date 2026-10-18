//! Exact ordered Ishikawa whitespace lexer and Jison-style parser, Pele 458322a5 (MIT).
use super::IshikawaModel;
use crate::compat::{
    source::{space, word},
    PeleError,
};
#[derive(Clone, Copy, PartialEq, Eq)]
enum T {
    End,
    Eof,
    SpaceLine,
    Nl,
    Ishikawa,
    SpaceList,
    Text,
}
const NAMES: &[&str] = &[
    "$end",
    "EOF",
    "SPACELINE",
    "NL",
    "ISHIKAWA",
    "SPACELIST",
    "TEXT",
];
struct Token {
    kind: T,
    from: usize,
    to: usize,
}
fn keyword(src: &str, at: usize) -> usize {
    let bytes = src.as_bytes();
    let expected = b"ishikawa-beta";
    let mut k = 0;
    while k < 13
        && bytes
            .get(at + k)
            .is_some_and(|c| c.to_ascii_lowercase() == expected[k])
    {
        k += 1;
    }
    let word_at = |p| {
        src.get(p..)
            .and_then(|s| s.chars().next())
            .is_some_and(word)
    };
    if k == 13 && !word_at(at + 13) {
        13
    } else if k >= 8 && !word_at(at + 8) {
        8
    } else {
        0
    }
}
fn tokenize(src: &str) -> Vec<Token> {
    let mut tokens = vec![];
    let mut i = 0;
    while i < src.len() {
        let mut j = i;
        let mut last = None;
        while let Some(c) = src[j..].chars().next().filter(|c| space(*c)) {
            if c == '\n' {
                last = Some(j);
            }
            j += c.len_utf8();
        }
        let (kind, end) = if src[j..].starts_with("%%") {
            let end = src[j + 2..]
                .find(['\n', '\r', '\u{2028}', '\u{2029}'])
                .map_or(src.len(), |k| j + 2 + k);
            (T::SpaceLine, end)
        } else if j == i {
            let length = keyword(src, i);
            if length > 0 {
                (T::Ishikawa, i + length)
            } else {
                (T::Text, src[i..].find('\n').map_or(src.len(), |k| i + k))
            }
        } else if last.is_some_and(|last| last > i) {
            (T::SpaceLine, last.unwrap() + 1)
        } else if last == Some(i) {
            (T::Nl, i + 1)
        } else {
            (T::SpaceList, j)
        };
        tokens.push(Token {
            kind,
            from: i,
            to: end,
        });
        i = end;
    }
    tokens.push(Token {
        kind: T::Eof,
        from: src.len(),
        to: src.len(),
    });
    tokens.push(Token {
        kind: T::End,
        from: src.len(),
        to: src.len(),
    });
    tokens
}
pub(super) fn parse(src: &str, title: Option<&str>) -> Result<IshikawaModel, PeleError> {
    let tokens = tokenize(src);
    let mut i = 0;
    let kind = |i: usize| tokens.get(i).map_or(T::End, |t| t.kind);
    let fail = |i: usize, expected: &str| {
        let token = tokens.get(i).unwrap_or_else(|| tokens.last().unwrap());
        PeleError::syntax(
            "ishikawa",
            src,
            token.from,
            &format!("Expecting {expected}, got '{}'", NAMES[token.kind as usize]),
            false,
        )
    };
    if kind(i) == T::SpaceLine {
        i += 1;
        while matches!(kind(i), T::SpaceLine | T::Nl) {
            i += 1;
        }
        if kind(i) != T::Ishikawa {
            return Err(fail(i, "'SPACELINE', 'NL', 'ISHIKAWA'"));
        }
    } else if kind(i) != T::Ishikawa {
        return Err(fail(i, "'SPACELINE', 'ISHIKAWA'"));
    }
    i += 1;
    if kind(i) == T::Nl {
        i += 1;
    }
    let mut model = IshikawaModel::new(title);
    const STOP: &str = "'SPACELINE', 'NL', 'EOF'";
    loop {
        let t = kind(i);
        if t == T::Text || (t == T::SpaceList && kind(i + 1) == T::Text) {
            let text = if t == T::Text { i } else { i + 1 };
            i = text + 1;
            if !matches!(kind(i), T::SpaceLine | T::Nl | T::Eof) {
                return Err(fail(i, STOP));
            }
            let level = if t == T::Text {
                0
            } else {
                src[tokens[text - 1].from..tokens[text - 1].to]
                    .encode_utf16()
                    .count()
            };
            model.add_node(
                level as f64,
                src[tokens[text].from..tokens[text].to]
                    .trim_matches(space)
                    .into(),
            );
        } else if matches!(t, T::SpaceLine | T::SpaceList) {
            i += 1;
            if !matches!(kind(i), T::SpaceLine | T::Nl | T::Eof) {
                return Err(fail(
                    i,
                    if t == T::SpaceList {
                        "'SPACELINE', 'NL', 'EOF', 'TEXT'"
                    } else {
                        STOP
                    },
                ));
            }
        } else {
            return Err(fail(i, "'SPACELINE', 'SPACELIST', 'TEXT'"));
        }
        i += 1;
        while matches!(kind(i), T::Nl | T::Eof) {
            i += 1;
        }
        if kind(i) == T::End {
            return Ok(model);
        }
    }
}
