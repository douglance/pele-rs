//! Pele treeview/icons.ts, 458322a5 (MIT, kepano).
use super::{NodeType, TreeNode};
use crate::compat::{Object, Value};
pub(super) const PACK: &str = "mermaid-treeview";
const FOLDER:&str="M3 6.5A1.5 1.5 0 0 1 4.5 5h4.6l2.4 2.5h8A1.5 1.5 0 0 1 21 9v9a1.5 1.5 0 0 1-1.5 1.5h-15A1.5 1.5 0 0 1 3 18z";
const FILE:&str="M6.5 3h7L19 8.5v11a1.5 1.5 0 0 1-1.5 1.5h-11A1.5 1.5 0 0 1 5 19.5v-15A1.5 1.5 0 0 1 6.5 3zM13.5 3v5.5H19";
pub(super) struct Config<'a> {
    pub show: bool,
    pub pack: &'a str,
    pub filename: Option<&'a Object>,
    pub extension: Option<&'a Object>,
}
impl<'a> Config<'a> {
    pub fn new(config: &'a Object) -> Self {
        let c = config.get("treeView").and_then(Value::as_object);
        Self {
            show: c
                .and_then(|c| c.get("showIcons"))
                .is_some_and(Value::truthy),
            pack: c
                .and_then(|c| c.get("defaultIconPack"))
                .and_then(|v| match v {
                    Value::String(s) => Some(s.as_str()),
                    _ => None,
                })
                .unwrap_or(""),
            filename: c
                .and_then(|c| c.get("filenameIcons"))
                .and_then(Value::as_object),
            extension: c
                .and_then(|c| c.get("extensionIcons"))
                .and_then(Value::as_object),
        }
    }
}
fn own<'a>(map: Option<&'a Object>, key: &str) -> Option<&'a str> {
    map.and_then(|map| map.get(key)).and_then(|v| match v {
        Value::String(s) => Some(s.as_str()),
        _ => None,
    })
}
fn qualify(icon: &str, c: &Config<'_>) -> String {
    if icon.contains(':') {
        icon.into()
    } else if matches!(icon, "file" | "folder") || c.pack.is_empty() {
        format!("{PACK}:{icon}")
    } else {
        format!("{}:{icon}", c.pack)
    }
}
pub(super) fn get(node: &TreeNode, c: &Config<'_>) -> Option<String> {
    if node.icon.as_deref() == Some("none") {
        return None;
    }
    if let Some(icon) = node.icon.as_deref().filter(|s| !s.is_empty()) {
        return Some(qualify(icon, c));
    }
    if !c.show {
        return None;
    }
    if node.node_type == NodeType::File {
        let by_name = own(c.filename, &node.name).filter(|s| !s.is_empty());
        let detected = by_name.or_else(|| {
            let dot = node.name.rfind('.')?;
            if dot == 0 {
                return None;
            }
            let ext = node.name[dot..].to_lowercase();
            own(c.extension, &ext).or_else(|| own(c.extension, &ext[1..]))
        });
        if detected == Some("none") {
            return None;
        }
        if let Some(icon) = detected.filter(|s| !s.is_empty()) {
            return Some(qualify(icon, c));
        }
    }
    Some(format!(
        "{PACK}:{}",
        if node.node_type == NodeType::Directory {
            "folder"
        } else {
            "file"
        }
    ))
}
pub(super) fn glyph(icon: &str, directory: bool) -> &'static str {
    match icon.strip_prefix("mermaid-treeview:") {
        Some("folder") => FOLDER,
        Some("file") => FILE,
        _ => {
            if directory {
                FOLDER
            } else {
                FILE
            }
        }
    }
}
