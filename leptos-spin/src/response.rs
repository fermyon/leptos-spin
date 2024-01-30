use crate::ResponseOptions;
use bytes::Bytes;
use futures::{Stream, StreamExt};
use leptos::expect_context;
use leptos::server_fn::error::{
    ServerFnError, ServerFnErrorErr, ServerFnErrorSerde, SERVER_FN_ERROR_HEADER,
};
use leptos::server_fn::response::Res;
use spin_sdk::http::Headers;
use std::pin::Pin;
use std::{
    fmt::{Debug, Display},
    str::FromStr,
};
use typed_builder::TypedBuilder;
/// This is here because the orphan rule does not allow us to implement it on IncomingRequest with
/// the generic error. So we have to wrap it to make it happy
pub struct SpinResponse(pub SpinResponseParts);

#[derive(TypedBuilder)]
pub struct SpinResponseParts {
    pub status_code: u16,
    pub headers: Headers,
    pub body: SpinBody,
}

/// We either can return a fairly simple Box type for normal bodies or a Stream for Streaming
/// server functions
pub enum SpinBody {
    Plain(Vec<u8>),
    Streaming(Pin<Box<dyn Stream<Item = Result<Bytes, Box<dyn std::error::Error>>>>>),
}
impl<CustErr> Res<CustErr> for SpinResponse
where
    CustErr: Send + Sync + Debug + FromStr + Display + 'static,
{
    fn try_from_string(content_type: &str, data: String) -> Result<Self, ServerFnError<CustErr>> {
        let headers = Headers::new(&[("Content-Type".to_string(), content_type.into())]);
        let parts = SpinResponseParts::builder()
            .status_code(200)
            .headers(headers)
            .body(SpinBody::Plain(data.into()))
            .build();
        Ok(SpinResponse(parts))
    }

    fn try_from_bytes(content_type: &str, data: Bytes) -> Result<Self, ServerFnError<CustErr>> {
        let headers = Headers::new(&[("Content-Type".to_string(), content_type.into())]);
        let parts = SpinResponseParts::builder()
            .status_code(200)
            .headers(headers)
            .body(SpinBody::Plain(data.into()))
            .build();
        Ok(SpinResponse(parts))
    }

    fn try_from_stream(
        content_type: &str,
        data: impl Stream<Item = Result<Bytes, ServerFnError<CustErr>>> + Send + 'static,
    ) -> Result<Self, ServerFnError<CustErr>> {
        let body = data.map(|n| {
            n.map_err(ServerFnErrorErr::from)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        });

        let headers = Headers::new(&[("Content-Type".to_string(), content_type.into())]);
        let parts = SpinResponseParts::builder()
            .status_code(200)
            .headers(headers)
            .body(SpinBody::Streaming(Box::pin(body)))
            .build();
        Ok(SpinResponse(parts))
    }

    fn error_response(path: &str, err: &ServerFnError<CustErr>) -> Self {
        let headers = Headers::new(&[(SERVER_FN_ERROR_HEADER.to_string(), path.into())]);
        let parts = SpinResponseParts::builder()
            .status_code(500)
            .headers(headers)
            .body(SpinBody::Plain(
                err.ser().unwrap_or_else(|_| err.to_string()).into(),
            ))
            .build();
        SpinResponse(parts)
    }

    fn redirect(&mut self, path: &str) {
        let res_options = expect_context::<ResponseOptions>();
        res_options.insert_header("Location", path);
        res_options.set_status(302);
    }
}
