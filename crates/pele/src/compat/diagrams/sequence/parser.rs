//! Pele sequence/parser.ts grammar with deferred, flat source-ordered actions,
//! 458322a5 (MIT, kepano). Flat actions preserve apply order without recursive trees.
use super::{
    kinds::*,
    lexer::{tokenize, Token},
    model::{parse_message, Db, NoteActors, SeqNumbering, SequenceModel, Statement, Text},
    tokens::{NAMES, T},
};
use crate::compat::{source::space, ParseContext, PeleError};
fn signal(t: T) -> Option<u8> {
    Some(match t {
        T::SOLID_OPEN_ARROW => SOLID_OPEN,
        T::DOTTED_OPEN_ARROW => DOTTED_OPEN,
        T::SOLID_ARROW => SOLID,
        T::DOTTED_ARROW => DOTTED,
        T::BIDIRECTIONAL_SOLID_ARROW => BIDIRECTIONAL_SOLID,
        T::BIDIRECTIONAL_DOTTED_ARROW => BIDIRECTIONAL_DOTTED,
        T::SOLID_CROSS => SOLID_CROSS,
        T::DOTTED_CROSS => DOTTED_CROSS,
        T::SOLID_POINT => SOLID_POINT,
        T::DOTTED_POINT => DOTTED_POINT,
        T::SOLID_ARROW_TOP => SOLID_TOP,
        T::SOLID_ARROW_BOTTOM => SOLID_BOTTOM,
        T::STICK_ARROW_TOP => STICK_TOP,
        T::STICK_ARROW_BOTTOM => STICK_BOTTOM,
        T::SOLID_ARROW_TOP_DOTTED => SOLID_TOP_DOTTED,
        T::SOLID_ARROW_BOTTOM_DOTTED => SOLID_BOTTOM_DOTTED,
        T::STICK_ARROW_TOP_DOTTED => STICK_TOP_DOTTED,
        T::STICK_ARROW_BOTTOM_DOTTED => STICK_BOTTOM_DOTTED,
        T::SOLID_ARROW_TOP_REVERSE => SOLID_ARROW_TOP_REVERSE,
        T::SOLID_ARROW_BOTTOM_REVERSE => SOLID_ARROW_BOTTOM_REVERSE,
        T::STICK_ARROW_TOP_REVERSE => STICK_ARROW_TOP_REVERSE,
        T::STICK_ARROW_BOTTOM_REVERSE => STICK_ARROW_BOTTOM_REVERSE,
        T::SOLID_ARROW_TOP_REVERSE_DOTTED => SOLID_ARROW_TOP_REVERSE_DOTTED,
        T::SOLID_ARROW_BOTTOM_REVERSE_DOTTED => SOLID_ARROW_BOTTOM_REVERSE_DOTTED,
        T::STICK_ARROW_TOP_REVERSE_DOTTED => STICK_ARROW_TOP_REVERSE_DOTTED,
        T::STICK_ARROW_BOTTOM_REVERSE_DOTTED => STICK_ARROW_BOTTOM_REVERSE_DOTTED,
        _ => return None,
    })
}
#[derive(Clone, Copy, PartialEq)]
enum K {
    Top,
    Box,
    Loop,
    Rect,
    Opt,
    Break,
    Alt,
    Par,
    Critical,
}
fn block(t: T) -> Option<(K, u8, u8)> {
    Some(match t {
        T::r#loop => (K::Loop, LOOP_START, LOOP_END),
        T::rect => (K::Rect, RECT_START, RECT_END),
        T::opt => (K::Opt, OPT_START, OPT_END),
        T::r#break => (K::Break, BREAK_START, BREAK_END),
        T::alt => (K::Alt, ALT_START, ALT_END),
        T::par => (K::Par, PAR_START, PAR_END),
        T::par_over => (K::Par, PAR_OVER_START, PAR_END),
        T::critical => (K::Critical, CRITICAL_START, CRITICAL_END),
        T::r#box => (K::Box, 0, 0),
        _ => return None,
    })
}
const SIGNAL_TOKENS:&str="'SOLID_OPEN_ARROW', 'DOTTED_OPEN_ARROW', 'SOLID_ARROW', 'DOTTED_ARROW', 'SOLID_CROSS', 'DOTTED_CROSS', 'SOLID_POINT', 'DOTTED_POINT'";
struct Frame {
    kind: K,
    raw: String,
    start: usize,
    start_type: u8,
    end_type: u8,
}
struct Reader<'a> {
    src: &'a str,
    tokens: Vec<Token>,
    i: usize,
    db: Db,
    statements: Vec<Statement>,
}
fn sig(from: Option<String>, to: Option<String>, text: Option<Text>, kind: u8) -> Statement {
    Statement::Signal {
        from,
        to,
        text,
        kind,
        activate: false,
        central: 0,
        validate_life: false,
    }
}
fn participant_id(s: &Statement) -> &str {
    if let Statement::Participant { actor, .. } = s {
        actor
    } else {
        unreachable!()
    }
}
impl Reader<'_> {
    fn t(&self) -> T {
        self.tokens.get(self.i).map_or(T::END, |t| t.kind)
    }
    fn take(&mut self) -> String {
        let text = self.tokens[self.i].text.clone();
        self.i += 1;
        text
    }
    fn fail(&self, expected: &str) -> PeleError {
        let t = &self.tokens[self.i.min(self.tokens.len() - 1)];
        PeleError::syntax(
            "sequence",
            self.src,
            t.start,
            &if t.kind == T::ERROR {
                String::new()
            } else {
                format!("Expecting {expected}, got '{}'", NAMES[t.kind as usize])
            },
            t.kind == T::ERROR,
        )
    }
    fn expect(&mut self, t: T) -> Result<String, PeleError> {
        if self.t() != t {
            return Err(self.fail(&format!("'{}'", NAMES[t as usize])));
        }
        Ok(self.take())
    }
    fn actor(&mut self) -> Result<Statement, PeleError> {
        if !matches!(self.t(), T::ACTOR | T::link) {
            return Err(self.fail("'ACTOR'"));
        }
        Ok(Statement::Participant {
            actor: self.take(),
            draw: None,
            description: None,
            config: None,
            created: false,
        })
    }
    fn text2(&mut self) -> Result<Text, PeleError> {
        let t = self.expect(T::TXT)?;
        let s = t.trim_matches(space);
        let text = s.chars().next().map_or("", |c| &s[c.len_utf8()..]);
        Ok(parse_message(text))
    }
    fn participant(&mut self) -> Result<Statement, PeleError> {
        let t = self.t();
        self.i += 1;
        let node = self.actor()?;
        let actor = participant_id(&node).to_owned();
        if t == T::destroy {
            self.expect(T::NEWLINE)?;
            return Ok(Statement::Destroy(actor));
        }
        let config = if self.t() == T::CONFIG_START && self.tokens[self.i - 1].kind == T::ACTOR {
            self.i += 1;
            let s = self.expect(T::CONFIG_CONTENT)?;
            self.expect(T::CONFIG_END)?;
            Some(s.trim_matches(space).into())
        } else {
            None
        };
        let draw = Some(
            if t == T::participant {
                "participant"
            } else {
                "actor"
            }
            .into(),
        );
        let description = if self.t() == T::AS {
            self.i += 1;
            let alias = self.expect(T::restOfLine)?;
            self.expect(T::NEWLINE)?;
            Some(parse_message(&alias))
        } else if self.t() == T::NEWLINE {
            self.i += 1;
            None
        } else {
            return Err(self.fail("'AS', 'NEWLINE'"));
        };
        Ok(Statement::Participant {
            actor,
            draw,
            description,
            config,
            created: false,
        })
    }
    fn signal(&mut self, from: Statement) -> Result<(), PeleError> {
        let from_id = participant_id(&from).to_owned();
        let central_from = self.t() == T::CENTRAL;
        if central_from {
            self.i += 1
        }
        let Some(kind) = signal(self.t()) else {
            return Err(self.fail(SIGNAL_TOKENS));
        };
        self.i += 1;
        let mark = self.t();
        if mark == T::CENTRAL || !central_from && matches!(mark, T::PLUS | T::MINUS) {
            self.i += 1
        }
        let to = self.actor()?;
        let to_id = participant_id(&to).to_owned();
        let text = self.text2()?;
        self.statements.push(from);
        self.statements.push(to);
        let (activate, central) = if central_from {
            (
                mark == T::CENTRAL,
                if mark == T::CENTRAL {
                    CENTRAL_CONNECTION_DUAL
                } else {
                    CENTRAL_CONNECTION_REVERSE
                },
            )
        } else {
            (
                matches!(mark, T::PLUS | T::CENTRAL),
                if mark == T::CENTRAL {
                    CENTRAL_CONNECTION
                } else {
                    0
                },
            )
        };
        self.statements.push(Statement::Signal {
            from: Some(from_id.clone()),
            to: Some(to_id.clone()),
            text: Some(text),
            kind,
            activate,
            central,
            validate_life: true,
        });
        if central_from {
            if mark == T::CENTRAL {
                self.statements
                    .push(sig(Some(to_id), None, None, CENTRAL_CONNECTION))
            }
            self.statements
                .push(sig(Some(from_id), None, None, CENTRAL_CONNECTION_REVERSE));
        } else if mark == T::PLUS {
            self.statements
                .push(sig(Some(to_id), None, None, ACTIVE_START))
        } else if mark == T::MINUS {
            self.statements
                .push(sig(Some(from_id), None, None, ACTIVE_END))
        } else if mark == T::CENTRAL {
            self.statements
                .push(sig(Some(to_id), None, None, CENTRAL_CONNECTION))
        }
        Ok(())
    }
    fn menu(&mut self, kind: &str) -> Result<(), PeleError> {
        self.i += 1;
        let who = self.actor()?;
        let id = participant_id(&who).into();
        let text = self.text2()?;
        self.statements.push(who);
        self.statements.push(Statement::Menu(kind.into(), id, text));
        Ok(())
    }
    fn note(&mut self) -> Result<(), PeleError> {
        self.i += 1;
        let t = self.t();
        self.i += 1;
        if t == T::over {
            let first = self.actor()?;
            let a = participant_id(&first).to_owned();
            self.statements.push(first);
            let b = if self.t() == T::COMMA {
                self.i += 1;
                let second = self.actor()?;
                let b = participant_id(&second).to_owned();
                self.statements.push(second);
                b
            } else {
                a.clone()
            };
            let text = self.text2()?;
            self.statements
                .push(Statement::Note(NoteActors::Actors(vec![a, b]), 2, text))
        } else if matches!(t, T::left_of | T::right_of) {
            let who = self.actor()?;
            let id = participant_id(&who).into();
            let text = self.text2()?;
            self.statements.push(who);
            self.statements.push(Statement::Note(
                NoteActors::Actor(id),
                if t == T::left_of { 0 } else { 1 },
                text,
            ))
        } else {
            self.i -= 1;
            return Err(self.fail("'over', 'left_of', 'right_of'"));
        }
        Ok(())
    }
    fn autonumber(&mut self) -> Result<(), PeleError> {
        self.i += 1;
        let mut spec = SeqNumbering {
            start: None,
            step: None,
            visible: true,
        };
        if self.t() == T::NUM {
            spec.start = Some(self.take().parse().unwrap_or(f64::NAN));
            spec.step = Some(if self.t() == T::NUM {
                self.take().parse().unwrap_or(f64::NAN)
            } else {
                1.
            });
        } else if self.t() == T::off {
            self.i += 1;
            spec.visible = false
        }
        if self.t() != T::NEWLINE {
            return Err(self.fail("'NEWLINE', 'NUM', 'off'"));
        }
        self.i += 1;
        self.statements.push(Statement::Number(spec));
        Ok(())
    }
    fn close(&mut self, frame: Frame) {
        if frame.kind == K::Box {
            self.statements[frame.start] = Statement::BoxStart(frame.raw);
            self.statements.push(Statement::BoxEnd)
        } else {
            self.statements[frame.start] = sig(
                None,
                None,
                Some(parse_message(&frame.raw)),
                frame.start_type,
            );
            self.statements.push(sig(None, None, None, frame.end_type));
        }
    }
    fn parse(mut self) -> Result<SequenceModel, PeleError> {
        while self.t() == T::NEWLINE {
            self.i += 1
        }
        self.expect(T::SD)?;
        let mut stack: Vec<Frame> = vec![];
        let mut frame = Frame {
            kind: K::Top,
            raw: String::new(),
            start: 0,
            start_type: 0,
            end_type: 0,
        };
        loop {
            let t = self.t();
            if frame.kind == K::Box {
                match t {
                    T::participant | T::participant_actor | T::destroy => {
                        let p = self.participant()?;
                        self.statements.push(p)
                    }
                    T::NEWLINE => self.i += 1,
                    T::end => {
                        self.i += 1;
                        self.close(frame);
                        frame = stack.pop().unwrap()
                    }
                    _ => {
                        return Err(self.fail(
                            "'end', 'NEWLINE', 'participant', 'participant_actor', 'destroy'",
                        ))
                    }
                }
                continue;
            }
            match t {
                T::NEWLINE | T::INVALID => self.i += 1,
                T::participant | T::participant_actor | T::destroy => {
                    let p = self.participant()?;
                    self.statements.push(p)
                }
                T::create => {
                    self.i += 1;
                    if !matches!(self.t(), T::participant | T::participant_actor | T::destroy) {
                        return Err(self.fail("'participant', 'participant_actor', 'destroy'"));
                    }
                    let p = self.participant()?;
                    let p = match p {
                        Statement::Participant {
                            actor,
                            draw,
                            description,
                            config,
                            ..
                        } => Statement::Participant {
                            actor,
                            draw,
                            description,
                            config,
                            created: true,
                        },
                        Statement::Destroy(actor) => Statement::Participant {
                            actor,
                            draw: None,
                            description: None,
                            config: None,
                            created: true,
                        },
                        _ => unreachable!(),
                    };
                    self.statements.push(p)
                }
                T::ACTOR => {
                    let a = self.actor()?;
                    self.signal(a)?;
                    self.expect(T::NEWLINE)?;
                }
                T::link => {
                    let next = self.tokens.get(self.i + 1).map(|t| t.kind);
                    if matches!(next, Some(T::ACTOR | T::link)) {
                        self.menu("link")?
                    } else {
                        let a = self.actor()?;
                        self.signal(a)?
                    }
                    self.expect(T::NEWLINE)?;
                }
                T::links | T::properties | T::details => {
                    self.menu(match t {
                        T::links => "links",
                        T::properties => "properties",
                        _ => "details",
                    })?;
                    self.expect(T::NEWLINE)?;
                }
                T::note => {
                    self.note()?;
                    self.expect(T::NEWLINE)?;
                }
                T::autonumber => self.autonumber()?,
                T::activate | T::deactivate => {
                    self.i += 1;
                    let who = self.actor()?;
                    let id = participant_id(&who).into();
                    self.expect(T::NEWLINE)?;
                    self.statements.push(sig(
                        Some(id),
                        None,
                        None,
                        if t == T::activate {
                            ACTIVE_START
                        } else {
                            ACTIVE_END
                        },
                    ))
                }
                T::title | T::legacy_title => {
                    let title = self.take();
                    let n = if t == T::title { 6 } else { 7 };
                    self.db.model.title = Some(title.chars().skip(n).collect())
                }
                T::acc_title => {
                    self.i += 1;
                    self.db.model.acc_title =
                        Some(self.expect(T::acc_title_value)?.trim_matches(space).into())
                }
                T::acc_descr | T::acc_descr_multiline_value => {
                    let value = if t == T::acc_descr {
                        self.i += 1;
                        self.expect(T::acc_descr_value)?
                    } else {
                        self.take()
                    };
                    let s = value.trim_matches(space);
                    let mut chars = s.chars().peekable();
                    let mut out = String::new();
                    while let Some(c) = chars.next() {
                        out.push(c);
                        if c == '\n' {
                            while chars.peek().is_some_and(|c| space(*c)) {
                                chars.next();
                            }
                        }
                    }
                    self.db.model.acc_descr = Some(out)
                }
                T::r#else | T::and | T::option => {
                    if frame.kind
                        != match t {
                            T::r#else => K::Alt,
                            T::and => K::Par,
                            _ => K::Critical,
                        }
                    {
                        return Err(self.fail("'end'"));
                    }
                    self.i += 1;
                    let raw = self.expect(T::restOfLine)?;
                    self.statements.push(sig(
                        None,
                        None,
                        Some(parse_message(&raw)),
                        match t {
                            T::r#else => ALT_ELSE,
                            T::and => PAR_AND,
                            _ => CRITICAL_OPTION,
                        },
                    ))
                }
                T::end => {
                    if frame.kind == K::Top {
                        return Err(self.fail("'NEWLINE', 'EOF'"));
                    }
                    self.i += 1;
                    self.close(frame);
                    frame = stack.pop().unwrap()
                }
                T::END => {
                    if frame.kind != K::Top {
                        return Err(self.fail("'end'"));
                    }
                    return self.db.apply(self.statements);
                }
                _ => {
                    if let Some((kind, start_type, end_type)) = block(t) {
                        self.i += 1;
                        let raw = self.expect(T::restOfLine)?;
                        stack.push(frame);
                        frame = Frame {
                            kind,
                            raw,
                            start: self.statements.len(),
                            start_type,
                            end_type,
                        };
                        self.statements.push(Statement::Empty)
                    } else {
                        return Err(self.fail("'NEWLINE', 'ACTOR', 'participant', 'note', 'end'"));
                    }
                }
            }
        }
    }
}
pub(super) fn parse(context: ParseContext<'_>) -> Result<SequenceModel, PeleError> {
    Reader {
        src: context.source,
        tokens: tokenize(context.source),
        i: 0,
        db: Db::new(context.config, context.title),
        statements: vec![],
    }
    .parse()
}
