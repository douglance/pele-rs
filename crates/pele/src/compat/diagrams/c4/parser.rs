//! Pele c4/parser.ts ordered grammar/database calls, 458322a5 (MIT, kepano).
use super::{
    lexer::{tokenize, Token},
    model::{C4Attr, C4Model, C4Named, Db},
    tokens::{NAMES, T},
};
use crate::compat::{source::space, ParseContext, PeleError};
const KINDS: [&str; 20] = [
    "person",
    "external_person",
    "system",
    "system_db",
    "system_queue",
    "external_system",
    "external_system_db",
    "external_system_queue",
    "container",
    "container_db",
    "container_queue",
    "external_container",
    "external_container_db",
    "external_container_queue",
    "component",
    "component_db",
    "component_queue",
    "external_component",
    "external_component_db",
    "external_component_queue",
];
const REL_TYPES: [&str; 8] = [
    "rel", "birel", "rel_u", "rel_d", "rel_l", "rel_r", "rel_b", "rel",
];
const NODE_TYPES: [&str; 3] = ["node", "nodeL", "nodeR"];
const HEADERS:&str="'direction_tb', 'direction_bt', 'direction_rl', 'direction_lr', 'C4_CONTEXT', 'C4_CONTAINER', 'C4_COMPONENT', 'C4_DYNAMIC', 'C4_DEPLOYMENT'";
const ATTRIBUTES: &str = "'STR', 'STR_KEY', 'ATTRIBUTE', 'ATTRIBUTE_EMPTY'";
#[derive(Clone, Copy, PartialEq)]
enum P {
    Item,
    Diagram,
    ItemOrEnd,
    AfterOther,
    AfterDiagram,
}
fn other(t: T) -> bool {
    matches!(
        t,
        T::title | T::accDescription | T::acc_title | T::acc_descr | T::acc_descr_multiline_value
    )
}
fn diagram(t: T) -> bool {
    t >= T::PERSON && t <= T::UPDATE_LAYOUT_CONFIG
}
struct Reader<'a> {
    src: &'a str,
    tokens: Vec<Token>,
    i: usize,
    db: Db,
}
impl Reader<'_> {
    fn t(&self) -> T {
        self.tokens[self.i].kind
    }
    fn take(&mut self) -> String {
        let t = &self.tokens[self.i];
        self.i += 1;
        self.src[t.start..t.end].into()
    }
    fn fail(&self, expected: &str) -> PeleError {
        let t = &self.tokens[self.i];
        if t.kind == T::ERROR {
            PeleError::syntax("c4", self.src, t.start, "", true)
        } else {
            PeleError::syntax(
                "c4",
                self.src,
                t.start,
                &format!("Expecting {expected}, got '{}'", NAMES[t.kind as usize]),
                false,
            )
        }
    }
    fn expect(&mut self, t: T) -> Result<(), PeleError> {
        if self.t() != t {
            return Err(self.fail(&format!("'{}'", NAMES[t as usize])));
        }
        self.i += 1;
        Ok(())
    }
    fn attrs(&mut self) -> Result<Vec<C4Attr>, PeleError> {
        let mut attrs = Vec::new();
        loop {
            match self.t() {
                T::STR => attrs.push(C4Attr::Text(self.take().trim_matches(space).into())),
                T::ATTRIBUTE_EMPTY => {
                    attrs.push(C4Attr::Text(String::new()));
                    self.i += 1
                }
                T::STR_KEY => {
                    let key = self.take().trim_matches(space).into();
                    if self.t() != T::STR_VALUE {
                        return Err(self.fail("'STR_VALUE'"));
                    }
                    attrs.push(C4Attr::Named(C4Named {
                        key,
                        value: self.take().trim_matches(space).into(),
                    }))
                }
                _ => break,
            }
        }
        if attrs.is_empty() {
            return Err(self.fail(ATTRIBUTES));
        }
        Ok(attrs)
    }
    fn other(&mut self, t: T) -> Result<(), PeleError> {
        match t {
            T::title => {
                let text = self.take();
                self.db.model.title =
                    Some(text[5 + text[5..].chars().next().map_or(0, char::len_utf8)..].into())
            }
            T::accDescription => {
                let text = self.take();
                self.db.acc_descr(
                    text[14 + text[14..].chars().next().map_or(0, char::len_utf8)..].into(),
                )
            }
            T::acc_descr_multiline_value => {
                let text = self.take();
                self.db.acc_descr(text.trim_matches(space).into())
            }
            _ => {
                self.i += 1;
                let expected = if t == T::acc_title {
                    T::acc_title_value
                } else {
                    T::acc_descr_value
                };
                if self.t() != expected {
                    return Err(self.fail(&format!("'{}'", NAMES[expected as usize])));
                }
                let text = self.take().trim_matches(space).into();
                if t == T::acc_title {
                    self.db.model.title = Some(text)
                } else {
                    self.db.acc_descr(text)
                }
            }
        }
        Ok(())
    }
    fn diagram(&mut self, t: T) -> Result<bool, PeleError> {
        self.i += 1;
        let mut attrs = self.attrs()?;
        if t <= T::SYSTEM_EXT_QUEUE {
            self.db
                .add_shape(KINDS[t as usize - T::PERSON as usize], &attrs, false)
        } else if t <= T::COMPONENT_EXT_QUEUE {
            self.db
                .add_shape(KINDS[t as usize - T::PERSON as usize], &attrs, true)
        } else if t <= T::NODE_R {
            let typ = match t {
                T::ENTERPRISE_BOUNDARY => Some("ENTERPRISE"),
                T::SYSTEM_BOUNDARY => Some("SYSTEM"),
                T::CONTAINER_BOUNDARY => Some("CONTAINER"),
                _ => None,
            };
            if let Some(typ) = typ {
                attrs.insert(2.min(attrs.len()), C4Attr::Text(typ.into()))
            }
            if t >= T::NODE {
                self.db.boundary(
                    &attrs,
                    "node",
                    Some(NODE_TYPES[t as usize - T::NODE as usize]),
                )
            } else {
                self.db.boundary(
                    &attrs,
                    if t == T::CONTAINER_BOUNDARY {
                        "container"
                    } else {
                        "system"
                    },
                    None,
                )
            }
            if self.t() == T::LBRACE {
                self.i += 1;
                self.expect(T::NEWLINE)?
            } else if self.t() == T::NEWLINE {
                self.i += 1;
                self.expect(T::LBRACE)?;
                if self.t() == T::NEWLINE {
                    self.i += 1
                }
            } else {
                return Err(self.fail("'NEWLINE', 'LBRACE'"));
            }
            return Ok(true);
        } else if t <= T::REL_INDEX {
            if t == T::REL_INDEX {
                attrs.remove(0);
            }
            self.db.rel(REL_TYPES[t as usize - T::REL as usize], &attrs)
        } else if t == T::UPDATE_EL_STYLE {
            self.db.update_element(&attrs)
        } else if t == T::UPDATE_REL_STYLE {
            self.db.update_rel(&attrs)
        } else {
            self.db.update_layout(&attrs)
        }
        Ok(false)
    }
}
pub(super) fn parse(c: ParseContext<'_>) -> Result<C4Model, PeleError> {
    let mut r = Reader {
        src: c.source,
        tokens: tokenize(c.source),
        i: 0,
        db: Db::new(c.title),
    };
    let header = r.t();
    if header < T::C4_CONTEXT || header > T::C4_DEPLOYMENT {
        return Err(r.fail(HEADERS));
    }
    r.i += 1;
    r.expect(T::NEWLINE)?;
    let mut depth = 0;
    let mut state = P::Item;
    loop {
        let t = r.t();
        if matches!(state, P::AfterOther | P::AfterDiagram) {
            if t == T::NEWLINE {
                r.i += 1;
                state = P::ItemOrEnd;
                continue;
            }
            if state == P::AfterOther && diagram(t) {
                state = P::Diagram;
                continue;
            }
        } else if diagram(t) {
            if r.diagram(t)? {
                depth += 1;
                state = P::Diagram
            } else {
                state = P::AfterDiagram
            }
            continue;
        } else if other(t) && state != P::Diagram {
            r.other(t)?;
            state = P::AfterOther;
            continue;
        } else if state != P::ItemOrEnd {
            return Err(r.fail("'title', 'accDescription', 'acc_title', 'acc_descr', 'acc_descr_multiline_value', 'PERSON', 'REL'"));
        }
        if depth > 0 {
            if t != T::RBRACE {
                return Err(r.fail("'NEWLINE', 'RBRACE'"));
            }
            r.i += 1;
            r.db.pop();
            depth -= 1;
            state = P::AfterDiagram
        } else {
            if t != T::EOF {
                return Err(r.fail("'NEWLINE', 'EOF'"));
            }
            r.i += 1;
            r.expect(T::END)?;
            let first = &r.tokens[0];
            r.db.model.c4_type = Some(c.source[first.start..first.end].into());
            return Ok(r.db.finish());
        }
    }
}
