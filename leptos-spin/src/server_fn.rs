use crate::request_parts::RequestParts;
use crate::response_options::ResponseOptions;
use crate::{
    request::SpinRequest,
    response::{SpinBody, SpinResponse},
};
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use http::Method as HttpMethod;
use leptos::prelude::provide_context;
use leptos::server_fn::middleware::Service;
/// Leptos Spin Integration for server functions
use leptos::server_fn::{codec::Encoding, initialize_server_fn_map, ServerFn, ServerFnTraitObj};
use multimap::MultiMap;
use once_cell::sync::Lazy;
use spin_sdk::http::{Headers, IncomingRequest, OutgoingResponse, ResponseOutparam};
use url::Url;

#[allow(unused)] // used by server integrations
type LazyServerFnMap<Req, Res> = Lazy<DashMap<(String, http::Method), ServerFnTraitObj<Req, Res>>>;

static REGISTERED_SERVER_FUNCTIONS: LazyServerFnMap<SpinRequest, SpinResponse> =
    initialize_server_fn_map!(SpinRequest, SpinResponse);

/// Explicitly register a server function. This is only necessary if you are
/// running the server in a WASM environment (or a rare environment that the
/// `inventory`) crate doesn't support. Spin is one of those environments
pub fn register_explicit<T>()
where
    T: ServerFn<ServerRequest = SpinRequest, ServerResponse = SpinResponse> + 'static,
{
    REGISTERED_SERVER_FUNCTIONS.insert(
        (T::PATH.to_owned(), T::InputEncoding::METHOD),
        ServerFnTraitObj::new(
            T::PATH,
            T::InputEncoding::METHOD,
            |req| Box::pin(T::run_on_server(req)),
            T::middlewares,
        ),
    );
}

/// The set of all registered server function paths.
pub fn server_fn_paths() -> impl Iterator<Item = (&'static str, HttpMethod)> {
    REGISTERED_SERVER_FUNCTIONS
        .iter()
        .map(|item| (item.path(), item.method()))
}

pub async fn handle_server_fns(req: IncomingRequest, resp_out: ResponseOutparam) {
    handle_server_fns_with_context(req, resp_out, || {}).await;
}
pub async fn handle_server_fns_with_context(
    req: IncomingRequest,
    resp_out: ResponseOutparam,
    additional_context: impl Fn() + 'static + Clone + Send,
) {
    let pq = req.path_with_query().unwrap_or_default();

    let (spin_res, req_parts, res_options) =
        match crate::server_fn::get_server_fn_by_path_and_method(&pq, req.method().into()) {
            Some(lepfn) => {
                // let runtime = create_runtime();
                let req_parts = RequestParts::new_from_req(&req);
                provide_context(req_parts.clone());
                let res_options = ResponseOptions::default_without_headers();
                provide_context(res_options.clone());
                additional_context();
                let spin_req = SpinRequest::new_from_req(req);
                (
                    lepfn.clone().run(spin_req).await,
                    req_parts,
                    res_options,
                    // runtime,
                )
            }
            None => panic!("Server FN path {} not found", &pq),
        };
    // If the Accept header contains text/html, than this is a request from
    // a regular html form, so we should setup a redirect to either the referrer
    // or the user specified location

    let req_headers = Headers::from_list(req_parts.headers())
        .expect("Failed to construct Headers from Request Input for a Server Fn.");
    let accepts_html = &req_headers.get(&"Accept".to_string())[0];
    let accepts_html_bool = String::from_utf8_lossy(accepts_html).contains("text/html");

    if accepts_html_bool {
        let referrer = &req_headers.get(&"Referer".to_string())[0];
        let location = &req_headers.get(&"Location".to_string());
        if location.is_empty() {
            res_options.insert_header("location", referrer.to_owned());
        }
        // Set status and header for redirect
        if !res_options.status_is_set() {
            res_options.set_status(302);
        }
    }

    let headers = merge_headers(spin_res.0.headers, res_options.headers());
    let status = res_options.status().unwrap_or(spin_res.0.status_code);
    match spin_res.0.body {
        SpinBody::Plain(r) => {
            let og = OutgoingResponse::new(headers);
            og.set_status_code(status).expect("Failed to set Status");
            let mut ogbod = og.take_body();
            resp_out.set(og);
            ogbod.send(r).await.unwrap();
        }
        SpinBody::Streaming(mut s) => {
            let og = OutgoingResponse::new(headers);
            og.set_status_code(status).expect("Failed to set Status");
            let mut res_body = og.take_body();
            resp_out.set(og);

            while let Some(Ok(chunk)) = s.next().await {
                let _ = res_body.send(chunk.to_vec()).await;
            }
        }
    }
    // runtime.dispose();
}

/// Returns the server function at the given path
pub fn get_server_fn_by_path_and_method(
    path: &str,
    method: http::Method,
) -> Option<ServerFnTraitObj<SpinRequest, SpinResponse>> {
    // Sanitize Url to prevent query string or ids causing issues. To do that Url wants a full url,
    // so we give it a fake one. We're only using the path anyway!
    let full_url = format!("http://leptos.dev{}", path);
    let Ok(url) = Url::parse(&full_url) else {
        println!("Failed to parse: {full_url:?}");
        return None;
    };
    REGISTERED_SERVER_FUNCTIONS
        .get_mut(&(url.path().to_owned(), method))
        .map(|f| f.clone())
}

/// Merge together two sets of headers, deleting any in the first set of Headers that have a key in
/// the second set of headers.
pub fn merge_headers(h1: Headers, h2: Headers) -> Headers {
    //1. Get all keys in H1 and H2
    let entries1 = h1.entries();
    let entries2 = h2.entries();

    let mut mmap1 = MultiMap::new();
    entries1.iter().for_each(|(k, v)| {
        mmap1.insert(k, v);
    });
    let mut mmap2 = MultiMap::new();
    entries2.iter().for_each(|(k, v)| {
        mmap2.insert(k, v);
    });

    //2. Delete any keys in H1 that are present in H2
    mmap1.retain(|&k, &_v| mmap2.get(k).is_none());

    //3. Iterate through H2, adding them to H1
    mmap1.extend(mmap2);

    //4. Profit
    let mut merged_vec: Vec<(String, Vec<u8>)> = vec![];
    mmap1.iter_all().for_each(|(k, v)| {
        for v in v.iter() {
            merged_vec.push((k.to_string(), v.to_vec()))
        }
    });
    Headers::from_list(&merged_vec).expect("Failed to create headers from merged list")
}
