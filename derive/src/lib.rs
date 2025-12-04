pub(crate) type DeriveResult<T> = Result<T, syn::Error>;

pub(crate) mod bind_group;
pub(crate) mod texture_view_params;
pub(crate) mod sampler_params;

#[proc_macro_derive(AsBindGroup, attributes(binding, shader_stages, uniform, texture_view, sampler))]
pub fn derive_as_bind_group_(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match bind_group::derive_as_bind_group(input.into()) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
