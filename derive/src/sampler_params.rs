use syn::{Ident, parse::Parse};

use crate::DeriveResult;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplerBindingType {
    #[default]
    Filtering,
    NonFiltering,
    Comparison,
}

impl Parse for SamplerBindingType {
    fn parse(input: syn::parse::ParseStream) -> DeriveResult<Self> {
        let ident = input.parse::<Ident>()?;
        match ident.to_string().as_str() {
            "filtering" => Ok(Self::Filtering),
            "non_filtering" => Ok(Self::NonFiltering),
            "comparison" => Ok(Self::Comparison),
            _ => Err(syn::Error::new(
                ident.span(),
                "invalid sample type (availible types are: `filering`, `non_filtering`, `comparison`)",
            )),
        }
    }
}
