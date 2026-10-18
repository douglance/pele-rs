//! Pele usecase/db.ts, 458322a5 (MIT, kepano). Deferred symbol resolution,
//! declaration conflicts, metadata precedence and source-ordered model publication.
use super::{lexer::Source, model::*};
use crate::compat::{source::space, ErrorCode, PeleError};
use std::collections::{HashMap, HashSet};
#[derive(Clone)]
struct Origin {
    kind: String,
    span: Span,
    generated: bool,
    label: Option<String>,
}
#[derive(Clone)]
struct State {
    draft: DraftElement,
    actor_type: Option<String>,
    icon: Option<String>,
    business: Option<bool>,
    stereotype_span: Option<Span>,
}
struct Boundary {
    draft: DraftBoundary,
    boundary_type: Option<String>,
    members: Vec<String>,
}
fn unique(target: &mut Vec<String>, values: &[String]) {
    let mut seen: HashSet<_> = target.iter().cloned().collect();
    for v in values {
        if seen.insert(v.clone()) {
            target.push(v.clone())
        }
    }
}
fn error(src: &Source, message: String, span: Span) -> PeleError {
    let (line, column) = src.loc(span[0]);
    let mut e = PeleError::new(message, ErrorCode::Semantic);
    e.diagram_type = Some("usecase".into());
    e.line = line;
    e.column = column;
    e
}
fn fail(src: &Source, message: String, span: Span) -> PeleError {
    error(src, format!("{message} at {}", src.location(span)), span)
}
fn conflict(
    src: &Source,
    message: String,
    current: Span,
    previous: Span,
    current_label: Option<&str>,
    previous_label: Option<&str>,
) -> PeleError {
    let suffix = |s: Option<&str>| s.map_or(String::new(), |s| format!(" (label \"{s}\")"));
    error(
        src,
        format!(
            "{message} at {}{}; previous declaration at {}{}",
            src.location(current),
            suffix(current_label),
            src.location(previous),
            suffix(previous_label)
        ),
        current,
    )
}
fn invalid(src: &Source, id: &str, kind: &str, property: &DraftProperty) -> PeleError {
    fail(
        src,
        format!(
            "Metadata property '{}' is invalid for {kind} '{id}'",
            property.occurrence.key
        ),
        property.occurrence.key_span,
    )
}
fn infer(properties: &[DraftProperty]) -> Option<&'static str> {
    let mut possible: HashSet<&str> = ["actor", "usecase", "boundary", "edge"]
        .into_iter()
        .collect();
    for p in properties {
        match p.occurrence.key.as_str() {
            "icon" => {
                possible.clear();
                possible.insert("actor");
            }
            "animate" | "animation" => {
                possible.clear();
                possible.insert("edge");
            }
            "type" => {
                possible.clear();
                match p.value.text() {
                    Some("rect" | "package") => {
                        possible.insert("boundary");
                    }
                    Some("normal" | "hollow" | "awesome") => {
                        possible.insert("actor");
                    }
                    _ => {}
                }
            }
            "business" => {
                possible.remove("boundary");
                possible.remove("edge");
            }
            _ => return None,
        }
    }
    if possible.len() == 1 {
        possible.into_iter().next()
    } else {
        None
    }
}
fn metadata(
    src: &Source,
    state: &mut State,
    properties: &[DraftProperty],
    replace: bool,
) -> Result<(), PeleError> {
    for p in properties {
        let key = &p.occurrence.key;
        let id = &state.draft.named.id;
        let before = state.draft.named.span;
        if state.draft.kind == "actor" {
            match (key.as_str(), &p.value) {
                ("type", PropertyValue::Text(v))
                    if ["normal", "hollow", "awesome"].contains(&v.as_str()) =>
                {
                    if !replace && state.actor_type.as_ref().is_some_and(|old| old != v) {
                        return Err(conflict(
                            src,
                            format!("Actor '{id}' has conflicting {key} metadata"),
                            p.occurrence.key_span,
                            before,
                            None,
                            None,
                        ));
                    }
                    state.actor_type = Some(v.clone())
                }
                ("icon", PropertyValue::Text(v)) => {
                    if !replace && state.icon.as_ref().is_some_and(|old| old != v) {
                        return Err(conflict(
                            src,
                            format!("Actor '{id}' has conflicting {key} metadata"),
                            p.occurrence.key_span,
                            before,
                            None,
                            None,
                        ));
                    }
                    state.icon = Some(v.clone())
                }
                ("business", PropertyValue::Bool(v)) => {
                    if !replace && state.business.is_some_and(|old| old != *v) {
                        return Err(conflict(
                            src,
                            format!("Actor '{id}' has conflicting {key} metadata"),
                            p.occurrence.key_span,
                            before,
                            None,
                            None,
                        ));
                    }
                    state.business = Some(*v)
                }
                _ => return Err(invalid(src, id, "actor", p)),
            }
        } else if key == "business" && p.value.boolean().is_some() {
            let v = p.value.boolean().unwrap();
            if !replace && state.business.is_some_and(|old| old != v) {
                return Err(conflict(
                    src,
                    format!("Use case '{id}' has conflicting business metadata"),
                    p.occurrence.key_span,
                    before,
                    None,
                    None,
                ));
            }
            state.business = Some(v)
        } else {
            return Err(invalid(src, id, "usecase", p));
        }
    }
    Ok(())
}
struct Resolver<'a> {
    src: &'a Source,
    symbols: HashMap<String, Origin>,
    elements: HashMap<String, State>,
    element_order: Vec<String>,
    boundaries: HashMap<String, Boundary>,
    boundary_order: Vec<String>,
    first: HashMap<String, usize>,
}
impl Resolver<'_> {
    fn mention(&mut self, id: &str, offset: usize) {
        self.first
            .entry(id.into())
            .and_modify(|old| *old = (*old).min(offset))
            .or_insert(offset);
    }
    fn register(&mut self, named: &Named, kind: &str) {
        self.symbols.insert(
            named.id.clone(),
            Origin {
                kind: kind.into(),
                span: named.span,
                generated: named.generated,
                label: Some(named.label.text.clone()),
            },
        );
    }
    fn claim(&self, named: &Named, kind: &str) -> Result<(), PeleError> {
        if let Some(old) = self.symbols.get(&named.id) {
            let label = if named.generated {
                Some(named.label.text.as_str())
            } else {
                None
            };
            let old_label = if old.generated {
                old.label.as_deref()
            } else {
                None
            };
            if old.kind != kind {
                return Err(conflict(
                    self.src,
                    format!(
                        "ID '{}' is declared as both {} and {kind}",
                        named.id, old.kind
                    ),
                    named.span,
                    old.span,
                    label,
                    old_label,
                ));
            }
            if old.generated || named.generated {
                return Err(conflict(
                    self.src,
                    format!(
                        "Generated ID '{}' collides with another declaration",
                        named.id
                    ),
                    named.span,
                    old.span,
                    label,
                    old_label,
                ));
            }
        }
        Ok(())
    }
    fn unique(&mut self, id: &str, kind: &str, span: Span) -> Result<(), PeleError> {
        if let Some(old) = self.symbols.get(id) {
            return Err(conflict(
                self.src,
                format!(
                    "ID '{id}' is declared more than once ({} and {kind})",
                    old.kind
                ),
                span,
                old.span,
                None,
                if old.generated {
                    old.label.as_deref()
                } else {
                    None
                },
            ));
        }
        self.symbols.insert(
            id.into(),
            Origin {
                kind: kind.into(),
                span,
                generated: false,
                label: None,
            },
        );
        Ok(())
    }
    fn element(&mut self, d: &DraftElement) -> Result<(), PeleError> {
        self.claim(&d.named, &d.kind)?;
        let id = &d.named.id;
        if let Some(s) = self.elements.get_mut(id) {
            let old = &s.draft;
            if old.named.label.text != d.named.label.text
                || old.named.label.kind != d.named.label.kind
            {
                return Err(conflict(
                    self.src,
                    format!("ID '{id}' has conflicting labels"),
                    d.named.span,
                    old.named.span,
                    None,
                    None,
                ));
            }
            if let (Some(a), Some(b)) = (&d.shape, &old.shape) {
                if a != b {
                    return Err(conflict(
                        self.src,
                        format!("Use case '{id}' has conflicting shapes"),
                        d.named.span,
                        old.named.span,
                        None,
                        None,
                    ));
                }
            }
            if let (Some(a), Some(b)) = (&d.parent, &old.parent) {
                if !a.named.id.is_empty() && !b.named.id.is_empty() && a.named.id != b.named.id {
                    return Err(conflict(
                        self.src,
                        format!("Element '{id}' belongs to more than one system boundary"),
                        a.named.span,
                        b.named.span,
                        None,
                        None,
                    ));
                }
            }
            if let (Some(a), Some(b)) = (&d.stereotype, &old.stereotype) {
                if !a.is_empty() && !b.is_empty() && a != b {
                    return Err(conflict(
                        self.src,
                        format!("Element '{id}' has conflicting stereotypes"),
                        d.named.span,
                        s.stereotype_span.unwrap_or(old.named.span),
                        None,
                        None,
                    ));
                }
            }
            if s.draft.shape.is_none() {
                s.draft.shape = d.shape.clone()
            }
            if s.draft.parent.is_none() {
                s.draft.parent = d.parent.clone()
            }
            if s.draft.stereotype.is_none() {
                s.draft.stereotype = d.stereotype.clone()
            }
            if s.stereotype_span.is_none() && d.stereotype.as_ref().is_some_and(|s| !s.is_empty()) {
                s.stereotype_span = Some(d.named.span)
            }
            unique(&mut s.draft.classes, &d.classes);
            metadata(
                self.src,
                s,
                d.metadata.as_deref().unwrap_or_default(),
                false,
            )?;
        } else {
            let mut draft = d.clone();
            draft.parent = d
                .parent
                .as_ref()
                .filter(|p| !p.named.id.is_empty())
                .cloned();
            let mut state = State {
                draft,
                actor_type: None,
                icon: None,
                business: None,
                stereotype_span: d
                    .stereotype
                    .as_ref()
                    .filter(|s| !s.is_empty())
                    .map(|_| d.named.span),
            };
            metadata(
                self.src,
                &mut state,
                d.metadata.as_deref().unwrap_or_default(),
                false,
            )?;
            self.elements.insert(id.clone(), state);
            self.element_order.push(id.clone());
            self.register(&d.named, &d.kind)
        }
        Ok(())
    }
    fn boundary(&mut self, d: &DraftBoundary) -> Result<(), PeleError> {
        self.claim(&d.named, "boundary")?;
        let id = &d.named.id;
        if let Some(old) = self.boundaries.get_mut(id) {
            if old.draft.named.label.text != d.named.label.text
                || old.draft.named.label.kind != d.named.label.kind
            {
                return Err(conflict(
                    self.src,
                    format!("Boundary '{id}' has conflicting titles"),
                    d.named.span,
                    old.draft.named.span,
                    None,
                    None,
                ));
            }
            unique(&mut old.draft.classes, &d.classes)
        } else {
            self.boundaries.insert(
                id.clone(),
                Boundary {
                    draft: d.clone(),
                    boundary_type: None,
                    members: vec![],
                },
            );
            self.boundary_order.push(id.clone());
            self.register(&d.named, "boundary")
        }
        Ok(())
    }
    fn endpoint(&mut self, end: &DraftEndpoint) -> Result<String, PeleError> {
        let id = &end.named.id;
        if end.classed && !end.declares {
            return Err(fail(
                self.src,
                format!("Relationship endpoint '{id}' uses ::: without declaring the node"),
                end.named.span,
            ));
        }
        if let Some(old) = self.symbols.get(id) {
            return Ok(old.kind.clone());
        }
        self.elements.insert(
            id.clone(),
            State {
                draft: DraftElement {
                    named: end.named.clone(),
                    kind: "usecase".into(),
                    parent: None,
                    shape: Some("ellipse".into()),
                    metadata: None,
                    stereotype: None,
                    classes: vec![],
                },
                actor_type: None,
                icon: None,
                business: None,
                stereotype_span: None,
            },
        );
        self.element_order.push(id.clone());
        self.register(&end.named, "usecase");
        self.mention(id, end.named.span[0]);
        Ok("usecase".into())
    }
}
pub(super) fn resolve(
    source: &str,
    d: Drafts,
    header: Span,
    mut statements: Vec<GraphStatement>,
    title: Option<String>,
) -> Result<UsecaseModel, PeleError> {
    let src = Source::new(source);
    let mut r = Resolver {
        src: &src,
        symbols: HashMap::new(),
        elements: HashMap::new(),
        element_order: vec![],
        boundaries: HashMap::new(),
        boundary_order: vec![],
        first: HashMap::new(),
    };
    let mut model = UsecaseModel::default();
    let mut declarations = vec![];
    for rel in &d.relationships {
        r.mention(&rel.source.named.id, rel.source.named.span[0]);
        r.mention(&rel.target.named.id, rel.target.named.span[0])
    }
    for (i, x) in d.elements.iter().enumerate() {
        r.mention(&x.named.id, x.named.span[0]);
        declarations.push((x.named.span[0], 0, i))
    }
    for (i, x) in d.boundaries.iter().enumerate() {
        r.mention(&x.named.id, x.named.span[0]);
        declarations.push((x.named.span[0], 1, i))
    }
    for (i, x) in d.jsons.iter().enumerate() {
        r.mention(&x.id, x.span[0]);
        declarations.push((x.span[0], 2, i))
    }
    for (i, x) in d.relationships.iter().enumerate() {
        if let (Some(id), Some(span)) = (&x.id, x.id_span) {
            if !id.is_empty() {
                declarations.push((span[0], 3, i))
            }
        }
    }
    declarations.sort_by_key(|a| a.0);
    for (_, kind, i) in declarations {
        match kind {
            0 => r.element(&d.elements[i])?,
            1 => r.boundary(&d.boundaries[i])?,
            2 => r.unique(&d.jsons[i].id, "json", d.jsons[i].span)?,
            _ => {
                let x = &d.relationships[i];
                r.unique(x.id.as_ref().unwrap(), "edge", x.id_span.unwrap())?
            }
        }
    }
    let mut jsons: Vec<_> = d.jsons.iter().collect();
    jsons.sort_by_key(|x| r.first.get(&x.id).copied().unwrap_or(0));
    for x in jsons {
        model.json_nodes.push((
            x.id.clone(),
            UsecaseJsonNode {
                id: x.id.clone(),
                value: x.value.clone(),
                property_order: x.property_order.clone(),
                classes: x.classes.clone(),
                styles: vec![],
            },
        ))
    }
    let mut anonymous = 0;
    let mut edge_index = HashMap::new();
    for x in &d.relationships {
        let source_kind = r.endpoint(&x.source)?;
        let target_kind = r.endpoint(&x.target)?;
        for (end, kind, role) in [
            (&x.source, &source_kind, "source"),
            (&x.target, &target_kind, "target"),
        ] {
            if !["actor", "usecase", "json"].contains(&kind.as_str()) {
                return Err(conflict(
                    &src,
                    format!("Relationship {role} '{}' cannot be {kind}", end.named.id),
                    end.named.span,
                    r.symbols[&end.named.id].span,
                    None,
                    None,
                ));
            }
        }
        let kind = x.relationship_type.as_str();
        if ["include", "extend"].contains(&kind)
            && (source_kind != "usecase" || target_kind != "usecase")
        {
            return Err(fail(
                &src,
                format!("{kind} relationship requires use-case endpoints"),
                x.span,
            ));
        }
        if kind == "generalization" && (source_kind == "json" || source_kind != target_kind) {
            return Err(fail(
                &src,
                "Generalization requires actor-to-actor or use-case-to-use-case endpoints".into(),
                x.span,
            ));
        }
        if kind == "association"
            && (source_kind == "json" || target_kind == "json")
            && x.arrow_type > 2
        {
            return Err(fail(&src,format!("JSON relationship '{}' to '{}' permits only point, reversed-point, or markerless solid association",x.source.named.id,x.target.named.id),x.span));
        }
        let id = x.id.clone().unwrap_or_else(|| {
            let s = format!("edge-{anonymous}");
            anonymous += 1;
            s
        });
        edge_index.insert(id.clone(), model.relationships.len());
        model.relationships.push(Relationship {
            id,
            explicit_id: x.id.as_ref().is_some_and(|s| !s.is_empty()),
            source: x.source.named.id.clone(),
            target: x.target.named.id.clone(),
            relationship_type: x.relationship_type.clone(),
            arrow_type: x.arrow_type,
            label: x.label.as_ref().map(|l| l.text.clone()),
            label_type: x.label.as_ref().map(|l| l.kind.clone()),
            minlen: x.minlen,
            ..Default::default()
        });
    }
    for x in &d.metadata {
        let Some(origin) = r.symbols.get(&x.target) else {
            let suffix = infer(&x.properties)
                .map_or(String::new(), |kind| format!(" (metadata implies {kind})"));
            return Err(fail(
                &src,
                format!("Metadata target '{}' is unresolved{suffix}", x.target),
                x.span,
            ));
        };
        let kind = origin.kind.as_str();
        if kind == "actor" || kind == "usecase" {
            metadata(
                &src,
                r.elements.get_mut(&x.target).unwrap(),
                &x.properties,
                true,
            )?
        } else if kind == "edge" {
            let s = if let Some(child) = x.child {
                &mut statements[x.statement].children.as_mut().unwrap()[child]
            } else {
                &mut statements[x.statement]
            };
            s.kind = "edgeMetadata".into();
            s.edges = Some(vec![EdgeOccurrence {
                id: x.target.clone(),
                span: s.span,
                id_span: Some(x.span),
                metadata: s.metadata.clone(),
                ..Default::default()
            }]);
            s.nodes = None;
        }
        if kind != "actor" && kind != "usecase" {
            for p in &x.properties {
                match (kind, p.occurrence.key.as_str(), &p.value) {
                    ("boundary", "type", PropertyValue::Text(v))
                        if ["rect", "package"].contains(&v.as_str()) =>
                    {
                        r.boundaries.get_mut(&x.target).unwrap().boundary_type = Some(v.clone())
                    }
                    ("edge", "animate", PropertyValue::Bool(v)) => {
                        model.relationships[*edge_index.get(&x.target).unwrap()].animate = *v
                    }
                    ("edge", "animation", PropertyValue::Text(v))
                        if ["fast", "slow"].contains(&v.as_str()) =>
                    {
                        let e = &mut model.relationships[edge_index[&x.target]];
                        e.animation = Some(v.clone());
                        e.animate = true
                    }
                    _ => return Err(invalid(&src, &x.target, kind, p)),
                }
            }
        }
    }
    for e in &mut model.relationships {
        if e.animation.is_some() {
            e.animate = true
        }
    }
    r.element_order
        .sort_by_key(|id| r.first.get(id).copied().unwrap_or(0));
    for id in &r.element_order {
        let state = &r.elements[id];
        let d = &state.draft;
        let business = state.business.unwrap_or(false);
        if d.kind == "actor" {
            if state.icon.as_ref().is_some_and(|s| !s.is_empty())
                && state.actor_type.as_deref().is_some_and(|s| s != "normal")
            {
                return Err(fail(
                    &src,
                    format!(
                        "Actor '{id}' cannot combine icon with type '{}'",
                        state.actor_type.as_ref().unwrap()
                    ),
                    d.named.span,
                ));
            }
            if business
                && (state.icon.as_ref().is_some_and(|s| !s.is_empty())
                    || state.actor_type.as_deref() == Some("awesome"))
            {
                return Err(fail(
                    &src,
                    format!("Business actor '{id}' must use normal or hollow geometry"),
                    d.named.span,
                ));
            }
            model.actors.push((
                id.clone(),
                Actor {
                    id: id.clone(),
                    label: d.named.label.text.clone(),
                    label_type: d.named.label.kind.clone(),
                    actor_type: if state.icon.as_ref().is_some_and(|s| !s.is_empty()) {
                        "icon".into()
                    } else {
                        state.actor_type.clone().unwrap_or_else(|| "normal".into())
                    },
                    icon: state.icon.clone().filter(|s| !s.is_empty()),
                    business,
                    stereotype: d.stereotype.clone().filter(|s| !s.is_empty()),
                    parent_id: d
                        .parent
                        .as_ref()
                        .map(|p| p.named.id.clone())
                        .filter(|s| !s.is_empty()),
                    classes: d.classes.clone(),
                    styles: vec![],
                },
            ));
        } else {
            if d.shape.as_deref() == Some("rect") && business {
                return Err(fail(
                    &src,
                    format!("Rectangular use case '{id}' cannot be a business use case"),
                    d.named.span,
                ));
            }
            model.use_cases.push((
                id.clone(),
                UseCase {
                    id: id.clone(),
                    label: d.named.label.text.clone(),
                    label_type: d.named.label.kind.clone(),
                    shape: d.shape.clone().unwrap_or_else(|| "ellipse".into()),
                    business,
                    stereotype: d.stereotype.clone().filter(|s| !s.is_empty()),
                    parent_id: d
                        .parent
                        .as_ref()
                        .map(|p| p.named.id.clone())
                        .filter(|s| !s.is_empty()),
                    classes: d.classes.clone(),
                    styles: vec![],
                },
            ));
        }
    }
    let mut elements: Vec<_> = d.elements.iter().collect();
    elements.sort_by_key(|x| x.named.span[0]);
    let mut member_sets: HashMap<String, HashSet<String>> = HashMap::new();
    for e in elements {
        if let Some(parent) = &e.parent {
            if !parent.named.id.is_empty() {
                if member_sets
                    .entry(parent.named.id.clone())
                    .or_default()
                    .insert(e.named.id.clone())
                {
                    r.boundaries
                        .get_mut(&parent.named.id)
                        .unwrap()
                        .members
                        .push(e.named.id.clone())
                }
            }
        }
    }
    r.boundary_order
        .sort_by_key(|id| r.first.get(id).copied().unwrap_or(0));
    for id in &r.boundary_order {
        let s = &r.boundaries[id];
        model.system_boundaries.push((
            id.clone(),
            SystemBoundary {
                id: id.clone(),
                label: s.draft.named.label.text.clone(),
                label_type: s.draft.named.label.kind.clone(),
                boundary_type: s.boundary_type.clone().unwrap_or_else(|| "rect".into()),
                members: s.members.clone(),
                classes: s.draft.classes.clone(),
                styles: vec![],
            },
        ));
    }
    for (i, (target, span, label)) in d.notes.iter().enumerate() {
        let Some(origin) = r.symbols.get(target) else {
            return Err(fail(
                &src,
                format!("Note target '{target}' is unresolved"),
                *span,
            ));
        };
        if origin.kind != "actor" && origin.kind != "usecase" {
            return Err(conflict(
                &src,
                format!(
                    "Note target '{target}' must be an actor or use case, not {}",
                    origin.kind
                ),
                *span,
                origin.span,
                None,
                None,
            ));
        }
        let id = format!("note-{i}");
        model.notes.push((
            id.clone(),
            UsecaseNote {
                id,
                target: target.clone(),
                label: label.text.clone(),
                label_type: label.kind.clone(),
            },
        ));
    }
    let mut class_index = HashMap::new();
    for (ids, styles) in d.class_defs {
        let styles = std::sync::Arc::new(styles);
        for id in ids {
            let def = ClassDef {
                id: id.clone(),
                styles: styles.clone(),
            };
            if let Some(&i) = class_index.get(&id) {
                model.class_defs[i] = (id, def)
            } else {
                class_index.insert(id.clone(), model.class_defs.len());
                model.class_defs.push((id, def));
            }
        }
    }
    let actor_index: HashMap<_, _> = model
        .actors
        .iter()
        .enumerate()
        .map(|(i, (id, _))| (id.clone(), i))
        .collect();
    let case_index: HashMap<_, _> = model
        .use_cases
        .iter()
        .enumerate()
        .map(|(i, (id, _))| (id.clone(), i))
        .collect();
    let boundary_index: HashMap<_, _> = model
        .system_boundaries
        .iter()
        .enumerate()
        .map(|(i, (id, _))| (id.clone(), i))
        .collect();
    let json_index: HashMap<_, _> = model
        .json_nodes
        .iter()
        .enumerate()
        .map(|(i, (id, _))| (id.clone(), i))
        .collect();
    let mut assign = |id: &str,
                      span: Span,
                      classes: Option<&[String]>,
                      styles: Option<&[String]>|
     -> Result<(), PeleError> {
        let (c, s) = match r.symbols.get(id).map(|o| o.kind.as_str()) {
            Some("actor") => {
                let a = &mut model.actors[actor_index[id]].1;
                (&mut a.classes, &mut a.styles)
            }
            Some("usecase") => {
                let a = &mut model.use_cases[case_index[id]].1;
                (&mut a.classes, &mut a.styles)
            }
            Some("boundary") => {
                let a = &mut model.system_boundaries[boundary_index[id]].1;
                (&mut a.classes, &mut a.styles)
            }
            Some("json") => {
                let a = &mut model.json_nodes[json_index[id]].1;
                (&mut a.classes, &mut a.styles)
            }
            _ => {
                let Some(&i) = edge_index.get(id) else {
                    return Err(fail(
                        &src,
                        format!("Class/style target '{id}' is unresolved or anonymous"),
                        span,
                    ));
                };
                let a = &mut model.relationships[i];
                (&mut a.classes, &mut a.styles)
            }
        };
        if let Some(classes) = classes {
            unique(c, classes)
        }
        if let Some(styles) = styles {
            s.extend_from_slice(styles)
        }
        Ok(())
    };
    for (targets, classes) in &d.classes {
        for (id, span) in targets {
            assign(id, *span, Some(classes), None)?
        }
    }
    for (id, span, styles) in &d.styles {
        assign(id, *span, None, Some(styles))?
    }
    model.direction = d.direction.unwrap_or(model.direction);
    model.title = title;
    model.acc_title = d.acc_title;
    model.acc_description = d.acc_description;
    model.acc_descr = description(&model.acc_description);
    model.source = source.into();
    model.header_span = header;
    model.statements = statements;
    Ok(model)
}

fn description(s: &str) -> String {
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
    out
}
