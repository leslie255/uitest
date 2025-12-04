use syn::{Ident, LitBool, LitInt, Token, parse::Parse};

use crate::DeriveResult;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextureViewSampleType {
    #[default]
    Float,
    FloatUnfilterable,
    Depth,
    Sint,
    Uint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TextureViewBindingParams {
    pub(crate) sample_type: TextureViewSampleType,
    pub(crate) view_dimension: u32,
    pub(crate) multisampled: bool,
}

impl Default for TextureViewBindingParams {
    fn default() -> Self {
        Self {
            sample_type: Default::default(),
            view_dimension: 2,
            multisampled: Default::default(),
        }
    }
}

impl Parse for TextureViewBindingParams {
    fn parse(input: syn::parse::ParseStream) -> DeriveResult<Self> {
        let mut sample_type = None;
        let mut view_dimension = None;
        let mut multisampled = None;

        while !input.is_empty() {
            let key = input.parse::<Ident>()?;
            input.parse::<Token![=]>()?;
            match key.to_string().as_str() {
                "sample_type" => {
                    let value: Ident = input.parse()?;
                    let parsed = match &*value.to_string() {
                        "float" => TextureViewSampleType::Float,
                        "float_unfilterable" => TextureViewSampleType::FloatUnfilterable,
                        "depth" => TextureViewSampleType::Depth,
                        "sint" => TextureViewSampleType::Sint,
                        "uint" => TextureViewSampleType::Uint,
                        _ => {
                            return Err(syn::Error::new(
                                value.span(),
                                "invalid sample type (availible types are: `float`, `float_unfilterable`, `depth`, `sint`, `uint`)",
                            ));
                        }
                    };
                    sample_type = Some(parsed);
                }
                "view_dimension" => {
                    let lit = input.parse::<LitInt>()?;
                    let value = match lit.base10_parse()? {
                        value @ 1..=3 => value,
                        _ => {
                            return Err(syn::Error::new(
                                lit.span(),
                                "view_dimension can only be 1, 2 or 3",
                            ));
                        }
                    };
                    view_dimension = Some(value);
                }
                "multisampled" => {
                    multisampled = Some(input.parse::<LitBool>()?.value);
                }
                _ => {
                    return Err(syn::Error::new(
                        key.span(),
                        "unknown field (availible fields are: `sample_type`, `view_dimension`, `multisampled`)",
                    ));
                }
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(Self {
            sample_type: sample_type.unwrap_or_default(),
            view_dimension: view_dimension.unwrap_or(2),
            multisampled: multisampled.unwrap_or_default(),
        })
    }
}
