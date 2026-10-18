//! Direct Pele xychart/parser.ts, 458322a5 (MIT, kepano).
use super::{
    lexer::{tokenize, Token},
    model::{Orientation, PlotType, XyChartModel},
    tokens::{NAMES, T},
};
use crate::compat::{source::space, ParseContext, PeleError};
fn word(t: T) -> bool {
    matches!(
        t,
        T::AMP
            | T::NUM
            | T::ALPHA
            | T::PLUS
            | T::EQUALS
            | T::MULT
            | T::DOT
            | T::BRKT
            | T::MINUS
            | T::UNDERSCORE
    )
}
fn eol(t: T) -> bool {
    matches!(t, T::NEWLINE | T::SEMI | T::EOF)
}
fn is_text(t: T) -> bool {
    t == T::STR || word(t)
}
struct Reader<'a> {
    src: &'a str,
    tokens: Vec<Token>,
    i: usize,
    db: XyChartModel,
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
        let t = &self.tokens[self.i.min(self.tokens.len() - 1)];
        PeleError::syntax(
            "xychart",
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
    fn text(&mut self) -> Result<String, PeleError> {
        let t = self.t();
        if !is_text(t) {
            return Err(self.fail("'STR', 'ALPHA', 'NUM'"));
        }
        let mut s = self.take();
        if t != T::STR {
            while word(self.t()) {
                s.push_str(&self.take())
            }
        }
        Ok(s)
    }
    fn number(&mut self) -> Result<f64, PeleError> {
        Ok(self
            .expect(T::NUMBER_WITH_DECIMAL)?
            .parse()
            .unwrap_or(f64::NAN))
    }
    fn range(&mut self) -> Result<(f64, f64), PeleError> {
        let lo = self.number()?;
        self.expect(T::ARROW_DELIMITER)?;
        Ok((lo, self.number()?))
    }
    fn points(&mut self) -> Result<Vec<(f64, String)>, PeleError> {
        self.expect(T::SQUARE_BRACES_START)?;
        let mut out = vec![];
        loop {
            let value = self.number()?;
            let label = if self.t() == T::STR {
                self.take()
            } else {
                String::new()
            };
            out.push((value, label));
            if self.t() != T::COMMA {
                break;
            }
            self.i += 1;
        }
        self.expect(T::SQUARE_BRACES_END)?;
        Ok(out)
    }
    fn parse(mut self) -> Result<XyChartModel, PeleError> {
        loop {
            if eol(self.t()) {
                self.i += 1
            } else if self.t() != T::XYCHART {
                break;
            } else {
                self.i += 1;
                if self.t() == T::CHART_ORIENTATION {
                    self.db.orientation = Some(if self.take() == "horizontal" {
                        Orientation::Horizontal
                    } else {
                        Orientation::Vertical
                    })
                }
            }
        }
        while self.t() != T::END {
            let t = self.t();
            self.i += 1;
            match t {
                T::title => self.db.title = Some(self.text()?.trim_matches(space).into()),
                T::X_AXIS | T::Y_AXIS => {
                    let title = if is_text(self.t()) {
                        Some(self.text()?)
                    } else {
                        None
                    };
                    if t == T::X_AXIS && self.t() == T::SQUARE_BRACES_START {
                        let mut categories = vec![];
                        loop {
                            self.i += 1;
                            categories.push(self.text()?);
                            if self.t() != T::COMMA {
                                break;
                            }
                        }
                        self.expect(T::SQUARE_BRACES_END)?;
                        self.db.x_band(categories)
                    } else if self.t() == T::NUMBER_WITH_DECIMAL || title.is_none() {
                        let (min, max) = self.range()?;
                        if t == T::X_AXIS {
                            self.db.x_range(min, max)
                        } else {
                            self.db.y_range(min, max)
                        }
                    }
                    if t == T::X_AXIS {
                        self.db.x_title(title.unwrap_or_default())
                    } else {
                        self.db.y_title(title.unwrap_or_default())
                    }
                }
                T::LINE | T::BAR => {
                    let title = if self.t() == T::SQUARE_BRACES_START {
                        String::new()
                    } else {
                        self.text()?
                    };
                    let points = self.points()?;
                    self.db.add_plot(
                        if t == T::LINE {
                            PlotType::Line
                        } else {
                            PlotType::Bar
                        },
                        title,
                        points,
                    )
                }
                T::acc_title => {
                    self.db.acc_title =
                        Some(self.expect(T::acc_title_value)?.trim_matches(space).into())
                }
                T::acc_descr | T::acc_descr_multiline_value => {
                    let s = if t == T::acc_descr {
                        self.expect(T::acc_descr_value)?
                    } else {
                        self.tokens[self.i - 1].text.clone()
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
                    self.db.acc_descr = Some(out)
                }
                _ => {
                    self.i -= 1;
                    return Err(self.fail("'title', 'X_AXIS', 'Y_AXIS', 'LINE', 'BAR', 'acc_title', 'acc_descr', 'acc_descr_multiline_value'"));
                }
            }
            while eol(self.t()) {
                self.i += 1;
            }
        }
        Ok(self.db)
    }
}
pub(super) fn parse(context: ParseContext<'_>) -> Result<XyChartModel, PeleError> {
    Reader {
        src: context.source,
        tokens: tokenize(context.source),
        i: 0,
        db: XyChartModel::new(context.title),
    }
    .parse()
}
