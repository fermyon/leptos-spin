#![allow(dead_code)]

use futures::{SinkExt,Stream,StreamExt};
use leptos::{provide_context, use_context, LeptosOptions, RuntimeId};
use leptos_router::RouteListing;
use route_table::RouteMatch;
use spin_sdk::http::{Headers, IncomingRequest, OutgoingResponse, ResponseOutparam};
pub mod request;
pub mod request_parts;
pub mod response;
pub mod response_options;
pub mod route_table;
pub mod server_fn;

use crate::server_fn::handle_server_fns_with_context;
pub use request_parts::RequestParts;
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
    render_best_match_to_stream_with_context(req, resp_out, routes, app_fn, || {},  leptos_opts).await;
}
pub async fn render_best_match_to_stream_with_context<IV>(
    req: IncomingRequest,
    resp_out: ResponseOutparam,
    routes: &RouteTable,
    app_fn: impl Fn() -> IV + 'static + Clone,
    additional_context: impl Fn() + Clone + Send + 'static,
    leptos_opts: &LeptosOptions,
) where
    IV: leptos::IntoView + 'static,
{
    // req.uri() doesn't provide the full URI on Cloud (https://github.com/fermyon/spin/issues/2110). For now, use the header instead
    let url = url::Url::parse(&url(&req)).unwrap();
    let path = url.path();

    match routes.best_match(path) {
        RouteMatch::Route(best_listing) => {
            render_route_with_context(url, req, resp_out, app_fn, additional_context, leptos_opts, &best_listing).await
        }
        RouteMatch::ServerFn => handle_server_fns_with_context(req, resp_out, additional_context).await,
        RouteMatch::None => {
            eprintln!("No route found for {url}");
            not_found(resp_out).await
        }
    }
}

async fn render_route<IV>(
    url: url::Url,
    req: IncomingRequest,
    resp_out: ResponseOutparam,
    app_fn: impl Fn() -> IV + 'static + Clone,
    leptos_opts: &LeptosOptions,
    listing: &RouteListing,
) where
    IV: leptos::IntoView + 'static,
{
render_route_with_context(url, req, resp_out, app_fn, ||{}, leptos_opts, listing).await;
}

async fn render_route_with_context<IV>(
    url: url::Url,
    req: IncomingRequest,
    resp_out: ResponseOutparam,
    app_fn: impl Fn() -> IV + 'static + Clone,
    additional_context: impl Fn() + Clone + Send + 'static,
    leptos_opts: &LeptosOptions,
    listing: &RouteListing,
) where
    IV: leptos::IntoView + 'static,
{
    if listing.static_mode().is_some() {
        log_and_server_error("Static routes are not supported on Spin", resp_out);
        return;
    }

    match listing.mode() {
        leptos_router::SsrMode::OutOfOrder => {
            let resp_opts = ResponseOptions::default();
            let req_parts = RequestParts::new_from_req(&req);

            let app = {
                let app_fn2 = app_fn.clone();
                let res_options = resp_opts.clone();
                move || {
                    provide_contexts(&url, req_parts, res_options, additional_context);
                    (app_fn2)().into_view()
                }
            };
            render_view_into_response_stm(app, resp_opts, leptos_opts, resp_out).await;
        }
        leptos_router::SsrMode::Async => {
            let resp_opts = ResponseOptions::default();
            let req_parts = RequestParts::new_from_req(&req);

            let app = {
                let app_fn2 = app_fn.clone();
                let res_options = resp_opts.clone();
                move || {
                    provide_contexts(&url, req_parts, res_options, additional_context);
                    (app_fn2)().into_view()
                }
            };
            render_view_into_response_stm_async_mode(app, resp_opts, leptos_opts, resp_out).await;
        }
        leptos_router::SsrMode::InOrder => {
            let resp_opts = ResponseOptions::default();
            let req_parts = RequestParts::new_from_req(&req);

            let app = {
                let app_fn2 = app_fn.clone();
                let res_options = resp_opts.clone();
                move || {
                    provide_contexts(&url, req_parts, res_options, additional_context);
                    (app_fn2)().into_view()
                }
            };
            render_view_into_response_stm_in_order_mode(app, leptos_opts, resp_opts, resp_out)
                .await;
        }
        leptos_router::SsrMode::PartiallyBlocked => {
            let resp_opts = ResponseOptions::default();
            let req_parts = RequestParts::new_from_req(&req);

            let app = {
                let app_fn2 = app_fn.clone();
                let res_options = resp_opts.clone();
                move || {

                    provide_contexts(&url, req_parts, res_options, additional_context);
                    (app_fn2)().into_view()
                }
            };
            render_view_into_response_stm_partially_blocked_mode(
                app,
                leptos_opts,
                resp_opts,
                resp_out,
            )
            .await;
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
        || leptos_meta::generate_head_metadata_separated().1.into(),
        || {},
        false);
    build_stream_response(stm, leptos_opts, resp_opts, resp_out, runtime).await;
}

async fn render_view_into_response_stm_async_mode(
    app: impl FnOnce() -> leptos::View + 'static,
    resp_opts: ResponseOptions,
    leptos_opts: &leptos::leptos_config::LeptosOptions,
    resp_out: ResponseOutparam,
) {
    // In the Axum integration, all this happens in a separate task, and sends back
    // to the function via a futures::channel::oneshot(). WASI doesn't have an
    // equivalent for that yet, so for now, just truck along.
    let (stm, runtime) = leptos::ssr::render_to_stream_in_order_with_prefix_undisposed_with_context(
        app,
        move || "".into(),
        || {},
    );
    let html = leptos_integration_utils::build_async_response(stm, leptos_opts, runtime).await;

    let status_code = resp_opts.status().unwrap_or(200);
    let headers = resp_opts.headers();

    let og = OutgoingResponse::new(status_code, &headers);
    let mut ogbod = og.take_body();
    resp_out.set(og);
    ogbod.send(html.into_bytes()).await.unwrap();
}

async fn render_view_into_response_stm_in_order_mode(
    app: impl FnOnce() -> leptos::View + 'static,
    leptos_opts: &LeptosOptions,
    resp_opts: ResponseOptions,
    resp_out: ResponseOutparam,
) {
    let (stm, runtime) = leptos::ssr::render_to_stream_in_order_with_prefix_undisposed_with_context(
        app,
        || leptos_meta::generate_head_metadata_separated().1.into(),
        || {},
    );

    build_stream_response(stm, leptos_opts, resp_opts, resp_out, runtime).await;
}

async fn render_view_into_response_stm_partially_blocked_mode(
    app: impl FnOnce() -> leptos::View + 'static,
    leptos_opts: &LeptosOptions,
    resp_opts: ResponseOptions,
    resp_out: ResponseOutparam,
) {
    let (stm, runtime) =
        leptos::ssr::render_to_stream_with_prefix_undisposed_with_context_and_block_replacement(
            app,
            move || leptos_meta::generate_head_metadata_separated().1.into(),
            || (),
            true,
        );
    build_stream_response(stm, leptos_opts, resp_opts, resp_out, runtime).await;
}

async fn build_stream_response(
    stm: impl Stream<Item = String>,
    leptos_opts: &LeptosOptions,
    resp_opts: ResponseOptions,
    resp_out: ResponseOutparam,
    runtime: RuntimeId,
) {
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
    let headers = resp_opts.headers();

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

/// Provides an easy way to redirect the user from within a server function.
///
/// This sets the `Location` header to the URL given.
///
/// If the route or server function in which this is called is being accessed
/// by an ordinary `GET` request or an HTML `<form>` without any enhancement, it also sets a
/// status code of `302` for a temporary redirect. (This is determined by whether the `Accept`
/// header contains `text/html` as it does for an ordinary navigation.)
///
/// Otherwise, it sets a custom header that indicates to the client that it should redirect,
/// without actually setting the status code. This means that the client will not follow the
/// redirect, and can therefore return the value of the server function and then handle
/// the redirect with client-side routing.
pub fn redirect(path: &str) {
    if let (Some(req), Some(res)) = (
        use_context::<RequestParts>(),
        use_context::<ResponseOptions>(),
    ) {
        // insert the Location header in any case
        res.insert_header("Location", path);
        let headers = Headers::new(req.headers());
        let accepts_html = &headers.get("Accept")[0];
        let accepts_html_bool = { String::from_utf8_lossy(accepts_html) == "text/html" };

        if accepts_html_bool {
            // if the request accepts text/html, it's a plain form request and needs
            // to have the 302 code set
            res.set_status(302);
        } else {
            // otherwise, we sent it from the server fn client and actually don't want
            // to set a real redirect, as this will break the ability to return data
            // instead, set the REDIRECT_HEADER to indicate that the client should redirect
            res.insert_header("serverfnredirect", "");
        }
    } else {
        tracing::warn!(
            "Couldn't retrieve either Parts or ResponseOptions while trying \
             to redirect()."
        );
    }
    if let Some(response_options) = use_context::<ResponseOptions>() {
        response_options.set_status(302);
        response_options.insert_header("Location", path);
    }
}

fn provide_contexts(url: &url::Url, req_parts: RequestParts, res_options: ResponseOptions, additional_context: impl Fn() + Clone + Send + 'static) {
    let path = leptos_corrected_path(url);

    let integration = leptos_router::ServerIntegration { path };
    provide_context(leptos_router::RouterIntegrationContext::new(integration));
    provide_context(leptos_meta::MetaContext::new());
    provide_context(res_options);
    provide_context(req_parts);
    additional_context();
    leptos_router::provide_server_redirect(redirect);
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
    String::from_utf8_lossy(full_url).to_string()
}

fn log_and_server_error(message: impl Into<String>, resp_out: ResponseOutparam) {
    println!("Error: {}", message.into());
    let response = spin_sdk::http::OutgoingResponse::new(500, &spin_sdk::http::Fields::new(&[]));
    resp_out.set(response);
}
