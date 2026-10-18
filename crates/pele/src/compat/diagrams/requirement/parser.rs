//! Pele requirement/parser.ts, 458322a5 (MIT, kepano). Right-recursive fields
//! are collected and applied backwards, preserving first-field and duplicate-node rules.
use super::{
    lexer::{tokenize, Token},
    model::{Db, Relation, RequirementModel},
    tokens::{NAMES, T},
};
use crate::compat::{source::space, ParseContext, PeleError};
const VALUES: [&str; 21] = [
    "Requirement",
    "Functional Requirement",
    "Interface Requirement",
    "Performance Requirement",
    "Physical Requirement",
    "Design Constraint",
    "Low",
    "Medium",
    "High",
    "Analysis",
    "Demonstration",
    "Inspection",
    "Test",
    "",
    "contains",
    "copies",
    "derives",
    "satisfies",
    "verifies",
    "refines",
    "traces",
];
fn part(t: T) -> bool {
    matches!(t, T::ALPHA | T::COLON | T::BRKT | T::MINUS | T::SEMICOLON)
}
struct Reader<'a> {
    src: &'a str,
    ts: Vec<Token>,
    i: usize,
    db: Db,
}
impl Reader<'_> {
    fn t(&self) -> T {
        self.ts.get(self.i).map_or(T::END, |t| t.kind)
    }
    fn take(&mut self) -> String {
        let s = self.ts[self.i].text.clone();
        self.i += 1;
        s
    }
    fn fail(&self, from: T, to: T, others: &[T]) -> PeleError {
        let t = &self.ts[self.i.min(self.ts.len() - 1)];
        let expected = (from as usize..=to as usize)
            .chain(others.iter().map(|t| *t as usize))
            .map(|t| format!("'{}'", NAMES[t]))
            .collect::<Vec<_>>()
            .join(", ");
        PeleError::syntax(
            "requirement",
            self.src,
            t.start,
            &format!("Expecting {expected}, got '{}'", NAMES[t.kind as usize]),
            t.kind == T::ERROR,
        )
    }
    fn expect(&mut self, t: T) -> Result<String, PeleError> {
        if self.t() != t {
            return Err(self.fail(t, t, &[]));
        }
        Ok(self.take())
    }
    fn keyword(&mut self, from: T, to: T) -> Result<String, PeleError> {
        let t = self.t();
        if t < from || t > to {
            return Err(self.fail(from, to, &[]));
        }
        self.i += 1;
        Ok(VALUES[t as usize - T::REQUIREMENT as usize].into())
    }
    fn string(&mut self) -> Result<String, PeleError> {
        if !matches!(self.t(), T::unqString | T::qString) {
            return Err(self.fail(T::qString, T::unqString, &[]));
        }
        Ok(self.take())
    }
    fn directive(&mut self) -> Result<bool, PeleError> {
        let t = self.t();
        match t {
            T::acc_title => {
                self.i += 1;
                let s = self.expect(T::acc_title_value)?;
                self.db.model.acc_title =
                    Some(s.trim_matches(space).trim_start_matches(space).into());
            }
            T::acc_descr | T::acc_descr_multiline_value => {
                let s = if t == T::acc_descr {
                    self.i += 1;
                    self.expect(T::acc_descr_value)?
                } else {
                    self.take()
                };
                let s = s.trim_matches(space);
                let mut out = String::new();
                let mut chars = s.chars().peekable();
                while let Some(c) = chars.next() {
                    out.push(c);
                    if c == '\n' {
                        while chars.peek().is_some_and(|c| space(*c)) {
                            chars.next();
                        }
                    }
                }
                self.db.model.acc_descr = Some(out);
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
    fn ids(&mut self) -> Result<Vec<String>, PeleError> {
        let mut out = vec![];
        loop {
            if !matches!(self.t(), T::ALPHA | T::unqString | T::qString) {
                return Err(self.fail(T::qString, T::unqString, &[T::ALPHA]));
            }
            out.push(self.take());
            if self.t() != T::COMMA {
                return Ok(out);
            }
            self.i += 1;
        }
    }
    fn styles(&mut self) -> Result<Vec<String>, PeleError> {
        let mut out = vec![];
        loop {
            if !part(self.t()) {
                return Err(self.fail(T::ALPHA, T::SEMICOLON, &[T::MINUS, T::BRKT]));
            }
            let mut style = self.take();
            while part(self.t()) {
                style.push_str(&self.take())
            }
            out.push(style);
            if self.t() != T::COMMA {
                return Ok(out);
            }
            self.i += 1;
        }
    }
    fn classes(&mut self) -> Result<Option<Vec<String>>, PeleError> {
        let classes = if self.t() == T::STYLE_SEPARATOR {
            self.i += 1;
            let list = self.ids()?;
            self.expect(T::STRUCT_START)?;
            Some(list)
        } else if self.t() == T::STRUCT_START {
            self.i += 1;
            None
        } else {
            return Err(self.fail(T::STRUCT_START, T::STRUCT_START, &[T::STYLE_SEPARATOR]));
        };
        self.expect(T::NEWLINE)?;
        Ok(classes)
    }
    fn body(&mut self, requirement: bool) -> Result<(), PeleError> {
        let mut fields = vec![];
        loop {
            let t = self.t();
            if t == T::STRUCT_STOP {
                self.i += 1;
                break;
            }
            if t == T::NEWLINE {
                self.i += 1;
                continue;
            }
            if if requirement {
                t < T::ID || t > T::VERIFYMTHD
            } else {
                !matches!(t, T::TYPE | T::DOCREF)
            } {
                return Err(if requirement {
                    self.fail(T::ID, T::VERIFYMTHD, &[T::NEWLINE, T::STRUCT_STOP])
                } else {
                    self.fail(T::TYPE, T::DOCREF, &[T::NEWLINE, T::STRUCT_STOP])
                });
            }
            self.i += 1;
            self.expect(T::COLONSEP)?;
            let value = if t == T::RISK {
                self.keyword(T::LOW_RISK, T::HIGH_RISK)?
            } else if t == T::VERIFYMTHD {
                self.keyword(T::VERIFY_ANALYSIS, T::VERIFY_TEST)?
            } else {
                self.string()?
            };
            fields.push((t, value));
            self.expect(T::NEWLINE)?;
        }
        for (t, v) in fields.into_iter().rev() {
            match t {
                T::ID => self.db.new_id = v,
                T::TEXT => self.db.new_text = v,
                T::RISK => self.db.new_risk = v,
                T::VERIFYMTHD => self.db.new_verify = v,
                T::TYPE => self.db.new_type = v,
                _ => self.db.new_doc_ref = v,
            }
        }
        Ok(())
    }
    fn parse(mut self) -> Result<RequirementModel, PeleError> {
        while self.directive()? {
            if self.t() == T::NEWLINE {
                self.i += 1
            }
        }
        if self.t() != T::RD {
            return Err(self.fail(
                T::RD,
                T::RD,
                &[T::acc_title, T::acc_descr, T::acc_descr_multiline_value],
            ));
        }
        self.i += 1;
        self.expect(T::NEWLINE)?;
        loop {
            let t = self.t();
            if t == T::EOF {
                return Ok(self.db.model);
            }
            if t == T::NEWLINE {
                self.i += 1;
            } else if self.directive()? {
                continue;
            } else if (T::direction_tb..=T::direction_lr).contains(&t) {
                self.i += 1;
                self.db.model.direction = match t {
                    T::direction_tb => "TB",
                    T::direction_bt => "BT",
                    T::direction_rl => "RL",
                    _ => "LR",
                }
                .into();
            } else if (T::REQUIREMENT..=T::DESIGN_CONSTRAINT).contains(&t) {
                let kind = self.keyword(T::REQUIREMENT, T::DESIGN_CONSTRAINT)?;
                let name = self.string()?;
                let classes = self.classes()?;
                self.body(true)?;
                self.db.requirement(name.clone(), kind);
                if let Some(classes) = classes {
                    self.db.set_class(vec![name], classes)?;
                }
            } else if t == T::ELEMENT {
                self.i += 1;
                let name = self.string()?;
                let classes = self.classes()?;
                self.body(false)?;
                self.db.element(name.clone());
                if let Some(classes) = classes {
                    self.db.set_class(vec![name], classes)?;
                }
            } else if matches!(t, T::STYLE | T::CLASSDEF | T::CLASS) {
                self.i += 1;
                let ids = self.ids()?;
                if t == T::CLASS {
                    let names = self.ids()?;
                    self.db.set_class(ids, names)?;
                } else {
                    let styles = self.styles()?;
                    if t == T::STYLE {
                        self.db.set_style(ids, styles)?
                    } else {
                        self.db.define(ids, styles)?
                    }
                }
            } else if matches!(t, T::unqString | T::qString) {
                let id = self.take();
                let next = self.t();
                if next == T::STYLE_SEPARATOR {
                    self.i += 1;
                    let classes = self.ids()?;
                    self.db.set_class(vec![id], classes)?;
                } else if matches!(next, T::END_ARROW_L | T::LINE) {
                    self.i += 1;
                    let kind = self.keyword(T::CONTAINS, T::TRACES)?;
                    self.expect(if next == T::LINE {
                        T::END_ARROW_R
                    } else {
                        T::LINE
                    })?;
                    let other = self.string()?;
                    let (src, dst) = if next == T::LINE {
                        (id, other)
                    } else {
                        (other, id)
                    };
                    self.db.model.relations.push(Relation {
                        relation_type: kind,
                        src,
                        dst,
                    });
                } else {
                    return Err(self.fail(
                        T::STYLE_SEPARATOR,
                        T::STYLE_SEPARATOR,
                        &[T::END_ARROW_L, T::LINE],
                    ));
                }
            } else {
                return Err(self.fail(
                    T::NEWLINE,
                    T::EOF,
                    &[
                        T::acc_title,
                        T::acc_descr,
                        T::acc_descr_multiline_value,
                        T::direction_tb,
                        T::direction_bt,
                        T::direction_rl,
                        T::direction_lr,
                        T::REQUIREMENT,
                        T::FUNCTIONAL_REQUIREMENT,
                        T::INTERFACE_REQUIREMENT,
                        T::PERFORMANCE_REQUIREMENT,
                        T::PHYSICAL_REQUIREMENT,
                        T::DESIGN_CONSTRAINT,
                        T::ELEMENT,
                        T::CLASSDEF,
                        T::CLASS,
                        T::STYLE,
                        T::unqString,
                        T::qString,
                    ],
                ));
            }
        }
    }
}
pub(super) fn parse(c: ParseContext<'_>) -> Result<RequirementModel, PeleError> {
    Reader {
        src: c.source,
        ts: tokenize(c.source),
        i: 0,
        db: Db::new(c.title),
    }
    .parse()
}
