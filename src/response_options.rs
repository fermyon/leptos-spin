use std::sync::{Arc, RwLock};

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
}

#[derive(Debug, Default)]
struct ResponseOptionsInner {
    status: Option<u16>,
}
