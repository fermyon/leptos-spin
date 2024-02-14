use spin_sdk::http::{conversions::IntoHeaders, IncomingRequest, Method, Scheme};

// Because IncomingRequest is not Clone, we provide this struct with the
// easily cloneable parts.
// TODO: Evaluate whether Body can go here(perhaps as Bytes) without breaking Streaming
#[derive(Debug, Clone)]
pub struct RequestParts {
    method: Method,
    scheme: Option<Scheme>,
    headers: Vec<(String, Vec<u8>)>,
}
impl RequestParts {
    pub fn new_from_req(req: &IncomingRequest) -> Self {
        Self {
            method: req.method(),
            scheme: req.scheme(),
            headers: req.headers().into_headers(),
        }
    }
    /// Get the Headers for the Request
    pub fn headers(&self) -> &Vec<(String, Vec<u8>)> {
        &self.headers
    }
    /// Get the Method for the Request
    pub fn method(&self) -> &Method {
        &self.method
    }
    /// Get the Scheme for the Request
    pub fn scheme(&self) -> &Option<Scheme> {
        &self.scheme
    }
}
