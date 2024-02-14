use std::sync::{Arc, RwLock};

use spin_sdk::http::Headers;

#[derive(Clone, Debug, Default)]
pub struct ResponseOptions {
    inner: Arc<RwLock<ResponseOptionsInner>>,
}

impl ResponseOptions {
    pub fn status(&self) -> Option<u16> {
        self.inner.read().unwrap().status
    }
    pub fn set_status(&self, status: u16) {
        let mut inner = self.inner.write().unwrap();
        inner.status = Some(status);
    }

    pub fn headers(&self) -> Headers {
        self.inner.read().unwrap().headers.clone()
    }
    pub fn insert_header(&self, name: &str, value: impl Into<Vec<u8>>) {
        let inner = self.inner.write().unwrap();
        inner.headers.set(name, &[value.into()]);
    }
    pub fn append_header(&self, name: &str, value: &[u8]) {
        let inner = self.inner.write().unwrap();
        inner.headers.append(name, value);
    }
    // Creates a ResponseOptions object with a default 200 status and no headers
    // Useful for server functions
    pub fn default_without_headers() -> Self {
        Self{
            inner: Arc::new(RwLock::new(ResponseOptionsInner::default_without_headers())),
        }
    }
}

#[derive(Debug)]
struct ResponseOptionsInner {
    status: Option<u16>,
    headers: Headers,
}

impl Default for ResponseOptionsInner {
    fn default() -> Self {
        Self {
            status: Default::default(),
            headers: Headers::new(&[("content-type".to_owned(), "text/html".into())]),
        }
    }
}

impl ResponseOptionsInner{
    // Creates a ResponseOptionsInner object with a default 200 status and no headers
    // Useful for server functions
    pub fn default_without_headers() -> Self{
        Self {
            status: Default::default(),
            headers: Headers::new(&[]),
        }
    }
}
