use bytes::Bytes;
use futures::{Stream, StreamExt};
use leptos::server_fn::error::{
    ServerFnError, ServerFnErrorErr, ServerFnErrorSerde, SERVER_FN_ERROR_HEADER,
};
use leptos::server_fn::response::Res;
use spin_sdk::http::{Headers, OutgoingResponse, Response};
use std::{
    fmt::{Debug, Display},
    str::FromStr,
};

/// This is here because the orphan rule does not allow us to implement it on IncomingRequest with
/// the generic error. So we have to wrap it to make it happy
pub struct SpinResponse(pub OutgoingResponse);

impl<CustErr> Res<CustErr> for SpinResponse
where
    CustErr: Send + Sync + Debug + FromStr + Display + 'static,
{
    fn try_from_string(content_type: &str, data: String) -> Result<Self, ServerFnError<CustErr>> {
        let mut builder = Response::builder();
        Ok(SpinResponse(
            builder
                .status(200)
                .header("Content-Type", content_type)
                .body(data)
                .build()
                .into(),
        ))
    }

    fn try_from_bytes(content_type: &str, data: Bytes) -> Result<Self, ServerFnError<CustErr>> {
        let mut builder = Response::builder();
        Ok(SpinResponse(
            builder
                .status(200)
                .header("Content-Type", content_type)
                .body(data)
                .build()
                .into(),
        ))
    }

    fn try_from_stream(
        content_type: &str,
        data: impl Stream<Item = Result<Bytes, ServerFnError<CustErr>>> + Send + 'static,
    ) -> Result<Self, ServerFnError<CustErr>> {
        let body = data.map(|n| n.map_err(ServerFnErrorErr::from));
        let mut headers = Headers::new(&[("Content-Type".to_string(), content_type.into())]);
        let og_res = OutgoingResponse::new(200, &headers);
        let og_bod = og_res.take_body();
        og_bod.send(body)?;
        Ok(SpinResponse(og_res))
    }

    fn error_response(path: &str, err: &ServerFnError<CustErr>) -> Self {
        SpinResponse(
            Response::builder()
                .status(500)
                .header(SERVER_FN_ERROR_HEADER, path)
                .body(err.ser().unwrap_or_else(|_| err.to_string()))
                .build()
                .into(),
        )
    }

    fn redirect(&mut self, path: &str) {
        self.0.set_header("Location", path);
        *self.status_mut() = 302;
    }
}
