use bytes::Bytes;
use ferinth::{
    structures::{project::Project, version::Version},
    Ferinth,
};
use reqwest::{Client, ClientBuilder, IntoUrl};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use super::{Collection, ProjectID, ProjectKey, UserID};

const MODRINTH_ENDPOINT: &str = "https://api.modrinth.com/v3/";

#[derive(Debug, thiserror::Error)]
pub(crate) enum ApiErr {
    #[error("reqwest error: {0}")]
    Reqwest(reqwest::Error),
    #[error("json parse error: {0}")]
    Json(serde_json::Error),
    #[error("ferinth error: {0}")]
    Ferinth(ferinth::Error),
    #[error("not found")]
    NotFound,
}

pub(crate) type ApiResult<T> = Result<T, ApiErr>;

#[derive(Debug, Serialize, Deserialize)]
struct InnerCollection {
    id: String,
    user: UserID,
    name: String,
    description: String,
    projects: Vec<ProjectID>,
}

#[derive(Debug)]
pub struct ModrinthClient {
    v2: Ferinth,
    v3: Client,
    pub(crate) global_projects: RwLock<Vec<Project>>,
}

impl Default for ModrinthClient {
    fn default() -> Self {
        Self::new(
            env!("CARGO_PKG_NAME"),
            Some(env!("CARGO_PKG_VERSION")),
            Some("notaflyinggoose@gmail.com"),
        )
    }
}

impl ModrinthClient {
    pub fn new(name: &str, version: Option<&str>, contact: Option<&str>) -> Self {
        let mut user_agent = name.to_string();

        if let Some(version) = version {
            user_agent.push('/');
            user_agent.push_str(version);
        }

        if let Some(contact) = contact {
            user_agent.push_str(" (");
            user_agent.push_str(contact);
            user_agent.push(')');
        }

        Self {
            v2: Ferinth::new(name, version, contact, None).unwrap(),
            v3: ClientBuilder::default()
                .user_agent(user_agent)
                .build()
                .unwrap(),
            global_projects: Default::default(),
        }
    }

    pub(crate) async fn download_file<U>(&self, url: U) -> ApiResult<Bytes>
    where
        U: IntoUrl,
    {
        self.v3
            .get(url)
            .send()
            .await
            .map_err(ApiErr::Reqwest)?
            .bytes()
            .await
            .map_err(ApiErr::Reqwest)
    }

    pub(crate) async fn get_project_versions(
        &self,
        id: &str,
        loaders: &[&str],
        game_versions: &[&str],
    ) -> ApiResult<Vec<Version>> {
        self.v2
            .list_versions_filtered(id, Some(loaders), Some(game_versions), None)
            .await
            .map_err(ApiErr::Ferinth)
    }

    pub(crate) async fn get_version(&self, id: &str) -> ApiResult<Version> {
        self.v2.get_version(id).await.map_err(ApiErr::Ferinth)
    }

    pub(crate) async fn get_project(&self, id: &str) -> ApiResult<ProjectKey> {
        let project = self.v2.get_project(id).await.map_err(ApiErr::Ferinth)?;

        let mut global_projects = self.global_projects.write().await;

        global_projects.push(project);

        Ok(ProjectKey(global_projects.len() - 1))
    }

    pub(crate) async fn get_collection(&self, id: &str) -> ApiResult<Collection> {
        let response = self
            .v3
            .get(format!("{MODRINTH_ENDPOINT}collection/{}", id))
            .send()
            .await
            .map_err(ApiErr::Reqwest)?;

        if !response.status().is_success() {
            match response.status().as_u16() {
                404 => return Err(ApiErr::NotFound),
                other => panic!("api returned error code {other}"),
            }
        }

        let body = response.text().await.map_err(ApiErr::Reqwest)?;

        let pre: InnerCollection = serde_json::from_str(&body).map_err(ApiErr::Json)?;

        let mut projects = Vec::with_capacity(pre.projects.len());

        for project in pre.projects {
            let project = self.get_project(project.as_ref()).await?;

            projects.push(project);
        }

        Ok(Collection {
            id: pre.id,
            name: pre.name,
            user: pre.user,
            description: pre.description,
            projects,
        })
    }
}
