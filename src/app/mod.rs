pub mod modrinth;

use std::{collections::HashMap, sync::Arc};

use crate::error_template::{AppError, ErrorTemplate};
use ferinth::structures::project::Project;
use itertools::Itertools;
use leptos::{html::Input, *};
use leptos_meta::*;
use leptos_router::*;

use self::modrinth::{Collection, ProjectKey};

#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    view! {
        // injects a stylesheet into the document <head>
        // id=leptos means cargo-leptos will hot-reload this stylesheet
        <Stylesheet id="leptos" href="/pkg/mr-modpack.css"/>

        // sets the document title
        <Title text="Welcome to Leptos"/>

        // content for this welcome page
        <Router fallback=|| {
            let mut outside_errors = Errors::default();
            outside_errors.insert_with_default_key(AppError::NotFound);
            view! {
                <ErrorTemplate outside_errors/>
            }
            .into_view()
        }>
            <main>
                <Routes>
                    <Route path="" view=HomePage/>
                </Routes>
            </main>
        </Router>
    }
}

/// Renders the home page of your application.
#[component]
fn HomePage() -> impl IntoView {
    let input: NodeRef<Input> = create_node_ref();

    let collection_id = create_rw_signal(None);
    let collection = create_local_resource(
        move || collection_id.get(),
        move |id| async move {
            match id {
                Some(id) => {
                    let collection = get_collection(id).await?;

                    let projects = get_projects(collection.projects.clone()).await?;

                    let mut available_versions: HashMap<String, Vec<Project>> = HashMap::new();

                    for project in projects.iter() {
                        for version in &project.game_versions {
                            available_versions
                                .entry(version.clone())
                                .and_modify(|projects| {
                                    projects.push(project.clone());
                                })
                                .or_insert_with(|| vec![project.clone()]);
                        }

                        // println!(
                        //     "{} ({})",
                        //     project.title,
                        //     project.game_versions.last().unwrap()
                        // );
                    }

                    Ok::<_, ServerFnError>(Some((
                        collection,
                        available_versions
                            .into_iter()
                            .sorted_by_key(|(_, projects)| projects.len())
                            .rev()
                            .collect::<Vec<_>>(),
                    )))
                }
                None => Ok(None),
            }
        },
    );

    view! {
        <h1>"Mr Modpack"</h1>

        <form on:submit=move |ev| {
            ev.prevent_default();

            let input = input().expect("<input> hasn't been mounted");

            let val = input.value().trim().to_string();
            collection_id.set(Some(val));

            // input.set_value("");
        }>
            <input type="text" placeholder="Type a Modrinth Collection ID" node_ref=input/>
        </form>

        <Suspense
            fallback=|| view! {
                <p>
                    "Loading..."
                </p>
            }
        >
            {move || collection.get().map(|c| c.map(|c| c.map(|(collection, projects)| view! {
                <h2>{collection.name}</h2>

                {projects.into_iter().map(|(version, projects)| view! {
                    <h3>
                        {version}
                        " - "
                        {format!("{:.2}", (projects.len() as f64 / collection.projects.len() as f64) * 100.0)}
                        "% ("
                        {projects.len()}
                        " / "
                        {collection.projects.len()}
                        ")"
                    </h3>
                    // <ul>
                    {projects.into_iter().map(|project| view! {
                        <p>
                            {project.title}
                        </p>
                    }).collect_view()}
                    // </ul>
                }).collect_view()}
            })))}
        </Suspense>
    }
}

#[server]
async fn get_projects(projects: Vec<ProjectKey>) -> Result<Vec<Project>, ServerFnError> {
    let api: Arc<modrinth::ModrinthClient> = use_context().unwrap();

    let all_projects = api.global_projects.read().await;

    let mut res = Vec::with_capacity(projects.len());

    for project in projects {
        res.push(all_projects[project.0].clone());
    }

    Ok(res)
}

#[server]
async fn get_collection(collection_id: String) -> Result<Collection, ServerFnError> {
    let api: Arc<modrinth::ModrinthClient> = use_context().unwrap();

    api.get_collection(&collection_id)
        .await
        .map_err(ServerFnError::new)
}
