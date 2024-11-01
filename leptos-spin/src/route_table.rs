use std::borrow::Cow;

use leptos_router::{PathSegment, RouteListing};
use routefinder::{Router, Segment};

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
            let path: Vec<Segment> = listing
                .path()
                .iter()
                .map(|segment| match segment {
                    // TODO: verify all these mappings
                    PathSegment::Unit => Segment::Exact("".to_owned().into()),
                    PathSegment::Static(cow) => Segment::Exact(cow.clone().into()),
                    PathSegment::Param(cow) => Segment::Param(cow.clone().into()),
                    PathSegment::OptionalParam(cow) => Segment::Param(cow.clone().into()),
                    PathSegment::Splat(cow) => Segment::Exact(cow.clone().into()),
                })
                .collect();
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
    let routes = match leptos_router::RouteList::generate(app_fn) {
        Some(route_list) => route_list.into_inner(),
        None => vec![],
    };

    let routes = routes
        .into_iter()
        .map(empty_to_slash)
        .map(leptos_wildcards_to_spin)
        .collect::<Vec<_>>();

    if routes.is_empty() {
        vec![RouteListing::new(
            vec![
                PathSegment::Static(std::borrow::Cow::Borrowed("/")),
                PathSegment::Unit,
            ],
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
            listing
                .path()
                .iter()
                .map(|segment| match segment {
                    PathSegment::Unit => PathSegment::Static("/".into()),
                    other => other.to_owned(),
                })
                .collect::<Vec<_>>(),
            listing.mode().to_owned(),
            listing.methods(),
            listing.regenerate().to_owned(),
        );
    }
    listing
}

fn leptos_wildcards_to_spin(listing: RouteListing) -> RouteListing {
    // TODO: wildcards, parameters, etc etc etc.
    RouteListing::new(
        listing
            .path()
            .iter()
            .map(|segment| match segment {
                PathSegment::Static(Cow::Borrowed("*any")) => PathSegment::Static("*".into()),
                other => other.to_owned(),
            })
            .collect::<Vec<_>>(),
        listing.mode().to_owned(),
        listing.methods(),
        listing.regenerate().to_owned(),
    )
}
