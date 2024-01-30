use bytes::Bytes;
use futures::{Stream, StreamExt};
use leptos::server_fn::{error::ServerFnError, request::Req};
use spin_sdk::http::IncomingRequest;
use std::borrow::Cow;

/// This is here because the orphan rule does not allow us to implement it on IncomingRequest with
/// the generic error. So we have to wrap it to make it happy
pub struct SpinRequest{
    pub req: IncomingRequest,
    pub path_with_query: Option<String>
}
impl SpinRequest{
    pub fn new_from_req(req: IncomingRequest)-> Self{
        SpinRequest{
        path_with_query: req.path_with_query(),
        req,
        }
    }
}
//It looks like it's difficult to impl Req/Res for an external type due to Rust's orphan rules:
/*
error[E0210]: type parameter `CustErr` must be used as the type parameter for some local type (e.g., `MyStruct<CustErr>`)
  --> leptos-spin/src/response.rs:13:6
   |
13 | impl<CustErr> Res<CustErr> for Response
   |      ^^^^^^^ type parameter `CustErr` must be used as the type parameter for some local type
   |
   = note: implementing a foreign trait is only possible if at least one of the types for which it is implemented is local
   = note: only traits defined in the current crate can be implemented for a type parameter
*/
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

        self.req
            .headers()
            .get("Content-Type")
            .first()
            .map(|h| String::from_utf8_lossy(h))
            .map(Cow::into_owned)
            .map(Cow::<'static, str>::Owned)
    }

    fn accepts(&self) -> Option<Cow<'_, str>> {
        self.req
            .headers()
            .get("Accept")
            .first()
            .map(|h| String::from_utf8_lossy(h))
            .map(Cow::into_owned)
            .map(Cow::<'static, str>::Owned)
    }

    fn referer(&self) -> Option<Cow<'_, str>> {
        self.req
            .headers()
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
