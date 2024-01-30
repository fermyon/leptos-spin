use bytes::Bytes;
use futures::{Stream, StreamExt};
use leptos::server_fn::{error::ServerFnError, request::Req};
use spin_sdk::http::{IncomingRequest, Headers};
use std::borrow::Cow;

/// This is here because the orphan rule does not allow us to implement it on IncomingRequest with
/// the generic error. So we have to wrap it to make it happy
pub struct SpinRequest{
    pub req: IncomingRequest,
    pub path_with_query: Option<String>,
    pub headers: Headers,
}
impl SpinRequest{
    pub fn new_from_req(req: IncomingRequest)-> Self{
        SpinRequest{
        path_with_query: req.path_with_query(),
        headers: req.headers(),
        req,
        }
    }
}

impl<CustErr> Req<CustErr> for SpinRequest
where
    CustErr: 'static,
{
    fn as_query(&self) -> Option<&str> {
        self.path_with_query
            .as_ref()
            .and_then(|n| n.split_once('?').map(|(_, query)| query))
    }

    fn to_content_type(&self) -> Option<Cow<'_, str>> {

        self.headers
            .get("Content-Type")
            .first()
            .map(|h| String::from_utf8_lossy(h))
            .map(Cow::into_owned)
            .map(Cow::<'static, str>::Owned)
    }

    fn accepts(&self) -> Option<Cow<'_, str>> {
        self.headers
            .get("Accept")
            .first()
            .map(|h| String::from_utf8_lossy(h))
            .map(Cow::into_owned)
            .map(Cow::<'static, str>::Owned)
    }

    fn referer(&self) -> Option<Cow<'_, str>> {
        self.headers
            .get("Referer")
            .first()
            .map(|h| String::from_utf8_lossy(h))
            .map(Cow::into_owned)
            .map(Cow::<'static, str>::Owned)
    }

    async fn try_into_bytes(self) -> Result<Bytes, ServerFnError<CustErr>> {
        let buf = self
            .req
            .into_body()
            .await
            .map_err(|e| ServerFnError::Deserialization(e.to_string()))?;
        Ok(Bytes::copy_from_slice(&buf))
    }

    async fn try_into_string(self) -> Result<String, ServerFnError<CustErr>> {
        let bytes = self.try_into_bytes().await?;
        String::from_utf8(bytes.to_vec()).map_err(|e| ServerFnError::Deserialization(e.to_string()))
    }

    fn try_into_stream(
        self,
    ) -> Result<
        impl Stream<Item = Result<Bytes, ServerFnError>> + Send + 'static,
        ServerFnError<CustErr>,
    > {
        Ok(self.req
            .into_body_stream()
            .map(|chunk| chunk.map(|c| Bytes::copy_from_slice(&c)).map_err(|e| ServerFnError::Deserialization(e.to_string()))))
    }
}
