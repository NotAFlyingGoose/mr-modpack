pub mod modrinth;

use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    path::Path,
    rc::Rc,
    str::FromStr,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::error_template::{AppError, ErrorTemplate};
use ferinth::structures::{project::Project, version::DependencyType, ID};
use itertools::Itertools;
use leptos::{
    html::{Iframe, Input},
    leptos_dom::logging::console_log,
    *,
};
use leptos_meta::*;
use leptos_router::*;
use leptos_use::{use_cookie, utils::JsonCodec};
use serde::{Deserialize, Serialize};

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
        <Title text="Mr Modpack"/>

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

    let (collections, set_collections) =
        use_cookie::<Vec<String>, JsonCodec>("modrinth_collections");

    if collections.get_untracked().is_none() {
        set_collections(Default::default());
    }

    view! {
        <h1>"Mr Modpack"</h1>

        <form on:submit=move |ev| {
            ev.prevent_default();

            let input = input().expect("<input> hasn't been mounted");

            let val = input.value().trim().to_string();

            if val.is_empty() {
                return;
            }

            set_collections.update(|collections| {
                if let Some(collections) = collections {
                    if collections.iter().all(|c| *c != val) {
                        console_log("pushing collection");
                        collections.push(val);
                    }
                } else {
                    console_log("setting collection");
                    *collections = Some(vec![val]);
                }
            });

            input.set_value("");
        }>
            <input type="text" class="search" placeholder="Type a Modrinth Collection ID" node_ref=input/>
        </form>

        <div id="content">
            <For
                // a function that returns the items we're iterating over; a signal is fine
                each=move || collections.get().unwrap_or_default().into_iter().rev()
                // a unique key for each item
                key=|id| id.clone()
                // renders each item to a view
                let:id
            >
                <Collection id set_collections/>
            </For>
        </div>
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
struct ReleaseVersion {
    major: u32,
    minor: u32,
    patch: u32,
}

impl FromStr for ReleaseVersion {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let first_dot = s.find('.').ok_or(())?;
        let major: u32 = s[..first_dot].parse().map_err(|_| ())?;
        let rest = &s[first_dot + 1..];

        let second_dot = rest.find('.').ok_or(())?;
        let minor: u32 = rest[..second_dot].parse().map_err(|_| ())?;
        let rest = &rest[second_dot + 1..];

        let patch: u32 = rest.parse().map_err(|_| ())?;

        Ok(ReleaseVersion {
            major,
            minor,
            patch,
        })
    }
}

impl Display for ReleaseVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[component]
fn Collection(id: String, set_collections: WriteSignal<Option<Vec<String>>>) -> impl IntoView {
    let cloned_id = id.clone();
    let collection = create_local_resource(
        move || cloned_id.clone(),
        move |id| async move {
            let collection = get_collection(id).await?;

            let projects = get_projects(collection.projects.clone()).await?;

            let mut available_versions: HashMap<ReleaseVersion, HashSet<ProjectKey>> =
                HashMap::new();

            for (key, project) in projects.iter() {
                for version in project
                    .game_versions
                    .iter()
                    .filter_map(|v| v.parse::<ReleaseVersion>().ok())
                {
                    available_versions
                        .entry(version)
                        .and_modify(|projects| {
                            projects.insert(*key);
                        })
                        .or_insert_with(|| {
                            let mut p = HashSet::with_capacity(1);
                            p.insert(*key);
                            p
                        });
                }
            }

            Ok::<_, ServerFnError>((
                collection,
                projects,
                available_versions
                    .into_iter()
                    .sorted_by_key(|(_, projects)| projects.len())
                    .rev()
                    .collect::<Vec<_>>(),
            ))
        },
    );

    let close: Rc<dyn Fn()> = Rc::new(move || {
        let cloned_id = id.clone();
        set_collections.update(move |collections| {
            if let Some(collections) = collections {
                collections.remove(
                    collections
                        .iter()
                        .find_position(|c| *c == &cloned_id)
                        .unwrap()
                        .0,
                );
            }
        });
    });

    view! {
        <Suspense
            fallback=|| view! {
                <p>
                    "Loading..."
                </p>
            }
        >
            {
                let close = close.clone();
                move || {
                    let close = close.clone();
                    collection.get().map(move |c| c.map(move |(collection, projects, available_versions)| {
                    let collection_name = collection.name.clone();

                    view! {
                    <h2>{collection.name}</h2>
                    <p class="collection-id">{collection.id}</p>

                    <Spoiler close=close.clone()>
                    <div class="collection-table">
                    <table>
                        <tbody>
                            <tr>
                                <th>
                                    "Mod"
                                </th>
                                {available_versions.clone().into_iter().map(|(version, projects)| {
                                    let collection_name = collection_name.clone();
                                    let projects_2 = projects.clone();
                                    let download_loading = create_rw_signal(false);
                                    view! {
                                    <td>
                                        <span class="version">
                                            {version.to_string()}
                                        </span>
                                        <span class="percentage">
                                            {format!("{:.1}", (projects.len() as f64 / collection.projects.len() as f64) * 100.0)}
                                            "%"
                                        </span>
                                        <button
                                            class={move || if download_loading.get() {
                                                "download downloading"
                                            } else {
                                                "download"
                                            }}
                                            on:click=move |_| {
                                                if download_loading.get_untracked() {
                                                    return;
                                                }

                                                let collection_name = collection_name.clone();
                                                let projects_2 = projects_2.clone();
                                                download_loading.set(true);

                                                spawn_local(async move {
                                                    let zip = download_zip(collection_name.clone(), version, projects_2.clone()).await.unwrap();

                                                    download_loading.set(false);

                                                    window().open_with_url(&zip).unwrap();
                                                });
                                            }
                                        >
                                            {move || if download_loading.get() {
                                                "Downloading..."
                                            } else {
                                                "Download all"
                                            }}
                                        </button>

                                    </td>
                                }}).collect_view()}
                            </tr>

                            {projects.into_iter().map(|(key, project)| view! {
                                <tr>
                                    <th><a href={format!("https://modrinth.com/mod/{}", project.slug)} target="_blank">
                                        {project.title}
                                    </a></th>
                                    {available_versions.iter().map(|(_, projects)| view! {
                                        <td>
                                            {if projects.contains(&key) {
                                                "✅"
                                            } else {
                                                "❌"
                                            }}
                                        </td>
                                    }).collect_view()}
                                </tr>
                            }).collect_view()}
                        </tbody>
                    </table>
                    </div>
                    </Spoiler>
                }}))
            }}
        </Suspense>
    }
}

#[component]
fn Spoiler(close: Rc<dyn Fn()>, children: Children) -> impl IntoView {
    let visible = create_rw_signal(true);

    view! {
        <button class="margin-all" on:click=move |_| visible.update(|visible| *visible = !*visible)>
            {move || if visible.get() { "Hide" } else { "Show" }}
        </button>
        <button class="margin-all" on:click=move |_| close()>
            "Remove"
        </button>
        <div class={move || if visible.get() { "spoiler" } else { "spoiler hidden" }}>
            {children()}
        </div>
    }
}

#[server]
async fn get_projects(
    projects: Vec<ProjectKey>,
) -> Result<Vec<(ProjectKey, Project)>, ServerFnError> {
    let api: Arc<modrinth::ModrinthClient> = use_context().unwrap();

    let all_projects = api.global_projects.read().await;

    let mut res = Vec::with_capacity(projects.len());

    for project in projects {
        res.push((project, all_projects[project.0].clone()));
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

const LOADERS: &[&str] = &["fabric", "quilt"];

#[server]
async fn download_zip(
    collection_name: String,
    release_version: ReleaseVersion,
    projects: HashSet<ProjectKey>,
) -> Result<String, ServerFnError> {
    use async_zip::{base::write::ZipFileWriter, Compression, ZipEntryBuilder};

    let api: Arc<modrinth::ModrinthClient> = use_context().unwrap();

    let game_version = release_version.to_string();
    let game_versions: &[&str] = &[&game_version];

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let opts: LeptosOptions = use_context().unwrap();
    let output_folder = AsRef::<Path>::as_ref(&opts.site_root).join("temp-download-all");
    let _ = tokio::fs::create_dir(&output_folder).await;

    let filename = output_folder.join(format!("{}-{now}.zip", collection_name));
    let mut zip = tokio::fs::File::create(&filename).await.unwrap();
    let mut zip = ZipFileWriter::with_tokio(&mut zip);

    let mut downloaded = HashSet::new();

    let mut todo = projects.into_iter().map(|p| (p, 0)).collect_vec();

    // todo: do multiple downloads simultaneously
    while let Some((project, ident)) = todo.pop() {
        let global_projects = api.global_projects.read().await;
        let project = &global_projects[project.0];

        let versions = api
            .get_project_versions(&project.slug, LOADERS, game_versions)
            .await?;

        if versions.is_empty() {
            println!(
                "{}nothing found for {} ({})",
                "  ".repeat(ident),
                project.title,
                game_versions[0]
            );
            continue;
        }

        println!(
            "{}==={} ({})===",
            "  ".repeat(ident),
            project.title,
            game_versions[0]
        );

        let latest_version = versions
            .into_iter()
            .max_by_key(|v| v.version_number.clone())
            .unwrap();

        // todo: or_else(first_file)
        let primary_file = latest_version
            .files
            .iter()
            .find(|f| f.primary)
            .unwrap_or_else(|| latest_version.files.first().unwrap());
        println!(
            "{}{} : {}",
            "  ".repeat(ident + 1),
            latest_version.name,
            primary_file.filename
        );

        let jar = api.download_file(primary_file.url.clone()).await.unwrap();

        let mut dst = output_folder.to_path_buf();
        dst.push(&primary_file.filename);

        let builder =
            ZipEntryBuilder::new(primary_file.filename.clone().into(), Compression::Deflate);
        zip.write_entry_whole(builder, &jar).await.unwrap();

        downloaded.insert(project.id.clone());

        // do this before calling `get_project`
        // otherwise causes deadlock
        drop(global_projects);

        for dep in latest_version.dependencies {
            let project_id = dep.project_id.unwrap();

            if dep.dependency_type != DependencyType::Required {
                println!("{}- {} is not required", "  ".repeat(ident + 1), project_id);
                continue;
            }

            if downloaded.contains(&project_id) {
                println!(
                    "{}- {} already downloaded",
                    "  ".repeat(ident + 1),
                    project_id
                );
                continue;
            }

            let project = api.get_project(&project_id).await?;

            todo.push((project, ident + 1));
        }
    }

    println!("finished download!");

    zip.close().await.unwrap();

    tokio::task::spawn(async move {
        tokio::time::sleep(Duration::from_secs(2 * 60)).await;
        tokio::fs::remove_file(filename).await.unwrap()
    });

    Ok(format!("/temp-download-all/{}-{now}.zip", collection_name))
}
