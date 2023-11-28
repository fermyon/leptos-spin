# Spin/Leptos integration library

**THIS IS A WORK IN PROGRESS.** Actually 'in progress' probably oversells it. It is an early draft with a lot of learning as we go.

This library provides integration services for running [Leptos](https://leptos-rs.github.io/leptos/) server-side applications in Spin.  It plays a role similar to Leptos' Actix and Axum integrations, bridging Spin's implementation of concepts and Leptos', and abstracting away common functionality to routing requests to Leptos views and server functions.

At the moment, this library is _entirely_ experimental. It has known gaps, names and APIs will change, and Leptos experts will no doubt have much to say about its design!

There is no Leptos-on-Spin template yet: you'll need to copy and paste off a sample.

## Special requirements

* All server functions (`#[server]`) **must** be explicitly registered (see usage sample below). In native code, Leptos uses a clever macro to register them automatically; unfortunately, that doesn't work in WASI.
* Event handlers in views **must** be wrapped in `leptos::request_animation_frame` ([more info](https://leptos-rs.github.io/leptos/ssr/24_hydration_bugs.html#mismatches-between-server-and-client-code)). I am not sure if this is fundamental or if the requirement can be removed as we improve the integration.
* Resources currently do not work as an upstream fix is needed (and is in progress). Similar issues may affect other server code - we have not tested very exhaustively yet!

## Usage

```rust
use leptos::ServerFn;
use leptos_spin::{render_best_match_to_stream, RouteTable};
use spin_sdk::http::{ResponseOutparam, IncomingRequest};
use spin_sdk::http_component;

#[http_component]
async fn handle_request(req: IncomingRequest, resp_out: ResponseOutparam) {
    let mut conf = leptos::get_configuration(None).await.unwrap();
    conf.leptos_options.output_name = "sample".to_owned();

    // A line like this for every server function
    crate::app::SaveCount::register_explicit().unwrap();

    let app_fn = crate::app::App;

    let mut routes = RouteTable::build(app_fn);
    routes.add_server_fn_prefix("/api").unwrap();

    render_best_match_to_stream(req, resp_out, &routes, app_fn, &conf.leptos_options).await
}
```
