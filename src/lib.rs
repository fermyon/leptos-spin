use futures::SinkExt;
use futures::StreamExt;
use leptos::LeptosOptions;
use leptos_router::RouteListing;
use route_table::RouteMatch;
use spin_sdk::http::{Headers, IncomingRequest, OutgoingResponse, ResponseOutparam};

mod response_options;
mod route_table;

pub use response_options::ResponseOptions;
pub use route_table::RouteTable;

pub async fn render_best_match_to_stream<IV>(
    req: IncomingRequest,
    resp_out: ResponseOutparam,
    routes: &RouteTable,
    app_fn: impl Fn() -> IV + 'static + Clone,
    leptos_opts: &LeptosOptions,
) where
    IV: leptos::IntoView + 'static,
{
    // req.uri() doesn't provide the full URI on Cloud (https://github.com/fermyon/spin/issues/2110). For now, use the header instead
    let url = url::Url::parse(&url(&req)).unwrap(); 
    let path = url.path();

    // TODO: do we need to provide fallback to next best match if method does not match?  Probably
    // TODO: ensure req.method() is acceptable
    match routes.best_match(path) {
        RouteMatch::Route(best_listing) => {
            render_route(url, resp_out, app_fn, leptos_opts, &best_listing).await
        }
        RouteMatch::ServerFn => handle_server_fns(req, resp_out).await,
        RouteMatch::None => {
            eprintln!("No route found for {url}");
            not_found(resp_out).await
        }
    }
}

async fn render_route<IV>(
    url: url::Url,
    resp_out: ResponseOutparam,
    app_fn: impl Fn() -> IV + 'static + Clone,
    leptos_opts: &LeptosOptions,
    listing: &RouteListing,
) where
    IV: leptos::IntoView + 'static,
{
    if let Some(_static_mode) = listing.static_mode() {
        panic!("Static mode is not yet supported");
    } else {
        match listing.mode() {
            leptos_router::SsrMode::OutOfOrder => {
                let resp_opts = ResponseOptions::default();
                let app = {
                    let app_fn2 = app_fn.clone();
                    let res_options = resp_opts.clone();
                    move || {
                        provide_contexts(&url, res_options);
                        (app_fn2)().into_view()
                    }
                };
                render_view_into_response_stm(app, resp_opts, leptos_opts, resp_out).await;
            }
            mode => panic!("Mode {mode:?} is not yet supported"),
        }
    }
}

// This is a backstop - the app should normally include a "/*"" route
// mapping to a NotFound Leptos component.
async fn not_found(resp_out: ResponseOutparam) {
    let og = OutgoingResponse::new(404, &Headers::new(&[]));
    resp_out.set(og);
}

async fn render_view_into_response_stm(
    app: impl FnOnce() -> leptos::View + 'static,
    resp_opts: ResponseOptions,
    leptos_opts: &leptos::leptos_config::LeptosOptions,
    resp_out: ResponseOutparam,
) {
    let (stm, runtime) = leptos::leptos_dom::ssr::render_to_stream_with_prefix_undisposed_with_context_and_block_replacement(
        app,
        move || {
            let (_h, b) = leptos_meta::generate_head_metadata_separated();
            b.into()
        },
        || {},
        false);
    let mut stm2 = Box::pin(stm);

    let first_app_chunk = stm2.next().await.unwrap_or_default();
    let (head, tail) = leptos_integration_utils::html_parts_separated(
        leptos_opts,
        leptos::use_context::<leptos_meta::MetaContext>().as_ref(),
    );

    let mut stm3 = Box::pin(
        futures::stream::once(async move { head.clone() })
            .chain(futures::stream::once(async move { first_app_chunk }).chain(stm2))
            .map(|html| html.into_bytes()),
    );

    let first_chunk = stm3.next().await;
    let second_chunk = stm3.next().await;

    let status_code = resp_opts.status().unwrap_or(200);
    // TODO: and headers
    let headers = Headers::new(&[("content-type".to_owned(), "text/html".into())]);

    let og = OutgoingResponse::new(status_code, &headers);
    let mut ogbod = og.take_body();
    resp_out.set(og);

    let mut stm4 = Box::pin(
        futures::stream::iter([first_chunk.unwrap(), second_chunk.unwrap()])
            .chain(stm3)
            .chain(
                futures::stream::once(async move {
                    runtime.dispose();
                    tail.to_string()
                })
                .map(|html| html.into_bytes()),
            ),
    );

    while let Some(ch) = stm4.next().await {
        ogbod.send(ch).await.unwrap();
    }
}

async fn handle_server_fns(req: IncomingRequest, resp_out: ResponseOutparam) {
    let pq = req.path_with_query().unwrap_or_default();
    let url = url::Url::parse(&req.uri()).unwrap();
    let mut path_segs = url.path_segments().unwrap().collect::<Vec<_>>();

    let payload = loop {
        if path_segs.is_empty() {
            panic!("NO LEPTOS FN!  Ran out of path segs looking for a match");
        }

        let candidate = path_segs.join("/");

        if let Some(lepfn) = leptos::leptos_server::server_fn_by_path(&candidate) {
            // TODO: better checking here - again using the captures might help
            if pq.starts_with(lepfn.prefix()) {
                let bod = req.into_body().await.unwrap();
                break lepfn.call((), &bod).await.unwrap();
            }
        }

        path_segs.remove(0);
    };

    let plbytes = match payload {
        leptos::server_fn::Payload::Binary(b) => b,
        leptos::server_fn::Payload::Json(s) => s.into_bytes(),
        leptos::server_fn::Payload::Url(u) => u.into_bytes(),
    };

    let og = OutgoingResponse::new(200, &Headers::new(&[]));
    let mut ogbod = og.take_body();
    resp_out.set(og);
    ogbod.send(plbytes).await.unwrap();
}

fn provide_contexts(url: &url::Url, res_options: ResponseOptions) {
    use leptos::provide_context;

    let path = leptos_corrected_path(url);

    let integration = leptos_router::ServerIntegration { path };
    provide_context(leptos_router::RouterIntegrationContext::new(integration));
    provide_context(leptos_meta::MetaContext::new());
    provide_context(res_options);
    // provide_context(req.clone());  // TODO: this feels like it could be needed?
    // leptos_router::provide_server_redirect(redirect);  // TODO: do we want this?
    #[cfg(feature = "nonce")]
    leptos::nonce::provide_nonce();
}

fn leptos_corrected_path(req: &url::Url) -> String {
    let path = req.path();
    let query = req.query();
    if query.unwrap_or_default().is_empty() {
        "http://leptos".to_string() + path
    } else {
        "http://leptos".to_string() + path + "?" + query.unwrap_or_default()
    }
}

fn url(req: &IncomingRequest) -> String {
    let full_url = &req.headers().get("spin-full-url")[0];
    String::from_utf8_lossy(&full_url).to_string()
}
