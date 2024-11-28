use leptos::{
    config::get_configuration,
    task::Executor as LeptosExecutor
};
use leptos_wasi::{
    handler::HandlerError,
    prelude::{IncomingRequest, ResponseOutparam, WasiExecutor},
};
use wasi::exports::http::incoming_handler::Guest;
use wasi::http::proxy::export;

use crate::app::{shell, App, SaveCount};

struct LeptosServer;

impl Guest for LeptosServer {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        // Initiate a single-threaded [`Future`] Executor so we can run the
        // rendering system and take advantage of bodies streaming.
        let executor = WasiExecutor::new(leptos_wasi::executor::Mode::Stalled);
        if let Err(e) = LeptosExecutor::init_local_custom_executor(executor.clone()) {
            eprintln!("Got error while initializing leptos_wasi executor: {e:?}");
            return;
        }
        executor.run_until(async {
            if let Err(e) = handle_request(request, response_out).await {
                eprintln!("Got error while handling request: {e:?}");
            }
        })
    }
}

async fn handle_request(
    request: IncomingRequest,
    response_out: ResponseOutparam,
) -> Result<(), HandlerError> {
    use leptos_wasi::prelude::Handler;

    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;

    Handler::build(request, response_out)?
        // NOTE: Add all server functions here to ensure functionality works as expected!
        .with_server_fn::<SaveCount>()
        // Fetch all available routes from your App.
        .generate_routes(App)
        // Actually process the request and write the response.
        .handle_with_context(move || shell(leptos_options.clone()), || {})
        .await?;
    Ok(())
}

export!(LeptosServer with_types_in wasi);
