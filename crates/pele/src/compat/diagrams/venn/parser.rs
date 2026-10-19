//! Venn grammar and DB calls, Pele 458322a5 (MIT).
use super::{
    lexer::{Lexer, NAMES, T},
    model::{semantic, VennModel},
};
use crate::compat::PeleError;
const STATEMENT: &str = "'EOF', 'NEWLINE', 'TITLE', 'SET', 'UNION', 'TEXT', 'INDENT_TEXT', 'STYLE'";
const VALUE: &str = "'NUMERIC', 'IDENTIFIER', 'STRING', 'HEXCOLOR', 'RGBCOLOR', 'RGBACOLOR'";
struct Parser<'a> {
    lex: Lexer<'a>,
    kind: Option<T>,
    db: VennModel,
}
impl Parser<'_> {
    fn peek(&mut self) -> T {
        if let Some(t) = self.kind {
            t
        } else {
            let t = self.lex.next(&mut self.db);
            self.kind = Some(t);
            t
        }
    }
    fn take(&mut self) -> String {
        self.kind = None;
        self.lex.text.clone()
    }
    fn fail(&mut self, wanted: &str) -> PeleError {
        let t = self.peek();
        PeleError::syntax(
            "venn",
            self.lex.src,
            self.lex.start,
            &format!("Expecting {wanted}, got '{}'", NAMES[t as usize]),
            t == T::Error,
        )
    }
    fn expect(&mut self, t: T) -> Result<String, PeleError> {
        if self.peek() != t {
            return Err(self.fail(&format!("'{}'", NAMES[t as usize])));
        }
        Ok(self.take())
    }
    fn id(&mut self) -> Result<String, PeleError> {
        if !matches!(self.peek(), T::Identifier | T::String) {
            return Err(self.fail("'IDENTIFIER', 'STRING'"));
        }
        Ok(self.take())
    }
    fn ids(&mut self) -> Result<Vec<String>, PeleError> {
        let mut v = vec![self.id()?];
        while self.peek() == T::Comma {
            self.take();
            v.push(self.id()?)
        }
        Ok(v)
    }
    fn subset(&mut self, ids: Vec<String>, union: bool) -> Result<(), PeleError> {
        let label = if self.peek() == T::BracketLabel {
            Some(self.take())
        } else {
            None
        };
        let size = if self.peek() == T::Colon {
            self.take();
            let text = self.expect(T::Numeric)?;
            Some(text.parse().unwrap_or(if text.starts_with('-') {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            }))
        } else {
            None
        };
        if union {
            if ids.len() < 2 {
                return Err(semantic("union requires multiple identifiers"));
            }
            self.db.validate_union(&ids)?;
        }
        self.db.subset(ids, label, size);
        self.db.indent = true;
        Ok(())
    }
    fn text(&mut self, sets: Option<Vec<String>>) -> Result<(), PeleError> {
        let kind = self.peek();
        if !matches!(kind, T::Identifier | T::String | T::Numeric) {
            return Err(self.fail("'NUMERIC', 'IDENTIFIER', 'STRING'"));
        }
        let id = self.take();
        let label = if kind != T::Numeric && self.peek() == T::BracketLabel {
            Some(self.take())
        } else {
            None
        };
        let sets = sets
            .or_else(|| self.db.current.clone())
            .ok_or_else(|| semantic("text requires set"))?;
        self.db.text(sets, id, label);
        Ok(())
    }
    fn style(&mut self) -> Result<(), PeleError> {
        let ids = self.ids()?;
        let mut styles = vec![];
        loop {
            let key = self.expect(T::Identifier)?;
            self.expect(T::Colon)?;
            let value = if self.peek() == T::String {
                self.take()
            } else {
                let mut parts = vec![];
                while matches!(
                    self.peek(),
                    T::Identifier | T::Numeric | T::HexColor | T::RgbColor | T::RgbaColor
                ) {
                    parts.push(self.take());
                }
                if parts.is_empty() {
                    return Err(self.fail(VALUE));
                }
                parts.join(" ")
            };
            styles.push((key, value));
            if self.peek() != T::Comma {
                break;
            }
            self.take();
        }
        self.db.style(ids, styles);
        Ok(())
    }
}
pub(super) fn parse(src: &str, title: Option<&str>) -> Result<VennModel, PeleError> {
    let mut p = Parser {
        lex: Lexer::new(src),
        kind: None,
        db: VennModel::new(title),
    };
    while p.peek() == T::Newline {
        p.take();
    }
    p.expect(T::Venn)?;
    loop {
        let s = p.peek();
        if s == T::Eof {
            return Ok(p.db);
        }
        if !matches!(
            s,
            T::Newline | T::Title | T::Set | T::Union | T::Text | T::IndentText | T::Style
        ) {
            return Err(p.fail(STATEMENT));
        }
        let text = p.take();
        match s {
            T::Title => p.db.title = Some(text.chars().skip(6).collect()),
            T::Set => {
                let id = p.id()?;
                p.subset(vec![id], false)?
            }
            T::Union => {
                let ids = p.ids()?;
                p.subset(ids, true)?
            }
            T::Text => {
                let ids = p.ids()?;
                p.text(Some(ids))?
            }
            T::IndentText => p.text(None)?,
            T::Style => p.style()?,
            _ => {}
        }
    }
}
