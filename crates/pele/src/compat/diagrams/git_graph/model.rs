//! Pele git/db.ts ordered commit database, 458322a5 (MIT, kepano).
use crate::compat::{source::space, DiagramType, ErrorCode, Object, PeleError, Value};
use serde::Serialize;
use std::collections::HashMap;
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommit {
    pub id: String,
    pub message: String,
    pub seq: usize,
    #[serde(rename = "type")]
    pub commit_type: u8,
    pub tags: Vec<String>,
    pub parents: Vec<String>,
    pub branch: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_type: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_id: Option<bool>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitModel {
    #[serde(rename = "type")]
    pub diagram_type: DiagramType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acc_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acc_descr: Option<String>,
    pub direction: String,
    pub commits: Vec<(String, GitCommit)>,
    pub branches: Vec<(String, Option<String>)>,
    pub lanes: Vec<String>,
    pub current_branch: String,
    pub head: Option<GitCommit>,
    pub warnings: Vec<String>,
}
pub(super) enum Statement {
    Commit {
        id: Option<String>,
        message: Option<String>,
        tags: Vec<String>,
        kind: Option<u8>,
    },
    Branch {
        name: String,
        order: Option<f64>,
    },
    Merge {
        branch: String,
        id: Option<String>,
        tags: Vec<String>,
        kind: Option<u8>,
    },
    Checkout(String),
    CherryPick {
        id: Option<String>,
        tags: Vec<String>,
        parent: Option<String>,
    },
}
#[derive(Default)]
pub(super) struct Ast {
    pub direction: Option<String>,
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub statements: Vec<Statement>,
}
fn fail(message: impl Into<String>) -> PeleError {
    let mut e = PeleError::new(message, ErrorCode::Semantic);
    e.diagram_type = Some("gitGraph".into());
    e
}
struct Builder {
    model: GitModel,
    commits: HashMap<String, usize>,
    branches: HashMap<String, usize>,
    orders: Vec<Option<f64>>,
    seq: usize,
}
impl Builder {
    fn next_id(&self) -> String {
        let mut salt = self.seq as u32 + 1;
        loop {
            let hex = format!("{:08x}", salt.wrapping_mul(0x9e3779b1));
            let id = format!("{}-{}", self.seq, &hex[..7]);
            if !self.commits.contains_key(&id) {
                return id;
            }
            salt = salt.wrapping_add(0x9e3779b1)
        }
    }
    fn checkout(&mut self, name: &str) -> Result<(), PeleError> {
        let Some(&i) = self.branches.get(name) else {
            return Err(fail(format!("Trying to checkout branch which is not yet created. (Help try using \"branch {name}\")")));
        };
        self.model.current_branch = name.into();
        self.model.head = self.model.branches[i]
            .1
            .as_ref()
            .filter(|s| !s.is_empty())
            .and_then(|id| self.commits.get(id))
            .map(|&i| self.model.commits[i].1.clone());
        Ok(())
    }
    fn branch_commit(&self, name: &str) -> Option<&GitCommit> {
        self.branches
            .get(name)
            .and_then(|&i| self.model.branches[i].1.as_ref())
            .filter(|s| !s.is_empty())
            .and_then(|id| self.commits.get(id))
            .map(|&i| &self.model.commits[i].1)
    }
    fn add(&mut self, commit: GitCommit) {
        self.model.head = Some(commit.clone());
        self.model.branches[self.branches[&self.model.current_branch]].1 = Some(commit.id.clone());
        if let Some(&i) = self.commits.get(&commit.id) {
            self.model.commits[i].1 = commit
        } else {
            self.commits
                .insert(commit.id.clone(), self.model.commits.len());
            self.model.commits.push((commit.id.clone(), commit));
        }
    }
    fn statement(&mut self, s: Statement) -> Result<(), PeleError> {
        match s {
            Statement::Checkout(name) => self.checkout(&name)?,
            Statement::Commit {
                id,
                message,
                tags,
                kind,
            } => {
                let id = id
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| self.next_id());
                let commit = GitCommit {
                    id,
                    message: message.unwrap_or_default(),
                    seq: self.seq,
                    commit_type: kind.unwrap_or(0),
                    tags,
                    parents: self
                        .model
                        .head
                        .as_ref()
                        .map_or_else(Vec::new, |h| vec![h.id.clone()]),
                    branch: self.model.current_branch.clone(),
                    custom_type: None,
                    custom_id: None,
                };
                self.seq += 1;
                if self.commits.contains_key(&commit.id) {
                    self.model
                        .warnings
                        .push(format!("Commit ID {} already exists", commit.id))
                }
                self.add(commit)
            }
            Statement::Branch { name, order } => {
                if self.branches.contains_key(&name) {
                    return Err(fail(format!("Trying to create an existing branch. (Help: Either use a new name if you want create a new branch or try using \"checkout {name}\")")));
                }
                self.branches
                    .insert(name.clone(), self.model.branches.len());
                self.model
                    .branches
                    .push((name.clone(), self.model.head.as_ref().map(|h| h.id.clone())));
                self.orders.push(Some(order.unwrap_or(0.)));
                self.checkout(&name)?
            }
            Statement::Merge {
                branch: other,
                id,
                tags,
                kind,
            } => {
                let current = &self.model.current_branch;
                let a = self.branch_commit(current);
                let b = self.branch_commit(&other);
                if a.is_some() && b.is_some() && a.unwrap().branch == other {
                    return Err(fail(format!("Cannot merge branch '{other}' into itself.")));
                }
                if current == &other {
                    return Err(fail(
                        "Incorrect usage of \"merge\". Cannot merge a branch to itself",
                    ));
                }
                let Some(a) = a else {
                    return Err(fail(format!(
                        "Incorrect usage of \"merge\". Current branch ({current})has no commits"
                    )));
                };
                if !self.branches.contains_key(&other) {
                    return Err(fail(format!("Incorrect usage of \"merge\". Branch to be merged ({other}) does not exist")));
                }
                let Some(b) = b else {
                    return Err(fail(format!("Incorrect usage of \"merge\". Branch to be merged ({other}) has no commits")));
                };
                if a.id == b.id {
                    return Err(fail(
                        "Incorrect usage of \"merge\". Both branches have same head",
                    ));
                }
                let custom = id.filter(|s| !s.is_empty());
                if let Some(id) = &custom {
                    if self.commits.contains_key(id) {
                        return Err(fail(format!("Incorrect usage of \"merge\". Commit with id:{id} already exists, use different custom id")));
                    }
                }
                let other_head = b.id.clone();
                let commit = GitCommit {
                    id: custom.clone().unwrap_or_else(|| self.next_id()),
                    message: format!("merged branch {other} into {current}"),
                    seq: self.seq,
                    commit_type: 3,
                    tags,
                    parents: self
                        .model
                        .head
                        .as_ref()
                        .map_or_else(Vec::new, |h| vec![h.id.clone(), other_head]),
                    branch: current.clone(),
                    custom_type: kind,
                    custom_id: Some(custom.is_some()),
                };
                self.seq += 1;
                self.add(commit)
            }
            Statement::CherryPick { id, tags, parent } => {
                let source = id
                    .as_ref()
                    .filter(|s| !s.is_empty())
                    .and_then(|id| self.commits.get(id))
                    .map(|&i| &self.model.commits[i].1);
                let Some(source) = source else {
                    return Err(fail("Incorrect usage of \"cherryPick\". Source commit id should exist and provided"));
                };
                if parent
                    .as_ref()
                    .is_some_and(|s| !s.is_empty() && !source.parents.contains(s))
                {
                    return Err(fail("Invalid operation: The specified parent commit is not an immediate parent of the cherry-picked commit."));
                }
                if source.commit_type == 3 && parent.as_ref().is_none_or(|s| s.is_empty()) {
                    return Err(fail("Incorrect usage of cherry-pick: If the source commit is a merge commit, an immediate parent commit must be specified."));
                }
                let current = &self.model.current_branch;
                if &source.branch == current {
                    return Err(fail("Incorrect usage of \"cherryPick\". Source commit is already on current branch"));
                }
                if self.branch_commit(current).is_none() {
                    return Err(fail(format!("Incorrect usage of \"cherry-pick\". Current branch ({current})has no commits")));
                }
                let tags = if tags.is_empty() {
                    vec![format!(
                        "cherry-pick:{}{}",
                        source.id,
                        if source.commit_type == 3 {
                            format!("|parent:{}", parent.as_deref().unwrap_or(""))
                        } else {
                            String::new()
                        }
                    )]
                } else {
                    tags.into_iter().filter(|s| !s.is_empty()).collect()
                };
                let commit = GitCommit {
                    id: self.next_id(),
                    message: format!("cherry-picked {} into {current}", source.message),
                    seq: self.seq,
                    commit_type: 4,
                    tags,
                    parents: self
                        .model
                        .head
                        .as_ref()
                        .map_or_else(Vec::new, |h| vec![h.id.clone(), source.id.clone()]),
                    branch: current.clone(),
                    custom_type: None,
                    custom_id: None,
                };
                self.seq += 1;
                self.add(commit)
            }
        }
        Ok(())
    }
}
pub(super) fn build(ast: Ast, config: &Object, title: Option<&str>) -> Result<GitModel, PeleError> {
    let git = config.get("gitGraph").and_then(Value::as_object);
    let main = git
        .and_then(|g| g.get("mainBranchName"))
        .and_then(|v| {
            if let Value::String(s) = v {
                Some(s.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "main".into());
    let order = git
        .and_then(|g| g.get("mainBranchOrder"))
        .and_then(|v| {
            if let Value::Number(n) = v {
                Some(*n)
            } else {
                None
            }
        })
        .unwrap_or(0.);
    let mut b = Builder {
        model: GitModel {
            diagram_type: DiagramType::GitGraph,
            title: ast
                .title
                .filter(|s| !s.is_empty())
                .or_else(|| title.map(str::to_string)),
            acc_title: ast
                .acc_title
                .filter(|s| !s.is_empty())
                .map(|s| s.trim_start_matches(space).into()),
            acc_descr: ast.acc_descr.filter(|s| !s.is_empty()).map(|s| {
                let mut out = String::new();
                let mut after = false;
                for c in s.chars() {
                    if after && space(c) {
                        continue;
                    }
                    out.push(c);
                    after = c == '\n'
                }
                out
            }),
            direction: ast.direction.unwrap_or_else(|| "LR".into()),
            commits: Vec::new(),
            branches: vec![(main.clone(), None)],
            lanes: Vec::new(),
            current_branch: main.clone(),
            head: None,
            warnings: Vec::new(),
        },
        commits: HashMap::new(),
        branches: HashMap::from([(main, 0)]),
        orders: vec![Some(order)],
        seq: 0,
    };
    for s in ast.statements {
        b.statement(s)?
    }
    let mut lanes: Vec<_> = b
        .model
        .branches
        .iter()
        .enumerate()
        .map(|(i, (name, _))| {
            (
                name.clone(),
                b.orders[i].unwrap_or_else(|| format!("0.{i}").parse().unwrap_or(0.)),
            )
        })
        .collect();
    lanes.sort_by(|a, b| {
        (a.1 - b.1)
            .partial_cmp(&0.)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    b.model.lanes = lanes.into_iter().map(|(name, _)| name).collect();
    Ok(b.model)
}
