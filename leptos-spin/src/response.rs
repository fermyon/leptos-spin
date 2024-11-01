use bytes::Bytes;
use futures::{Stream, StreamExt};
use leptos::server_fn::error::{
    ServerFnError, ServerFnErrorErr, ServerFnErrorSerde, SERVER_FN_ERROR_HEADER,
};
use leptos::server_fn::response::Res;
use leptos_integration_utils::ExtendResponse;
use spin_sdk::http::conversions::{FromBody, IntoBody};
use spin_sdk::http::{Headers, IntoResponse};
use std::pin::Pin;
use std::{
    fmt::{Debug, Display},
    str::FromStr,
};
use typed_builder::TypedBuilder;

use crate::ResponseOptions;
/// This is here because the orphan rule does not allow us to implement it on IncomingRequest with
/// the generic error. So we have to wrap it to make it happy
pub struct SpinResponse(pub SpinResponseParts);

impl ExtendResponse for SpinResponse {
    type ResponseOptions = ResponseOptions;

    fn from_stream(stream: impl Stream<Item = String> + Send + 'static) -> Self {
        Self(
            SpinResponseParts::builder()
                .body(SpinBody::Streaming(Box::pin(stream.map(|chunk| {
                    Ok(Bytes::from_body(chunk.as_bytes().to_vec()))
                }))))
                .headers(Headers::new())
                .status_code(200)
                .build(),
        )
    }

    fn extend_response(&mut self, res_options: &Self::ResponseOptions) {
        if let Some(status) = res_options.status() {
            self.0.status_code = status;
        }

        for (name, value) in res_options.headers().entries() {
            // TODO: verify if append or replace
            self.0.headers.append(&name, &value);
        }
    }

    fn set_default_content_type(&mut self, content_type: &str) {
        let content_type_header = "Content-Type".to_string();
        if !self.0.headers.has(&content_type_header) {
            // Set the Content Type headers on all responses. This makes Firefox show the page source
            // without complaining
            // TODO: verify if append or replace
            self.0
                .headers
                .set(&content_type_header, &[content_type.as_bytes().to_vec()]);
        }
    }
}

impl IntoResponse for SpinResponse {
    fn into_response(self) -> spin_sdk::http::Response {
        let SpinResponseParts {
            status_code,
            headers,
            body,
        } = self.0;
        spin_sdk::http::Response::builder()
            .body(body)
            .headers(headers)
            .status(status_code)
            .build()
    }
}

impl IntoBody for SpinBody {
    fn into_body(self) -> Vec<u8> {
        match self {
            SpinBody::Plain(vec) => vec,
            SpinBody::Streaming(pin) => todo!("Need to figure this one out"),
        }
    }
}

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
    Streaming(Pin<Box<dyn Stream<Item = Result<Bytes, Box<dyn std::error::Error>>> + Send>>),
}
impl<CustErr> Res<CustErr> for SpinResponse
where
    CustErr: Send + Sync + Debug + FromStr + Display + 'static,
{
    fn try_from_string(content_type: &str, data: String) -> Result<Self, ServerFnError<CustErr>> {
        let headers =
            Headers::from_list(&[("Content-Type".to_string(), content_type.as_bytes().to_vec())])
                .expect("Failed to create Headers from String Response Input");
        let parts = SpinResponseParts::builder()
            .status_code(200)
            .headers(headers)
            .body(SpinBody::Plain(data.into()))
            .build();
        Ok(SpinResponse(parts))
    }

    fn try_from_bytes(content_type: &str, data: Bytes) -> Result<Self, ServerFnError<CustErr>> {
        let headers = Headers::from_list(&[("Content-Type".to_string(), content_type.into())])
            .expect("Failed to create Headers from Bytes Response Input");
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

        let headers = Headers::from_list(&[("Content-Type".to_string(), content_type.into())])
            .expect("Failed to create Headers from Stream Response Input");
        let parts = SpinResponseParts::builder()
            .status_code(200)
            .headers(headers)
            .body(SpinBody::Streaming(Box::pin(body)))
            .build();
        Ok(SpinResponse(parts))
    }

    fn error_response(path: &str, err: &ServerFnError<CustErr>) -> Self {
        let headers = Headers::from_list(&[(SERVER_FN_ERROR_HEADER.to_string(), path.into())])
            .expect("Failed to create Error Response. This should be impossible");
        let parts = SpinResponseParts::builder()
            .status_code(500)
            .headers(headers)
            .body(SpinBody::Plain(
                err.ser().unwrap_or_else(|_| err.to_string()).into(),
            ))
            .build();
        SpinResponse(parts)
    }

    fn redirect(&mut self, _path: &str) {
        //TODO: Enabling these seems to override location header
        // not sure what's causing that
        //let res_options = expect_context::<ResponseOptions>();
        //res_options.insert_header("Location", path);
        //res_options.set_status(302);
    }
}
