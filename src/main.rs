#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use std::net::{SocketAddr, SocketAddrV4};
    use std::sync::Arc;

    use axum::Router;
    use leptos::leptos_config::Env;
    use leptos::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use mr_modpack::app::modrinth::*;
    use mr_modpack::app::*;
    use mr_modpack::fileserv::file_and_error_handler;

    // Setting get_configuration(None) means we'll be using cargo-leptos's env values
    // For deployment these variables are:
    // <https://github.com/leptos-rs/start-axum#executing-a-server-on-a-remote-machine-without-the-toolchain>
    // Alternately a file can be specified such as Some("Cargo.toml")
    // The file would need to be included with the executable when moved to deployment
    let conf = get_configuration(None).await.unwrap();
    let mut leptos_options = conf.leptos_options;
    leptos_options.hash_files = true;
    if leptos_options.env == Env::PROD {
        // in the dockerfile, hash.txt will actually be here and not "./hash.txt'
        leptos_options.hash_file = "/app/target/release/hash.txt".to_string();
    }

    let addr = match std::env::var("PORT") {
        Ok(port) => SocketAddr::V4(SocketAddrV4::new(
            "0.0.0.0".parse().unwrap(),
            port.parse().expect("`PORT` to be an u16"),
        )),
        _ => leptos_options.site_addr,
    };
    let cloned_leptos_options = leptos_options.clone();
    let routes = generate_route_list(App);

    let modrinth = Arc::new(ModrinthClient::default());

    // build our application with a route
    let app = Router::new()
        .leptos_routes_with_context(
            &leptos_options,
            routes,
            move || {
                provide_context(modrinth.clone());
                provide_context(cloned_leptos_options.clone());
            },
            App,
        )
        .fallback(file_and_error_handler)
        .with_state(leptos_options);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    logging::log!("listening on http://{}", &addr);
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {
    // no client-side main function
    // unless we want this to work with e.g., Trunk for a purely client-side app
    // see lib.rs for hydration function instead
}
