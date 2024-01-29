use leptos_spin::{request::SpinRequest, response::SpinResponse};
use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;
use syn::__private::ToTokens;

#[proc_macro_attribute]
#[proc_macro_error]
pub fn server(args: proc_macro::TokenStream, s: TokenStream) -> TokenStream {
    match server_fn_macro::server_macro_impl(
        args.into(),
        s.into(),
        Some(syn::parse_quote!(::leptos::server_fn)),
        "/api",
        Some(syn::parse_quote!(SpinRequest)),
        Some(syn::parse_quote!(SpinResponse)),
    ) {
        Err(e) => e.to_compile_error().into(),
        Ok(s) => s.to_token_stream().into(),
    }
}
