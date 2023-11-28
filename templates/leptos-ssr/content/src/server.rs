use leptos::ServerFn;
use spin_sdk::http::{ResponseOutparam, IncomingRequest};
use spin_sdk::http_component;
use leptos_spin::{render_best_match_to_stream, RouteTable};

#[http_component]
async fn handle_{{project-name | snake_case}}(req: IncomingRequest, resp_out: ResponseOutparam) {
    let mut conf = leptos::get_configuration(None).await.unwrap();
    conf.leptos_options.output_name = "{{project-name | snake_case}}".to_owned();

    crate::app::SaveCount::register_explicit().unwrap();

    let app_fn = crate::app::App;

    let mut routes = RouteTable::build(app_fn);
    routes.add_server_fn_prefix("/api").unwrap();

    render_best_match_to_stream(req, resp_out, &routes, app_fn, &conf.leptos_options).await
}
