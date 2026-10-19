//! Pele common/generics.ts, 458322a5 (MIT, kepano).
fn tildes(s: &str) -> usize {
    s.bytes().filter(|c| *c == b'~').count()
}
fn pair(s: &str) -> String {
    let count = tildes(s);
    if count <= 1 {
        return s.into();
    }
    let lead = count % 2 != 0 && s.starts_with('~');
    let mut chars: Vec<char> = s.chars().collect();
    let at: Vec<usize> = chars
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            if *c == '~' && (!lead || i > 0) {
                Some(i)
            } else {
                None
            }
        })
        .collect();
    for i in 0..at.len() / 2 {
        chars[at[i]] = '<';
        chars[at[at.len() - 1 - i]] = '>';
    }
    chars.into_iter().collect()
}
pub fn parse_generic_types(input: &str) -> String {
    if !input.contains('~') {
        return input.into();
    }
    let mut sets = vec![];
    let mut start = 0;
    for (i, c) in input.char_indices() {
        if c == ',' {
            sets.push(&input[start..i]);
            sets.push(",");
            start = i + 1
        }
    }
    sets.push(&input[start..]);
    let mut out: Vec<String> = vec![];
    let mut i = 0;
    while i < sets.len() {
        if sets[i] == ","
            && i > 0
            && i + 1 < sets.len()
            && tildes(sets[i - 1]) == 1
            && tildes(sets[i + 1]) == 1
        {
            out.pop();
            out.push(pair(&format!("{},{}", sets[i - 1], sets[i + 1])));
            i += 2;
        } else {
            out.push(pair(sets[i]));
            i += 1
        }
    }
    out.join("")
}
