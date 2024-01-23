# Spin/Leptos integration library

**THIS IS A WORK IN PROGRESS.** Actually 'in progress' probably oversells it. It is an early draft with a lot of learning as we go.

This library provides integration services for running [Leptos](https://leptos-rs.github.io/leptos/) server-side applications in Spin.  It plays a role similar to Leptos' Actix and Axum integrations, bridging Spin's implementation of concepts and Leptos', and abstracting away common functionality to routing requests to Leptos views and server functions.

At the moment, this library is _entirely_ experimental. It has known gaps, names and APIs will change, and Leptos experts will no doubt have much to say about its design!

## Installing and running the template

The `leptos-ssr` template can be installed using the following command:

```bash
spin templates install --git https://github.com/fermyon/leptos-spin

Copying remote template source
Installing template leptos-ssr...
Installed 1 template(s)

+-------------------------------------------------------------+
| Name         Description                                    |
+=============================================================+
| leptos-ssr   Leptos application using server-side rendering |
+-------------------------------------------------------------+
```

Once the template is installed, a mew leptos project can be instantiated using: 

```bash
spin new -t leptos-ssr my-leptos-app -a
```
Before building and running the project [`cargo-leptos`](https://leptos-rs.github.io/leptos/ssr/21_cargo_leptos.html) needs to be installed:

```bash
cargo install cargo-leptos
```

To build and run the created project, the following command can be used:

```bash
cd my-leptos-app
spin build --up
```

Now the app should be served at `http://127.0.0.1:3000`

## Special requirements

* All server functions (`#[server]`) **must** be explicitly registered (see usage sample below). In native code, Leptos uses a clever macro to register them automatically; unfortunately, that doesn't work in WASI.

* When using a context value in a component in a `feature = "ssr"` block, you **must** call `use_context` **not** `expect_context`. `expect_context` will panic during routing.  E.g.

```rust
#[component]
fn HomePage() -> impl IntoView {
    #[cfg(feature = "ssr")]
    {
        if let Some(resp) = use_context::<leptos_spin::ResponseOptions>() {
            resp.append_header("X-Utensil", "spork".as_bytes());
        };
    }

    view! {
        <h1>"Come over to the Leptos side - we have headers!"</h1>
    }
}
```

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
