//! Wardley grammar AST and iterative statement parser, Pele 458322a5 (MIT).
use super::super::common::langium::{self as l, Reader, END};
use super::lexer::TOKENS;
use crate::compat::PeleError;
#[derive(Default)]
pub(super) struct Ast {
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub size: Option<(f64, f64)>,
    pub evolution: Option<Vec<Stage>>,
    pub anchors: Vec<Placed>,
    pub components: Vec<Component>,
    pub links: Vec<Link>,
    pub evolves: Vec<(String, f64)>,
    pub pipelines: Vec<(String, Vec<PipelineComponent>)>,
    pub notes: Vec<Placed>,
    pub annotations: Vec<(f64, f64)>,
    pub annotation: Vec<(f64, f64, f64, String)>,
    pub accelerators: Vec<Placed>,
    pub deaccelerators: Vec<Placed>,
}
pub(super) struct Stage {
    pub name: String,
    pub boundary: Option<f64>,
    pub second_name: Option<String>,
}
pub(super) struct Placed {
    pub name: String,
    pub visibility: f64,
    pub evolution: f64,
}
pub(super) struct Component {
    pub at: Placed,
    pub label: Option<(f64, f64)>,
    pub strategy: Option<String>,
    pub inertia: bool,
}
pub(super) struct PipelineComponent {
    pub name: String,
    pub evolution: f64,
    pub label: Option<(f64, f64)>,
}
pub(super) struct Link {
    pub from: String,
    pub to: String,
    pub from_port: Option<String>,
    pub to_port: Option<String>,
    pub arrow: Option<String>,
    pub label: Option<String>,
}
fn name(r: &mut Reader<'_>) -> Result<String, PeleError> {
    match r.kind() {
        36 => Ok(l::string_value(r.take())),
        30 | 37 => Ok(r.take().into()),
        _ => Err(r.fail("a name")),
    }
}
fn number(r: &mut Reader<'_>) -> Result<f64, PeleError> {
    Ok(r.expect(10)?.parse().unwrap_or(f64::INFINITY))
}
fn int(r: &mut Reader<'_>) -> Result<f64, PeleError> {
    Ok(r.expect(35)?.parse().unwrap_or(f64::INFINITY))
}
fn coordinate(r: &mut Reader<'_>) -> Result<f64, PeleError> {
    if r.kind() == 10 {
        number(r)
    } else {
        int(r)
    }
}
fn pair(
    r: &mut Reader<'_>,
    read: fn(&mut Reader<'_>) -> Result<f64, PeleError>,
) -> Result<(f64, f64), PeleError> {
    r.expect(0)?;
    let a = read(r)?;
    r.expect(1)?;
    let b = read(r)?;
    r.expect(2)?;
    Ok((a, b))
}
fn label(r: &mut Reader<'_>) -> Result<Option<(f64, f64)>, PeleError> {
    if !r.accept(21) {
        return Ok(None);
    }
    r.expect(0)?;
    let neg_x = r.accept(7);
    let x = int(r)?;
    r.expect(1)?;
    let neg_y = r.accept(7);
    let y = int(r)?;
    r.expect(2)?;
    Ok(Some((
        if neg_x { -x } else { x },
        if neg_y { -y } else { y },
    )))
}
fn stage(r: &mut Reader<'_>) -> Result<Stage, PeleError> {
    let name = name(r)?;
    let boundary = if r.accept(3) { Some(number(r)?) } else { None };
    let second_name = if r.accept(4) {
        Some(self::name(r)?)
    } else {
        None
    };
    Ok(Stage {
        name,
        boundary,
        second_name,
    })
}
fn end(r: &mut Reader<'_>) -> Result<(), PeleError> {
    if r.kind() == END {
        return Ok(());
    }
    if r.kind() != 38 {
        return Err(r.fail("a line break or the end of input"));
    }
    while r.accept(38) {}
    Ok(())
}
fn placed(r: &mut Reader<'_>) -> Result<Placed, PeleError> {
    let name = name(r)?;
    let (visibility, evolution) = pair(r, number)?;
    end(r)?;
    Ok(Placed {
        name,
        visibility,
        evolution,
    })
}
pub(super) fn parse(src: &str) -> Result<Ast, PeleError> {
    let mut r = Reader::new(src, TOKENS, "wardley")?;
    let mut ast = Ast::default();
    while r.accept(38) {}
    r.expect(16)?;
    while r.kind() != END {
        match r.kind() {
            38 => r.i += 1,
            32 => {
                ast.acc_descr = Some(l::acc_descr_value(r.take()));
                end(&mut r)?
            }
            33 => {
                ast.acc_title = Some(l::acc_title_value(r.take()));
                end(&mut r)?
            }
            34 => {
                ast.title = Some(l::title_value(r.take()));
                end(&mut r)?
            }
            17 => {
                r.i += 1;
                ast.size = Some(pair(&mut r, int)?);
                end(&mut r)?
            }
            18 => {
                r.i += 1;
                let mut stages = vec![stage(&mut r)?];
                r.expect(11)?;
                stages.push(stage(&mut r)?);
                while r.accept(11) {
                    stages.push(stage(&mut r)?)
                }
                end(&mut r)?;
                ast.evolution = Some(stages)
            }
            19 => {
                r.i += 1;
                ast.anchors.push(placed(&mut r)?)
            }
            20 => {
                r.i += 1;
                let name = name(&mut r)?;
                let (visibility, evolution) = pair(&mut r, number)?;
                let label = label(&mut r)?;
                let strategy = if r.kind() == 5 && r.kind_at(r.i + 1) == 15 {
                    r.i += 1;
                    let s = r.take().into();
                    r.expect(6)?;
                    Some(s)
                } else {
                    None
                };
                let inertia = if r.accept(22) {
                    true
                } else if r.accept(5) {
                    r.expect(22)?;
                    r.expect(6)?;
                    true
                } else {
                    false
                };
                end(&mut r)?;
                ast.components.push(Component {
                    at: Placed {
                        name,
                        visibility,
                        evolution,
                    },
                    label,
                    strategy,
                    inertia,
                })
            }
            23 => {
                r.i += 1;
                let name = name(&mut r)?;
                let target = number(&mut r)?;
                end(&mut r)?;
                ast.evolves.push((name, target));
            }
            24 => {
                r.i += 1;
                let parent = name(&mut r)?;
                r.expect(8)?;
                r.expect(38)?;
                while r.accept(38) {}
                let mut components = vec![];
                loop {
                    r.expect(20)?;
                    let name = name(&mut r)?;
                    r.expect(0)?;
                    let evolution = number(&mut r)?;
                    r.expect(2)?;
                    let label = label(&mut r)?;
                    end(&mut r)?;
                    components.push(PipelineComponent {
                        name,
                        evolution,
                        label,
                    });
                    if r.kind() != 20 {
                        break;
                    }
                }
                r.expect(9)?;
                end(&mut r)?;
                ast.pipelines.push((parent, components));
            }
            25 => {
                r.i += 1;
                let name = l::string_value(r.expect(36)?);
                let (visibility, evolution) = pair(&mut r, number)?;
                end(&mut r)?;
                ast.notes.push(Placed {
                    name,
                    visibility,
                    evolution,
                });
            }
            26 => {
                r.i += 1;
                let xy = pair(&mut r, coordinate)?;
                end(&mut r)?;
                ast.annotations.push(xy)
            }
            27 => {
                r.i += 1;
                let n = int(&mut r)?;
                r.expect(1)?;
                let (x, y) = pair(&mut r, coordinate)?;
                let text = l::string_value(r.expect(36)?);
                end(&mut r)?;
                ast.annotation.push((n, x, y, text))
            }
            28 | 29 => {
                let slow = r.kind() == 29;
                r.i += 1;
                let p = placed(&mut r)?;
                if slow {
                    ast.deaccelerators.push(p)
                } else {
                    ast.accelerators.push(p)
                }
            }
            _ => {
                if !matches!(r.kind(), 36 | 37 | 30) {
                    return Err(r.fail("a statement"));
                }
                let from = name(&mut r)?;
                let from_port = if r.kind() == 12 {
                    Some(r.take().into())
                } else {
                    None
                };
                let arrow = if matches!(r.kind(), 13 | 11) {
                    Some(r.take().into())
                } else {
                    None
                };
                let to = name(&mut r)?;
                let to_port = if r.kind() == 12 {
                    Some(r.take().into())
                } else {
                    None
                };
                let label = if r.kind() == 14 {
                    Some(
                        r.take()[1..]
                            .trim_matches(crate::compat::source::space)
                            .into(),
                    )
                } else {
                    None
                };
                end(&mut r)?;
                ast.links.push(Link {
                    from,
                    to,
                    from_port,
                    to_port,
                    arrow,
                    label,
                });
            }
        }
    }
    Ok(ast)
}
