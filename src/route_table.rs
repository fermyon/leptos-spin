use leptos_router::RouteListing;
use routefinder::Router;

pub struct RouteTable(Router<Option<RouteListing>>);

pub(crate) enum RouteMatch {
    Route(RouteListing),
    ServerFn, // TODO: consider including the path
    None,
}

impl RouteTable {
    pub fn build<IV>(app_fn: impl Fn() -> IV + 'static + Clone) -> RouteTable
    where
        IV: leptos::IntoView + 'static,
    {
        let routes = generate_route_list(app_fn);

        let mut rf = Router::new();
        for listing in routes {
            let path = listing.path().to_owned();
            rf.add(path, Some(listing)).unwrap();
        }

        RouteTable(rf)
    }

    pub fn add_server_fn_prefix(&mut self, prefix: &str) -> Result<(), String> {
        let wildcard = format!("{prefix}/*");
        self.0.add(wildcard, None)
    }

    pub(crate) fn best_match(&self, path: &str) -> RouteMatch {
        match self.0.best_match(path).as_ref() {
            Some(m) => match m.as_ref() {
                Some(listing) => RouteMatch::Route(listing.clone()),
                None => RouteMatch::ServerFn,
            },
            None => RouteMatch::None,
        }
    }
}

fn generate_route_list<IV>(app_fn: impl Fn() -> IV + 'static + Clone) -> Vec<RouteListing>
where
    IV: leptos::IntoView + 'static,
{
    let (routes, _static_data_map) = leptos_router::generate_route_list_inner(app_fn);

    let routes = routes
        .into_iter()
        .map(empty_to_slash)
        .map(leptos_wildcards_to_spin)
        .collect::<Vec<_>>();

    if routes.is_empty() {
        vec![RouteListing::new(
            "/",
            "",
            Default::default(),
            [leptos_router::Method::Get],
            None,
        )]
    } else {
        // TODO: the actix one has something about excluded routes
        routes
    }
}

fn empty_to_slash(listing: RouteListing) -> RouteListing {
    let path = listing.path();
    if path.is_empty() {
        return RouteListing::new(
            "/",
            listing.path(),
            listing.mode(),
            listing.methods(),
            listing.static_mode(),
        );
    }
    listing
}

fn leptos_wildcards_to_spin(listing: RouteListing) -> RouteListing {
    // TODO: wildcards, parameters, etc etc etc.
    let path = listing.path();
    let path2 = path.replace("*any", "*");
    RouteListing::new(
        path2,
        listing.path(),
        listing.mode(),
        listing.methods(),
        listing.static_mode(),
    )
}
