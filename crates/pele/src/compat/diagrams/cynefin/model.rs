//! Cynefin ordered domains and transitions, Pele 458322a5 (MIT).
use serde::Serialize;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DomainName {
    Complex,
    Complicated,
    Clear,
    Chaotic,
    Confusion,
}
impl DomainName {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Complex => "complex",
            Self::Complicated => "complicated",
            Self::Clear => "clear",
            Self::Chaotic => "chaotic",
            Self::Confusion => "confusion",
        }
    }
    pub(super) fn parse(s: &str) -> Self {
        match s {
            "complex" => Self::Complex,
            "complicated" => Self::Complicated,
            "clear" => Self::Clear,
            "chaotic" => Self::Chaotic,
            _ => Self::Confusion,
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct CynefinItem {
    pub label: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct CynefinDomain {
    pub name: DomainName,
    pub items: Vec<CynefinItem>,
}
#[derive(Clone, Debug, Serialize)]
pub struct CynefinTransition {
    pub from: DomainName,
    pub to: DomainName,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CynefinModel {
    #[serde(rename = "type")]
    pub diagram_type: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acc_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acc_descr: Option<String>,
    pub domains: Vec<(DomainName, CynefinDomain)>,
    pub transitions: Vec<CynefinTransition>,
}
impl CynefinModel {
    pub fn domain(&self, name: DomainName) -> Option<&CynefinDomain> {
        self.domains
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, d)| d)
    }
}
