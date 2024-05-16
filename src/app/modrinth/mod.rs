#[cfg(feature = "ssr")]
mod api;

#[cfg(feature = "ssr")]
pub use api::*;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectID(String);

impl AsRef<str> for ProjectID {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UserID(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProjectKey(pub usize);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub user: UserID,
    pub name: String,
    pub description: String,
    pub projects: Vec<ProjectKey>,
}
