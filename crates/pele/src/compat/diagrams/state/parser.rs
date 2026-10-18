//! Pele state/parser.ts ordered public grammar, 458322a5 (MIT, kepano).
use super::{
    lexer::{tokenize, Tokens},
    model::{
        self, ClassDefStmt, ClickId, ClickStmt, Db, Description, DirectionStmt, RelationStmt,
        StateModel, StateNote, StateStmt, Stmt, StyleStmt,
    },
    tokens::{NAMES, T},
};
use crate::compat::{source::space, ParseContext, PeleError};
const STATEMENT:&str="'NL', 'ID', 'HIDE_EMPTY', 'scale', 'COMPOSIT_STATE', 'STATE_DESCR', 'FORK', 'JOIN', 'CHOICE', 'CONCURRENT', 'note', 'acc_title', 'acc_descr', 'acc_descr_multiline_value', 'CLICK', 'classDef', 'style', 'class', 'direction_tb', 'direction_bt', 'direction_rl', 'direction_lr', 'EDGE_STATE'";
struct Reader<'a> {
    src: &'a str,
    tokens: Tokens,
    i: usize,
    db: Db,
}
impl Reader<'_> {
    fn t(&self) -> T {
        self.tokens.list[self.i].kind
    }
    fn take(&mut self) -> String {
        let text = self.tokens.list[self.i].text.clone();
        self.i += 1;
        text
    }
    fn fail(&self, expected: &str) -> PeleError {
        let t = &self.tokens.list[self.i];
        if t.kind == T::ERROR {
            if let Some(message) = &self.tokens.error {
                let mut e = PeleError::syntax("state", self.src, t.start, "", true);
                e.message = message.clone();
                e
            } else {
                PeleError::syntax("state", self.src, t.start, "", true)
            }
        } else {
            PeleError::syntax(
                "state",
                self.src,
                t.start,
                &format!("Expecting {expected}, got '{}'", NAMES[t.kind as usize]),
                false,
            )
        }
    }
    fn expect(&mut self, t: T) -> Result<String, PeleError> {
        if self.t() != t {
            Err(self.fail(&format!("'{}'", NAMES[t as usize])))
        } else {
            Ok(self.take())
        }
    }
    fn id(&mut self) -> Result<StateStmt, PeleError> {
        if !matches!(self.t(), T::ID | T::EDGE_STATE) {
            return Err(self.fail("'ID', 'EDGE_STATE'"));
        }
        let id = self.take().trim_matches(space).into();
        let mut state = StateStmt::new(id, Some("default"));
        state.description = Some(Description::Text(String::new()));
        if self.t() == T::STYLE_SEPARATOR {
            self.i += 1;
            state.classes = Some(vec![self.expect(T::ID)?.trim_matches(space).into()])
        }
        Ok(state)
    }
}
pub(super) fn parse(context: ParseContext<'_>) -> Result<StateModel, PeleError> {
    let mut r = Reader {
        src: context.source,
        tokens: tokenize(context.source),
        i: 0,
        db: Db::new(&context),
    };
    while r.t() == T::NL {
        r.i += 1
    }
    if r.t() != T::SD {
        return Err(r.fail("'NL', 'SD'"));
    }
    r.i += 1;
    let mut doc = Vec::new();
    let mut outer: Vec<(Vec<Stmt>, StateStmt)> = Vec::new();
    loop {
        let item = match r.t() {
            T::NL => {
                r.i += 1;
                None
            }
            T::ID | T::EDGE_STATE => {
                let mut first = r.id()?;
                if r.t() == T::DESCR {
                    first.description = Some(Description::Text(model::trim_colon(&r.take())));
                    Some(Stmt::State(first))
                } else if r.t() == T::ARROW {
                    r.i += 1;
                    let second = r.id()?;
                    let description = if r.t() == T::DESCR {
                        Some(model::trim_colon(&r.take()))
                    } else {
                        None
                    };
                    Some(Stmt::Relation(RelationStmt {
                        stmt: "relation",
                        state1: first,
                        state2: second,
                        description,
                    }))
                } else {
                    Some(Stmt::State(first))
                }
            }
            T::HIDE_EMPTY => Some(Stmt::Text(r.take())),
            T::scale => {
                let text = r.take();
                r.expect(T::WIDTH)?;
                Some(Stmt::Text(text))
            }
            T::COMPOSIT_STATE => {
                let id = r.take();
                if r.t() == T::STRUCT_START {
                    r.i += 1;
                    let mut state = StateStmt::new(id, Some("default"));
                    state.description = Some(Description::Text(String::new()));
                    outer.push((doc, state));
                    doc = Vec::new();
                    None
                } else {
                    Some(Stmt::Text(id))
                }
            }
            T::STATE_DESCR => {
                let description = r.take();
                r.expect(T::AS)?;
                let id = r.expect(T::ID)?;
                if r.t() == T::STRUCT_START {
                    r.i += 1;
                    let mut state = StateStmt::new(id, Some("default"));
                    state.description = Some(Description::Text(description));
                    outer.push((doc, state));
                    doc = Vec::new();
                    None
                } else {
                    let mut state = if let Some((first, rest)) = id.split_once(':') {
                        let mut state = StateStmt::new(first.into(), Some("default"));
                        state.description = Some(Description::Lines(vec![
                            description.trim_matches(space).into(),
                            rest.split(':').next().unwrap_or("").into(),
                        ]));
                        state
                    } else {
                        let mut state = StateStmt::new(id, Some("default"));
                        state.description =
                            Some(Description::Text(description.trim_matches(space).into()));
                        state
                    };
                    state.stmt = "state";
                    Some(Stmt::State(state))
                }
            }
            t @ (T::FORK | T::JOIN | T::CHOICE) => {
                let id = r.take();
                Some(Stmt::State(StateStmt::new(
                    id,
                    Some(match t {
                        T::FORK => "fork",
                        T::JOIN => "join",
                        _ => "choice",
                    }),
                )))
            }
            T::CONCURRENT => {
                r.i += 1;
                Some(Stmt::State(StateStmt::new(r.db.divider(), Some("divider"))))
            }
            T::note => {
                let keyword = r.take();
                if matches!(r.t(), T::left_of | T::right_of) {
                    let position = r.take().trim_matches(space).to_string();
                    let id = r.expect(T::ID)?.trim_matches(space).into();
                    let text = r.expect(T::NOTE_TEXT)?.trim_matches(space).into();
                    let mut state = StateStmt::new(id, None);
                    state.note = Some(StateNote {
                        position: Some(position),
                        text,
                    });
                    Some(Stmt::State(state))
                } else {
                    if r.t() != T::NOTE_TEXT {
                        return Err(r.fail("'NOTE_TEXT', 'left_of', 'right_of'"));
                    }
                    r.i += 1;
                    r.expect(T::AS)?;
                    r.expect(T::ID)?;
                    Some(Stmt::Text(keyword))
                }
            }
            t @ (T::direction_tb | T::direction_bt | T::direction_rl | T::direction_lr) => {
                r.i += 1;
                Some(Stmt::Direction(DirectionStmt {
                    stmt: "dir",
                    value: ["TB", "BT", "RL", "LR"][t as usize - T::direction_tb as usize].into(),
                }))
            }
            T::acc_title => {
                r.i += 1;
                let value = r
                    .expect(T::acc_title_value)?
                    .trim_matches(space)
                    .to_string();
                r.db.acc_title(value.clone());
                Some(Stmt::Text(value))
            }
            T::acc_descr => {
                r.i += 1;
                let value = r
                    .expect(T::acc_descr_value)?
                    .trim_matches(space)
                    .to_string();
                r.db.acc_descr(value.clone());
                Some(Stmt::Text(value))
            }
            T::acc_descr_multiline_value => {
                let value = r.take().trim_matches(space).to_string();
                r.db.acc_descr(value.clone());
                Some(Stmt::Text(value))
            }
            T::CLICK => {
                r.i += 1;
                let id = r.id()?;
                let (url, tooltip) = if r.t() == T::HREF {
                    r.i += 1;
                    (r.expect(T::STRING)?, String::new())
                } else {
                    if r.t() != T::STRING {
                        return Err(r.fail("'STRING', 'HREF'"));
                    }
                    (r.take(), r.expect(T::STRING)?)
                };
                r.expect(T::NL)?;
                Some(Stmt::Click(ClickStmt {
                    stmt: "click",
                    id: ClickId::State(id),
                    url,
                    tooltip,
                }))
            }
            T::classDef => {
                r.i += 1;
                let id = r.expect(T::CLASSDEF_ID)?.trim_matches(space).into();
                Some(Stmt::ClassDef(ClassDefStmt {
                    stmt: "classDef",
                    id,
                    classes: r.expect(T::CLASSDEF_STYLEOPTS)?.trim_matches(space).into(),
                }))
            }
            T::style | T::class => {
                let style = r.t() == T::style;
                r.i += 1;
                let id = r
                    .expect(if style {
                        T::STYLE_IDS
                    } else {
                        T::CLASSENTITY_IDS
                    })?
                    .trim_matches(space)
                    .into();
                let style_class = r
                    .expect(if style {
                        T::STYLEDEF_STYLEOPTS
                    } else {
                        T::STYLECLASS
                    })?
                    .trim_matches(space)
                    .into();
                Some(Stmt::Style(StyleStmt {
                    stmt: if style { "style" } else { "applyClass" },
                    id,
                    style_class,
                }))
            }
            T::STRUCT_STOP => {
                let Some((parent, mut state)) = outer.pop() else {
                    return Err(r.fail(&format!("'$end', {STATEMENT}")));
                };
                r.i += 1;
                state.doc = Some(doc);
                doc = parent;
                Some(Stmt::State(state))
            }
            T::END => {
                if !outer.is_empty() {
                    return Err(r.fail(&format!("'STRUCT_STOP', {STATEMENT}")));
                }
                return r.db.finish(doc);
            }
            _ => {
                return Err(r.fail(&if !outer.is_empty() {
                    format!("'STRUCT_STOP', {STATEMENT}")
                } else {
                    format!("'$end', {STATEMENT}")
                }))
            }
        };
        if let Some(item) = item {
            if !matches!(&item,Stmt::Text(s)if s=="nl") {
                doc.push(item)
            }
        }
    }
}
