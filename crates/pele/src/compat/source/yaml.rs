//! Translation of Pele src/util/yaml.ts, commit 458322a5 (MIT, kepano).
//! Keep the accepted JSON-schema subset and error behavior; this is not a general YAML parser.
use super::detect::space;
use crate::compat::value::{Object, Value};
type Result<T> = std::result::Result<T, Failure>;
#[derive(Debug)]
struct Failure {
    message: String,
    incomplete: bool,
}
impl Failure {
    fn error(s: impl Into<String>) -> Self {
        Self {
            message: s.into(),
            incomplete: false,
        }
    }
    fn incomplete(s: &str) -> Self {
        Self {
            message: s.into(),
            incomplete: true,
        }
    }
}
fn fail<T>(s: &str) -> Result<T> {
    Err(Failure::error(s))
}
fn trim(s: &str) -> &str {
    s.trim_matches(space)
}
fn trim_start(s: &str) -> &str {
    s.trim_start_matches(space)
}
fn leading_space_units(s: &str) -> usize {
    s.chars().take_while(|c| space(*c)).count()
}
fn after_units(s: &str, n: usize) -> &str {
    &s[s.char_indices().nth(n).map_or(s.len(), |(i, _)| i)..]
}
fn blank(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r')
}
struct Reader {
    lines: Vec<String>,
    i: usize,
}
impl Reader {
    fn new(src: &str) -> Self {
        Self {
            lines: src
                .replace("\r\n", "\n")
                .replace('\r', "\n")
                .split('\n')
                .map(str::to_owned)
                .collect(),
            i: 0,
        }
    }
    fn skip_blank(&mut self) {
        while self.i < self.lines.len() {
            let t = trim(&self.lines[self.i]);
            if !t.is_empty() && !t.starts_with('#') && t != "---" {
                break;
            }
            self.i += 1
        }
    }
    fn indent(&self) -> Result<usize> {
        let b = self.lines[self.i].as_bytes();
        let k = b.iter().take_while(|c| **c == b' ').count();
        if b.get(k) == Some(&b'\t') {
            return fail("YAML: tabs are not allowed for indentation");
        }
        Ok(k)
    }
}
fn split_key(line: &str) -> Option<(&str, usize)> {
    let b = line.as_bytes();
    let first = *b.first()?;
    let (end, colon);
    if first == b'"' || first == b'\'' {
        let mut i = 1;
        loop {
            let c = *b.get(i)?;
            if first == b'"' && c == b'\\' {
                i += 2
            } else if c == first {
                if first == b'"' || b.get(i + 1) != Some(&b'\'') {
                    break;
                }
                i += 2
            } else {
                i += 1
            }
        }
        end = i + 1;
        let mut c = end;
        while b.get(c).is_some_and(|c| blank(*c)) {
            c += 1
        }
        if b.get(c) != Some(&b':') {
            return None;
        }
        colon = c;
    } else {
        if line
            .chars()
            .next()
            .is_some_and(|c| space(c) || "#:,[]{}&*!|>'\"%@`".contains(c))
        {
            return None;
        }
        if first == b'-'
            && (b.len() < 2
                || b.get(1)
                    .is_some_and(|c| blank(*c) || matches!(c, b':' | b'#')))
        {
            return None;
        }
        let mut c = 1;
        while c < b.len() && b[c] != b':' && b[c] != b'#' {
            c += 1
        }
        if b.get(c) != Some(&b':') {
            return None;
        }
        colon = c;
        let mut e = c;
        while e > 0 && blank(b[e - 1]) {
            e -= 1
        }
        end = e;
    }
    let mut length = colon + 1;
    if length < b.len() && !blank(b[length]) {
        return None;
    }
    while b.get(length).is_some_and(|c| blank(*c)) {
        length += 1
    }
    Some((&line[..end], length))
}
fn scalar(raw: &str) -> Value {
    let s = trim(raw);
    match s {
        "" | "~" | "null" | "Null" | "NULL" => return Value::Null,
        "true" | "True" | "TRUE" => return Value::Bool(true),
        "false" | "False" | "FALSE" => return Value::Bool(false),
        _ => {}
    }
    let unsigned = s.strip_prefix(['-', '+']).unwrap_or(s);
    for (prefix, radix) in [("0x", 16), ("0o", 8), ("0b", 2)] {
        if let Some(digits) = unsigned.strip_prefix(prefix) {
            if !digits.is_empty() && digits.chars().all(|c| c.is_digit(radix)) {
                let v = digits.chars().fold(0.0, |n, c| {
                    n * (radix as f64) + c.to_digit(radix).unwrap() as f64
                });
                return Value::Number(if s.starts_with('-') { -v } else { v });
            }
        }
    }
    if matches!(unsigned, ".inf" | ".Inf" | ".INF") {
        return Value::Number(if s.starts_with('-') {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        });
    }
    if matches!(s, ".nan" | ".NaN" | ".NAN") {
        return Value::Number(f64::NAN);
    }
    // JS RE_FLOAT: decimal digits with optional dot and exponent; no Rust inf/NaN spellings.
    let b = unsigned.as_bytes();
    let mut p = 0;
    while b.get(p).is_some_and(u8::is_ascii_digit) {
        p += 1
    }
    let mut digits = p;
    if b.get(p) == Some(&b'.') {
        p += 1;
        let start = p;
        while b.get(p).is_some_and(u8::is_ascii_digit) {
            p += 1
        }
        digits += p - start
    }
    if digits > 0 && b.get(p).is_some_and(|c| matches!(c, b'e' | b'E')) {
        p += 1;
        if b.get(p).is_some_and(|c| matches!(c, b'-' | b'+')) {
            p += 1
        }
        let start = p;
        while b.get(p).is_some_and(u8::is_ascii_digit) {
            p += 1
        }
        if start == p {
            return Value::String(s.into());
        }
    }
    if digits > 0 && p == b.len() {
        if let Ok(n) = s.parse::<f64>() {
            return Value::Number(n);
        }
    }
    Value::String(s.into())
}
fn fold(text: &str) -> String {
    if !text.contains('\n') {
        return text.into();
    }
    let raw: Vec<_> = text.split('\n').collect();
    let last = raw.len() - 1;
    let lines: Vec<_> = raw
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let l = if i > 0 {
                l.trim_start_matches([' ', '\t'])
            } else {
                l
            };
            if i < last {
                l.trim_end_matches([' ', '\t'])
            } else {
                l
            }
        })
        .collect();
    let joined = lines.join("\n");
    let mut out = String::new();
    let mut run = 0;
    for c in joined.chars() {
        if c == '\n' {
            run += 1
        } else {
            if run > 0 {
                if run == 1 {
                    out.push(' ')
                } else {
                    out.push_str(&"\n".repeat(run - 1))
                }
                run = 0
            }
            out.push(c)
        }
    }
    if run > 0 {
        if run == 1 {
            out.push(' ')
        } else {
            out.push_str(&"\n".repeat(run - 1))
        }
    }
    out
}
fn unquote_double(body: &str) -> Result<String> {
    let mut joined = String::new();
    let mut it = body.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\\' && it.peek() == Some(&'\n') {
            it.next();
            while it.peek().is_some_and(|c| matches!(c, ' ' | '\t')) {
                it.next();
            }
        } else {
            joined.push(c)
        }
    }
    let s = fold(&joined);
    let chars: Vec<_> = s.chars().collect();
    let mut out = Vec::<u16>::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        i += 1;
        if c != '\\' {
            out.extend(c.encode_utf16(&mut [0; 2]).iter().copied());
            continue;
        }
        let e = chars.get(i).copied().unwrap_or('\0');
        i += 1;
        if matches!(e, 'x' | 'u' | 'U') {
            let len = if e == 'x' {
                2
            } else if e == 'u' {
                4
            } else {
                8
            };
            let hex: String = chars[i..(i + len).min(chars.len())].iter().collect();
            let valid: String = hex.chars().take_while(char::is_ascii_hexdigit).collect();
            let code = u32::from_str_radix(&valid, 16).ok();
            let Some(code) = code.filter(|n| *n <= 0x10ffff) else {
                return fail(&format!(
                    "Invalid code point {}",
                    code.map_or("NaN".into(), |n| n.to_string())
                ));
            };
            if code <= 0xffff {
                out.push(code as u16)
            } else {
                let c = char::from_u32(code).unwrap();
                out.extend(c.encode_utf16(&mut [0; 2]).iter().copied())
            }
            i += len;
        } else {
            let c = match e {
                '0' => '\0',
                'a' => '\x07',
                'b' => '\x08',
                't' => '\t',
                'n' => '\n',
                'v' => '\x0b',
                'f' => '\x0c',
                'r' => '\r',
                'e' => '\x1b',
                ' ' => ' ',
                '"' => '"',
                '/' => '/',
                '\\' => '\\',
                'N' => '\u{85}',
                '_' => '\u{a0}',
                'L' => '\u{2028}',
                'P' => '\u{2029}',
                _ => return fail("YAML: unknown escape sequence"),
            };
            out.extend(c.encode_utf16(&mut [0; 2]).iter().copied());
        }
    }
    Ok(String::from_utf16_lossy(&out))
}
fn skip(s: &str, mut p: usize) -> usize {
    while p < s.len() {
        let c = s.as_bytes()[p];
        if blank(c) {
            p += 1
        } else if c == b'#' && (p == 0 || s[..p].chars().next_back().is_some_and(space)) {
            p = s[p..].find('\n').map_or(s.len(), |n| p + n)
        } else {
            break;
        }
    }
    p
}
fn entry_start(c: u8, separated: bool) -> Result<()> {
    if !separated {
        return fail("YAML: missed comma between flow collection entries");
    }
    if c == b',' {
        return fail("YAML: expected the node content, but found ','");
    }
    Ok(())
}
fn flow(s: &str, mut p: usize, stops: &str, depth: usize) -> Result<(Value, usize)> {
    if depth > 64 {
        return fail("YAML: nesting is too deep");
    }
    p = skip(s, p);
    let b = s.as_bytes();
    let ch = b.get(p).copied();
    if ch == Some(b'{') {
        let mut out = Object::default();
        p += 1;
        let mut separated = true;
        loop {
            p = skip(s, p);
            let Some(&c) = b.get(p) else {
                return Err(Failure::incomplete(
                    "YAML: unexpected end of the stream within a flow collection",
                ));
            };
            if c == b'}' {
                return Ok((Value::Object(out), p + 1));
            }
            entry_start(c, separated)?;
            let (key, end) = flow(s, p, ":,}", depth + 1)?;
            p = skip(s, end);
            let mut value = Value::Null;
            if b.get(p) == Some(&b':') {
                (value, p) = flow(s, p + 1, ",}", depth + 1)?
            }
            out.insert(key.js_string(), value);
            p = skip(s, p);
            separated = b.get(p) == Some(&b',');
            if separated {
                p += 1
            }
        }
    }
    if ch == Some(b'[') {
        let mut out = Vec::new();
        p += 1;
        let mut separated = true;
        loop {
            p = skip(s, p);
            let Some(&c) = b.get(p) else {
                return Err(Failure::incomplete(
                    "YAML: unexpected end of the stream within a flow collection",
                ));
            };
            if c == b']' {
                return Ok((Value::Array(out), p + 1));
            }
            entry_start(c, separated)?;
            let (value, end) = flow(s, p, ",]", depth + 1)?;
            p = skip(s, end);
            out.push(value);
            separated = b.get(p) == Some(&b',');
            if separated {
                p += 1
            }
        }
    }
    if ch == Some(b'"') {
        let mut end = p + 1;
        while end < b.len() && b[end] != b'"' {
            end += if b[end] == b'\\' { 2 } else { 1 }
        }
        if end >= b.len() {
            return Err(Failure::incomplete(
                "YAML: unexpected end of the stream within a double quoted scalar",
            ));
        }
        return Ok((Value::String(unquote_double(&s[p + 1..end])?), end + 1));
    }
    if ch == Some(b'\'') {
        let mut end = p + 1;
        while end < b.len() {
            if b[end] == b'\'' {
                if b.get(end + 1) != Some(&b'\'') {
                    break;
                }
                end += 1
            }
            end += 1
        }
        if end >= b.len() {
            return Err(Failure::incomplete(
                "YAML: unexpected end of the stream within a single quoted scalar",
            ));
        }
        return Ok((
            Value::String(fold(&s[p + 1..end]).replace("''", "'")),
            end + 1,
        ));
    }
    let mut end = p;
    while end < s.len() {
        let c = s[end..].chars().next().unwrap();
        if stops.contains(c)
            && (c != ':'
                || s[end + 1..]
                    .chars()
                    .next()
                    .is_none_or(|c| space(c) || ",[]{}".contains(c)))
        {
            break;
        }
        if c == '#' && end > p && s[..end].chars().next_back().is_some_and(space) {
            let nl = s[end..].find('\n').map_or(s.len(), |i| end + i);
            return Ok((scalar(&fold(trim(&s[p..end]))), nl));
        }
        end += c.len_utf8()
    }
    Ok((scalar(&fold(trim(&s[p..end]))), end))
}
fn strip_comment(s: &str) -> &str {
    let b = s.as_bytes();
    let mut quote = 0;
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if quote != 0 {
            if c == b'\\' && quote == b'"' {
                i += 1
            } else if c == quote {
                quote = 0
            }
        } else if c == b'"' || c == b'\'' {
            quote = c
        } else if c == b'#' && (i == 0 || matches!(b[i - 1], b' ' | b'\t')) {
            return &s[..i];
        }
        i += 1
    }
    s
}
fn block_scalar(r: &mut Reader, header: &str, parent: isize) -> String {
    let folded = header.starts_with('>');
    let chomp = if header.contains('-') {
        '-'
    } else if header.contains('+') {
        '+'
    } else {
        ' '
    };
    let mut indent = header
        .chars()
        .find(|c| matches!(c, '1'..='9'))
        .map_or(-1, |c| parent + c.to_digit(10).unwrap() as isize);
    let mut out = Vec::new();
    while r.i < r.lines.len() {
        let line = &r.lines[r.i];
        if trim(line).is_empty() {
            out.push(String::new());
            r.i += 1;
            continue;
        }
        let k = leading_space_units(line) as isize;
        if indent == -1 {
            if k <= parent {
                break;
            }
            indent = k
        }
        if k < indent {
            break;
        }
        out.push(after_units(line, indent as usize).to_string());
        r.i += 1
    }
    let mut trailing = 0;
    while out.last().is_some_and(String::is_empty) {
        out.pop();
        trailing += 1
    }
    let mut text = String::new();
    if folded {
        for (i, cur) in out.iter().enumerate() {
            if i > 0 {
                let prev = &out[i - 1];
                if cur.is_empty() {
                    text.push('\n')
                } else if !prev.is_empty() {
                    text.push(
                        if prev.starts_with([' ', '\t']) || cur.starts_with([' ', '\t']) {
                            '\n'
                        } else {
                            ' '
                        },
                    )
                }
            }
            text.push_str(cur)
        }
    } else {
        text = out.join("\n")
    }
    if out.is_empty() {
        return if chomp == '+' {
            "\n".repeat(trailing)
        } else {
            String::new()
        };
    }
    match chomp {
        '-' => text,
        '+' => text + &"\n".repeat(trailing + 1),
        _ => text + "\n",
    }
}
fn inline_value(r: &mut Reader, rest: &str, indent: isize) -> Result<Value> {
    let first = rest.chars().next().unwrap_or('\0');
    if matches!(first, '|' | '>') {
        let tail = &rest[1..];
        let lead = tail.trim_start_matches(|c| matches!(c, '-' | '+' | '0'..='9'));
        let end = lead.trim_start_matches(space);
        if end.is_empty() || end.starts_with('#') {
            return Ok(Value::String(block_scalar(r, rest, indent)));
        }
    }
    if matches!(first, '{' | '[' | '"' | '\'') {
        let start = r.i;
        let mut text = rest.to_string();
        let mut taken = 0;
        let mut batch = 1;
        loop {
            match flow(&text, 0, "", 0) {
                Ok((value, end)) => {
                    let used = text[..end].bytes().filter(|c| *c == b'\n').count();
                    let line_end = text[end..].find('\n').map_or(text.len(), |i| end + i);
                    if !trim(strip_comment(&text[end..line_end])).is_empty() {
                        return fail("YAML: unexpected content after a value");
                    }
                    r.i = start + used;
                    return Ok(value);
                }
                Err(e) => {
                    if !e.incomplete || start + taken >= r.lines.len() {
                        return Err(e);
                    }
                    let to = (start + taken + batch).min(r.lines.len());
                    text.push('\n');
                    text.push_str(&r.lines[start + taken..to].join("\n"));
                    taken = to - start;
                    batch = batch.saturating_mul(2);
                }
            }
        }
    }
    let mut text = trim(strip_comment(rest)).to_string();
    while r.i < r.lines.len() {
        let line = &r.lines[r.i];
        if trim(line).is_empty() {
            let mut j = r.i;
            while j < r.lines.len() && trim(&r.lines[j]).is_empty() {
                j += 1
            }
            if j >= r.lines.len() || leading_space_units(&r.lines[j]) as isize <= indent {
                break;
            }
            text.push_str(&"\n".repeat(j - r.i));
            r.i = j;
            continue;
        }
        if leading_space_units(line) as isize <= indent {
            break;
        }
        let more = trim(strip_comment(line));
        if !text.ends_with('\n') {
            text.push(' ')
        }
        text.push_str(more);
        r.i += 1
    }
    Ok(scalar(&text))
}
fn block(r: &mut Reader, min_indent: usize, depth: usize) -> Result<Value> {
    if depth > 64 {
        return fail("YAML: nesting is too deep");
    }
    r.skip_blank();
    if r.i >= r.lines.len() {
        return Ok(Value::Null);
    }
    let indent = r.indent()?;
    if indent < min_indent {
        return Ok(Value::Null);
    }
    let line = r.lines[r.i][indent..].to_string();
    if line == "-" || line.starts_with("- ") {
        let mut out = Vec::new();
        while r.i < r.lines.len() {
            r.skip_blank();
            if r.i >= r.lines.len() || r.indent()? != indent {
                break;
            }
            let cur = &r.lines[r.i][indent..];
            if cur != "-" && !cur.starts_with("- ") {
                break;
            }
            let rest = cur[1..].to_string();
            let pad = leading_space_units(&rest) + 1;
            let body = trim(strip_comment(&rest));
            if body.is_empty() {
                r.i += 1;
                out.push(block(r, indent + 1, depth + 1)?)
            } else if split_key(body).is_some() {
                r.lines[r.i] = " ".repeat(indent + pad) + trim_start(&rest);
                out.push(block(r, indent + pad, depth + 1)?)
            } else {
                r.i += 1;
                out.push(inline_value(r, trim(&rest), indent as isize)?)
            }
        }
        return Ok(Value::Array(out));
    }
    if split_key(&line).is_none() {
        r.i += 1;
        return inline_value(r, trim(&line), indent as isize - 1);
    }
    let mut out = Object::default();
    while r.i < r.lines.len() {
        r.skip_blank();
        if r.i >= r.lines.len() {
            break;
        }
        let k = r.indent()?;
        if k < indent {
            break;
        }
        if k > indent {
            return fail("YAML: bad indentation of a mapping entry");
        }
        let cur = r.lines[r.i][indent..].to_string();
        let Some((key, length)) = split_key(&cur) else {
            return fail("YAML: can not read a block mapping entry");
        };
        let key = flow(key, 0, "", 0)?.0.js_string();
        let rest = &cur[length..];
        r.i += 1;
        let value = if trim(strip_comment(rest)).is_empty() {
            r.skip_blank();
            if r.i < r.lines.len() {
                let next = r.indent()?;
                let next_line = &r.lines[r.i][next..];
                let seq = next_line == "-" || next_line.starts_with("- ");
                if next > indent || seq && next == indent {
                    block(r, if seq { indent } else { indent + 1 }, depth + 1)?
                } else {
                    Value::Null
                }
            } else {
                Value::Null
            }
        } else {
            inline_value(r, trim(rest), indent as isize)?
        };
        out.insert(key, value)
    }
    Ok(Value::Object(out))
}
pub(crate) fn parse(src: &str) -> std::result::Result<Value, String> {
    let run = || -> Result<Value> {
        let trimmed = trim(src);
        if trimmed.starts_with(['{', '[']) {
            let (value, end) = flow(trimmed, 0, "", 0)?;
            if skip(trimmed, end) == trimmed.len() {
                return Ok(value);
            }
        }
        let mut r = Reader::new(src);
        let value = block(&mut r, 0, 0)?;
        r.skip_blank();
        if r.i < r.lines.len() && trim(&r.lines[r.i]) != "..." {
            return fail("YAML: end of the stream or a document separator is expected");
        }
        Ok(value)
    };
    run().map_err(|e| e.message)
}
