//! Pele class/parser.ts recursive-descent translation, 458322a5 (MIT, kepano).
use super::{
    lexer::{tokenize, Token},
    model::*,
    tokens::{NAMES, T},
};
use crate::compat::{source::space, PeleError};
const NAME: &str = "'UNICODE_TEXT', 'NUM', 'ALPHA', 'MINUS', 'BQUOTE_STR'";
fn alpha(t: T) -> bool {
    matches!(t, T::UNICODE_TEXT | T::ALPHA | T::MINUS)
}
fn style(t: T) -> bool {
    matches!(
        t,
        T::ALPHA | T::COLON | T::BRKT | T::STYLE | T::PCT | T::LABEL
    )
}
fn end(t: T) -> Option<u8> {
    match t {
        T::AGGREGATION => Some(0),
        T::EXTENSION => Some(1),
        T::COMPOSITION => Some(2),
        T::DEPENDENCY => Some(3),
        T::LOLLIPOP => Some(4),
        _ => None,
    }
}
struct Reader<'a> {
    src: &'a str,
    tokens: Vec<Token>,
    i: usize,
    db: &'a mut ClassModel,
}
impl Reader<'_> {
    fn t(&self) -> T {
        self.tokens.get(self.i).map_or(T::END, |t| t.kind)
    }
    fn take(&mut self) -> String {
        let s = self.tokens[self.i].text.clone();
        self.i += 1;
        s
    }
    fn fail(&self, expected: &str) -> PeleError {
        let t = self
            .tokens
            .get(self.i)
            .unwrap_or_else(|| self.tokens.last().unwrap());
        PeleError::syntax(
            "class",
            self.src,
            t.start,
            &format!("Expecting {expected}, got '{}'", NAMES[t.kind as usize]),
            t.kind == T::ERROR,
        )
    }
    fn expect(&mut self, t: T) -> Result<String, PeleError> {
        if self.t() != t {
            return Err(self.fail(&format!("'{}'", NAMES[t as usize])));
        }
        Ok(self.take())
    }
    fn alpha_num(&mut self) -> Result<String, PeleError> {
        if !alpha(self.t()) {
            return Err(self.fail("'UNICODE_TEXT', 'NUM', 'ALPHA', 'MINUS'"));
        }
        Ok(self.take())
    }
    fn name(&mut self, generic: bool) -> Result<String, PeleError> {
        let mut out = String::new();
        loop {
            if self.t() == T::BQUOTE_STR {
                out += &self.take();
                break;
            }
            if !alpha(self.t()) {
                return Err(self.fail(NAME));
            }
            out += &self.take();
            if self.t() == T::DOT {
                out.push('.');
                self.i += 1
            } else if !alpha(self.t()) && self.t() != T::BQUOTE_STR {
                break;
            }
        }
        if generic && self.t() == T::GENERICTYPE {
            out += &format!("~{}~", self.take())
        }
        Ok(out)
    }
    fn class_label(&mut self) -> Result<String, PeleError> {
        self.i += 1;
        let label = self.expect(T::STR)?;
        self.expect(T::SQE)?;
        Ok(label)
    }
    fn members(&mut self, id: &str) -> Result<(), PeleError> {
        if self.t() != T::MEMBER {
            return Err(self.fail("'STRUCT_STOP', 'MEMBER'"));
        }
        let mut v = vec![];
        while self.t() == T::MEMBER {
            v.push(self.take())
        }
        self.expect(T::STRUCT_STOP)?;
        for m in v {
            self.db.add_member(id, &m)
        }
        Ok(())
    }
    fn class_statement(&mut self) -> Result<String, PeleError> {
        self.i += 1;
        let id = self.name(true)?;
        self.db.add_class(&id);
        if self.t() == T::SQS {
            let label = self.class_label()?;
            self.db.set_label(&id, label)
        }
        match self.t() {
            T::STYLE_SEPARATOR => {
                self.i += 1;
                let css = self.alpha_num()?;
                self.db.css_class(&id, &css);
                if self.t() == T::STRUCT_START {
                    self.i += 1;
                    self.members(&id)?
                }
            }
            T::STRUCT_START => {
                self.i += 1;
                if self.t() == T::STRUCT_STOP {
                    self.i += 1
                } else {
                    self.members(&id)?
                }
            }
            T::ANNOTATION_START => {
                self.i += 1;
                let a = self.alpha_num()?;
                self.expect(T::ANNOTATION_END)?;
                self.db.add_annotation(&id, a);
                if self.t() == T::STRUCT_START {
                    self.i += 1;
                    if self.t() == T::STRUCT_STOP {
                        self.i += 1
                    } else {
                        self.members(&id)?
                    }
                }
            }
            _ => {}
        }
        Ok(id)
    }
    fn note(&mut self) -> Result<String, PeleError> {
        let standalone = self.t() == T::NOTE;
        self.i += 1;
        let id = if standalone {
            None
        } else {
            Some(self.name(true)?)
        };
        let text = self.expect(T::STR)?;
        Ok(self.db.add_note(text, id))
    }
    fn namespace(&mut self) -> Result<(), PeleError> {
        let mut open: Vec<(String, Vec<String>, Vec<String>)> = vec![];
        loop {
            self.i += 1;
            let ns = self.name(false)?;
            let label = if self.t() == T::SQS {
                Some(self.class_label()?)
            } else {
                None
            };
            let id = self.db.add_namespace(&ns, label);
            self.expect(T::STRUCT_START)?;
            if self.t() == T::NEWLINE {
                self.i += 1
            }
            open.push((id, vec![], vec![]));
            loop {
                if self.t() == T::NAMESPACE {
                    break;
                }
                match self.t() {
                    T::CLASS => {
                        let id = self.class_statement()?;
                        open.last_mut().unwrap().1.push(id)
                    }
                    T::NOTE | T::NOTE_FOR => {
                        let id = self.note()?;
                        open.last_mut().unwrap().2.push(id)
                    }
                    _ => return Err(self.fail("'NAMESPACE', 'CLASS', 'NOTE_FOR', 'NOTE'")),
                }
                loop {
                    if self.t() == T::NEWLINE {
                        let next = self.tokens.get(self.i + 1).map_or(T::END, |t| t.kind);
                        self.i += 1;
                        if matches!(next, T::NAMESPACE | T::CLASS | T::NOTE | T::NOTE_FOR) {
                            break;
                        }
                    }
                    self.expect(T::STRUCT_STOP)?;
                    let (id, classes, notes) = open.pop().unwrap();
                    self.db.add_to_namespace(&id, classes, notes);
                    self.db.namespace_stack.pop();
                    if open.is_empty() {
                        return Ok(());
                    }
                }
            }
        }
    }
    fn relation_end(&mut self) -> RelationEnd {
        if let Some(n) = end(self.t()) {
            self.i += 1;
            RelationEnd::Number(n)
        } else {
            RelationEnd::none()
        }
    }
    fn relation(&mut self) -> Result<(), PeleError> {
        let id1 = self.name(true)?;
        let mut t = self.t();
        if t == T::LABEL {
            let label = self.take();
            self.db.add_member(
                &id1,
                label
                    .strip_prefix(':')
                    .unwrap_or(&label)
                    .trim_matches(space),
            );
            return Ok(());
        }
        let mut title1 = "none".to_string();
        if t == T::STR {
            title1 = self.take();
        } else if end(t).is_none() && !matches!(t, T::LINE | T::DOTTED_LINE) {
            return Ok(());
        }
        let type1 = self.relation_end();
        t = self.t();
        if !matches!(t, T::LINE | T::DOTTED_LINE) {
            return Err(self.fail("'AGGREGATION', 'EXTENSION', 'COMPOSITION', 'DEPENDENCY', 'LOLLIPOP', 'LINE', 'DOTTED_LINE'"));
        }
        self.i += 1;
        let type2 = self.relation_end();
        let title2 = if self.t() == T::STR {
            self.take()
        } else {
            "none".into()
        };
        let id2 = self.name(true)?;
        let title = if self.t() == T::LABEL {
            let s = self.take();
            Some(s.strip_prefix(':').unwrap_or(&s).trim_matches(space).into())
        } else {
            None
        };
        self.db.add_relation(ClassRelation {
            id1,
            id2,
            relation_title1: title1,
            relation_title2: title2,
            title,
            relation: RelationType {
                type1,
                type2,
                line_type: if t == T::LINE { 0 } else { 1 },
            },
        });
        Ok(())
    }
    fn styles(&mut self) -> Result<Vec<String>, PeleError> {
        let mut out = vec![];
        loop {
            if !style(self.t()) {
                return Err(self.fail(
                    "'ALPHA', 'NUM', 'COLON', 'UNIT', 'SPACE', 'BRKT', 'STYLE', 'PCT', 'LABEL'",
                ));
            }
            let mut s = self.take();
            while style(self.t()) {
                s += &self.take()
            }
            out.push(s);
            if self.t() != T::COMMA {
                return Ok(out);
            }
            self.i += 1
        }
    }
    fn tooltip(&mut self, id: &str) {
        if self.t() == T::STR {
            let text = self.take();
            self.db.tooltip(id, text)
        }
    }
    fn link(&mut self, id: &str) -> Result<(), PeleError> {
        let href = self.expect(T::STR)?;
        let tip = if self.t() == T::STR {
            Some(self.take())
        } else {
            None
        };
        let target = if self.t() == T::LINK_TARGET {
            Some(self.take())
        } else {
            None
        };
        self.db.link(id, &href, target)?;
        if let Some(t) = tip {
            self.db.tooltip(id, t)
        }
        Ok(())
    }
    fn statement(&mut self) -> Result<(), PeleError> {
        match self.t(){T::UNICODE_TEXT|T::ALPHA|T::MINUS|T::BQUOTE_STR=>self.relation(),T::NAMESPACE=>self.namespace(),T::CLASS=>self.class_statement().map(|_|()),T::MEMBER=>{self.i+=1;Ok(())},T::ANNOTATION_START=>{self.i+=1;let a=self.alpha_num()?;self.expect(T::ANNOTATION_END)?;let id=self.name(true)?;self.db.add_annotation(&id,a);Ok(())},T::CALLBACK=>{self.i+=1;let id=self.name(true)?;self.expect(T::STR)?;self.db.click(&id);self.tooltip(&id);Ok(())},T::LINK=>{self.i+=1;let id=self.name(true)?;self.link(&id)},T::CLICK=>{self.i+=1;let id=self.name(true)?;if self.t()==T::HREF{self.i+=1;return self.link(&id)}self.expect(T::CALLBACK_NAME)?;if self.t()==T::CALLBACK_ARGS{self.i+=1}self.db.click(&id);self.tooltip(&id);Ok(())},T::STYLE=>{self.i+=1;let id=self.expect(T::ALPHA)?;let s=self.styles()?;self.db.css_style(&id,s);Ok(())},T::CSSCLASS=>{self.i+=1;let ids=self.expect(T::STR)?;let css=self.expect(T::ALPHA)?;self.db.css_class(&ids,&css);Ok(())},T::NOTE|T::NOTE_FOR=>self.note().map(|_|()),T::CLASSDEF=>{self.i+=1;let mut ids=vec![self.expect(T::ALPHA)?];while self.t()==T::COMMA{self.i+=1;ids.push(self.expect(T::ALPHA)?)}let styles=self.styles()?;self.db.define_class(ids,styles);Ok(())},T::direction_tb|T::direction_bt|T::direction_rl|T::direction_lr=>{self.db.direction=match self.t(){T::direction_tb=>"TB",T::direction_bt=>"BT",T::direction_rl=>"RL",_=>"LR"}.into();self.i+=1;Ok(())},T::acc_title=>{self.i+=1;self.db.acc_title=Some(self.expect(T::acc_title_value)?.trim_matches(space).into());Ok(())},T::acc_descr|T::acc_descr_multiline_value=>{let multi=self.t()==T::acc_descr_multiline_value;if !multi{self.i+=1}let text=if multi{self.take()}else{self.expect(T::acc_descr_value)?};let text=text.trim_matches(space);let mut out=String::new();let mut skip=false;for c in text.chars(){if skip&&space(c){continue}skip=c=='\n';out.push(c)}self.db.acc_descr=Some(out);Ok(())},_=>Err(self.fail("'acc_title', 'acc_descr', 'acc_descr_multiline_value', 'NAMESPACE', 'ANNOTATION_START', 'CLASS', 'MEMBER', 'NOTE_FOR', 'NOTE', 'CLASSDEF', 'direction_tb', 'CALLBACK', 'LINK', 'CLICK', 'STYLE', 'CSSCLASS', 'ALPHA', 'MINUS', 'BQUOTE_STR'"))}
    }
}
pub fn parse(src: &str, db: &mut ClassModel) -> Result<(), PeleError> {
    let mut r = Reader {
        src,
        tokens: tokenize(src),
        i: 0,
        db,
    };
    let header = r.t() == T::CLASS_DIAGRAM;
    if header {
        r.i += 1;
        r.expect(T::NEWLINE)?;
    }
    loop {
        r.statement()?;
        if r.t() != T::NEWLINE {
            break;
        }
        r.i += 1;
        if matches!(r.t(), T::EOF | T::END) {
            break;
        }
    }
    if header {
        r.expect(T::EOF)?;
    }
    r.expect(T::END)?;
    Ok(())
}
