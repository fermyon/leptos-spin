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
    /// Get the full URL for the Request
    pub fn url(&self) -> Option<&str> {
        let Some((_, full_url_header)) = self.headers.iter().find(|(name, _)| name == "spin-full-url") else {
            return None;
        };
        std::str::from_utf8(full_url_header).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn if_url_header_is_string_then_url_returns_it() {
        let parts = RequestParts {
            method: Method::Get,
            scheme: Some(Scheme::Https),
            headers: vec![
                ("spin-full-url".to_string(), "https://example.com/test?query".as_bytes().to_vec()),
            ],
        };
        assert_eq!("https://example.com/test?query", parts.url().expect("should have had a URL"));
    }

    #[test]
    fn if_url_header_is_missing_then_url_returns_none() {
        let parts = RequestParts {
            method: Method::Get,
            scheme: Some(Scheme::Https),
            headers: vec![],
        };
        assert_eq!(None, parts.url(), "should have NOT had a URL");
    }

    #[test]
    fn if_url_header_is_not_utf8_then_url_returns_none() {
        let parts = RequestParts {
            method: Method::Get,
            scheme: Some(Scheme::Https),
            headers: vec![
                ("spin-full-url".to_string(), vec![0xe2, 0x82, 0x28]),
            ],
        };
        assert_eq!(None, parts.url(), "should have NOT had a URL");
    }
}
