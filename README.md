# Spin/Leptos integration library

**THIS IS A WORK IN PROGRESS.** Actually 'in progress' probably oversells it. It is an early draft with a lot of learning as we go.

This library provides integration services for running [Leptos](https://leptos-rs.github.io/leptos/) server-side applications in Spin. Previously, in Leptos versions 0.6 and below, a server integration similar to Leptos' Actix and Axum integrations was designed, published, and maintained in this repository (through the excellent work of @itowlson from Fermyon and *@benwis* from Leptos). As of Leptos 0.7, this integration relies on the [leptos_wasi crate](https://github.com/leptos-rs/leptos_wasi) now maintained by Leptos to provide a server. As `leptos_wasi` creates a [wasi-http](https://github.com/WebAssembly/wasi-http) component, Spin can seamlessly run this generated server and uses the [spin-fileserver](https://github.com/fermyon/spin-fileserver) component to vend client assets. This library integrates Spin's concepts and Leptos', allowing utilization of the vast ecosystem of Spin + WASI functionality in a serverless Leptos back-end.

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

## Usage

```rust
async fn handle_request(request: IncomingRequest, response_out: ResponseOutparam) {
    use leptos_wasi::prelude::Handler;

    let mut conf = get_configuration(None).unwrap();
    conf.leptos_options.output_name = Arc::from("test_leptos_new".to_owned());
    let leptos_options = conf.leptos_options;

    Handler::build(request, response_out)
        .expect("could not create handler")
        // Register server functions here
        .with_server_fn::<SaveCount>()
        // Fetch all available routes from your App.
        .generate_routes(App)
        // Actually process the request and write the response.
        .handle_with_context(move || shell(leptos_options.clone()), || {})
        .await
        .expect("could not handle the request");
}
```
