//! Shared Langium-style token behavior, Pele common/tokens.ts (458322a5, MIT).
use crate::compat::source::space;

// /---[\t ]*\r?\n(?:[\S\s]*?\r?\n)?---(?:\r?\n|(?!\S))/y
// The optional body is greedy: try a nonempty body before the empty alternative.
pub(super) fn hidden_yaml_end(source: &str, start: usize) -> Option<usize> {
    if !source[start..].starts_with("---") {
        return None;
    }
    let mut open = start + 3;
    while source
        .as_bytes()
        .get(open)
        .is_some_and(|c| matches!(c, b' ' | b'\t'))
    {
        open += 1;
    }
    let body = if source[open..].starts_with("\r\n") {
        open + 2
    } else if source[open..].starts_with('\n') {
        open + 1
    } else {
        return None;
    };
    let closer = |at: usize| {
        if !source[at..].starts_with("---") {
            return None;
        }
        let after = at + 3;
        if source[after..].starts_with("\r\n") {
            Some(after + 2)
        } else if source[after..].starts_with('\n') {
            Some(after + 1)
        } else if source[after..].chars().next().is_none_or(space) {
            Some(after)
        } else {
            None
        }
    };
    let mut at = body;
    while let Some(next) = source[at..].find('\n').map(|i| at + i + 1) {
        if let Some(end) = closer(next) {
            return Some(end);
        }
        at = next;
    }
    closer(body)
}
