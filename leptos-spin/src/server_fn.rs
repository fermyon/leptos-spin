use crate::request_parts::RequestParts;
use crate::response_options::ResponseOptions;
use crate::url;
use crate::{incoming_request::SpinRequest, response::SpinResponse};
use dashmap::DashMap;
use futures::SinkExt;
use http::Method as HttpMethod;
/// Leptos Spin Integration for server functions
use leptos::server_fn::{codec::Encoding, initialize_server_fn_map, ServerFn, ServerFnTraitObj};
use leptos::{create_runtime, provide_context};
use once_cell::sync::Lazy;
use spin_sdk::http::{IncomingRequest, OutgoingResponse, Response, ResponseOutparam};

#[allow(unused)] // used by server integrations
type LazyServerFnMap<Req, Res> = Lazy<DashMap<&'static str, ServerFnTraitObj<Req, Res>>>;

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
        T::PATH,
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
    let pq = req.path_with_query().unwrap_or_default();
    // req.uri() doesn't provide the full URI on Cloud (https://github.com/fermyon/spin/issues/2110). For now, use the header instead
    let url = url::Url::parse(&url(&req)).unwrap();
    let mut path_segs = url.path_segments().unwrap().collect::<Vec<_>>();

    let (fn_res, res_parts, runtime) = loop {
        if path_segs.is_empty() {
            panic!("NO LEPTOS FN!  Ran out of path segs looking for a match");
        }

        let candidate = path_segs.join("/");
        if let Some(lepfn) = crate::server_fn::get_server_fn_by_path(&candidate) {
            // TODO: better checking here - again using the captures might help
            println!("PQ: {pq}");
            if pq.starts_with(lepfn.path()) {
                // Need to create a Runtime and provide some expected values
                let runtime = create_runtime();
                let req_parts = RequestParts::new_from_req(&req);
                provide_context(req_parts);
                let res_parts = ResponseOptions::default_without_headers();
                provide_context(res_parts.clone());

                break (lepfn.run(&req).await.unwrap(), res_parts, runtime);
            }
        }

        path_segs.remove(0);
    };

    let status = res_parts.status().unwrap_or(200);
    let headers = res_parts.headers();
    let og = OutgoingResponse::new(status, &headers);
    runtime.dispose();
    let mut ogbod = og.take_body();
    resp_out.set(og);
    ogbod.send(plbytes).await.unwrap();
}

/// Returns the server function at the given path
pub fn get_server_fn_by_path(path: &str) -> Option<&ServerFnTraitObj<SpinRequest, SpinResponse>> {
    REGISTERED_SERVER_FUNCTIONS.get(path).as_deref()
}
