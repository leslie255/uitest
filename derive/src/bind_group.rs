use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{Attribute, Field, Ident, ItemStruct, LitInt, Meta, parse2, spanned::Spanned};

use crate::{
    DeriveResult,
    sampler_params::SamplerBindingType,
    texture_view_params::{TextureViewBindingParams, TextureViewSampleType},
};

macro_rules! error_spanned {
    ($span:expr => $message:expr $(,)?) => {
        syn::Error::new($span, $message)
    };
}

macro_rules! error {
    ($message:expr $(,)?) => {
        syn::Error::new(Span::call_site(), $message)
    };
}

pub(crate) fn derive_as_bind_group(input: TokenStream) -> DeriveResult<TokenStream> {
    let item_struct = parse2::<ItemStruct>(input)
        .map_err(|_| error!("`#[derive(Family)]`: `syn` failed to parse this item"))?;
    let struct_name = item_struct.ident;
    let fields = match item_struct.fields {
        syn::Fields::Named(named_fields) => named_fields.named,
        syn::Fields::Unnamed(_) => {
            return Err(error!(
                "`#[derive(Family)]` does not support unnamed fields yet"
            ));
        }
        syn::Fields::Unit => {
            return Err(error!(
                "`#[derive(Family)]` does not support unit struct yet"
            ));
        }
    };
    let mut layout_entries: Vec<TokenStream> = Vec::with_capacity(fields.len());
    let mut entries: Vec<TokenStream> = Vec::with_capacity(fields.len());
    for field in &fields {
        let binding_attrs = parse_binding_attributes(&field.attrs)?;
        let field_span = field
            .ident
            .span()
            .join(field.ty.span())
            .unwrap_or(field.span());
        let valid_binding_attrs = match validate_binding_attributes(binding_attrs, field_span)? {
            Some(valid_binding_attrs) => valid_binding_attrs,
            None => continue,
        };
        layout_entries.push(layout_entry(valid_binding_attrs, field)?);
        entries.push(entry(valid_binding_attrs, field)?);
    }
    Ok(quote! {
        impl crate::wgpu_utils::AsBindGroup for #struct_name {
            fn bind_group_layout_entries() -> ::std::vec::Vec<::wgpu::BindGroupLayoutEntry> {
                ::std::vec::Vec::from_iter([ #( #layout_entries ),* ])
            }

            fn bind_group_entries(&self) -> ::std::vec::Vec<::wgpu::BindGroupEntry<'_>> {
                ::std::vec::Vec::from_iter([ #( #entries ),* ])
            }

            fn create_bind_group_layout(device: &::wgpu::Device) -> wgpu::BindGroupLayout {
                device.create_bind_group_layout(&::wgpu::BindGroupLayoutDescriptor { 
                    label: ::std::option::Option::Some(::std::any::type_name::<Self>()),
                    entries: &[ #( #layout_entries ),* ],
                })
            }

            fn create_bind_group(
                &self,
                layout: &::wgpu::BindGroupLayout,
                device: &::wgpu::Device,
            ) -> wgpu::BindGroup {
                device.create_bind_group(&::wgpu::BindGroupDescriptor {
                    label: ::std::option::Option::Some(::std::any::type_name::<Self>()),
                    layout,
                    entries: &[ #( #entries ),* ],
                })
            }
        }
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BindingType {
    /// #[uniform]
    Uniform,
    /// #[texture_view]
    TextureView(TextureViewBindingParams),
    /// #[sampler]
    Sampler(SamplerBindingType),
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
struct BindingAttributes {
    location: Option<u32>,
    type_: Option<BindingType>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ValidBindingAttributes {
    location: u32,
    type_: BindingType,
}

fn validate_binding_attributes(
    binding_attrs: BindingAttributes,
    field_span: Span,
) -> DeriveResult<Option<ValidBindingAttributes>> {
    match (binding_attrs.location, binding_attrs.type_) {
        (None, None) => Ok(None),
        (Some(location), Some(type_)) => Ok(Some(ValidBindingAttributes { location, type_ })),
        (None, Some(_)) => {
            Err(error_spanned!(field_span => "missing binding location (e.g. `#[binding(0)]`)"))
        }
        (Some(_), None) => {
            Err(error_spanned!(field_span => "missing binding type (e.g. `#[uniform]`)"))
        }
    }
}

fn parse_binding_attributes<'a>(
    attrs: impl IntoIterator<Item = &'a Attribute>,
) -> DeriveResult<BindingAttributes> {
    let mut attrs_iter = attrs.into_iter();
    let mut result: BindingAttributes = BindingAttributes::default();
    for attr in attrs_iter.by_ref() {
        parse_binding_attribute(&mut result, attr)?;
    }
    Ok(result)
}

fn parse_binding_attribute(result: &mut BindingAttributes, attr: &Attribute) -> DeriveResult<()> {
    let attr_span = attr.span();
    match &attr.meta {
        Meta::Path(path) if path.get_ident().is_some() => {
            let ident = path.get_ident().unwrap();
            let type_ = match ident.to_string().as_ref() {
                "uniform" => BindingType::Uniform,
                "texture_view" => BindingType::TextureView(TextureViewBindingParams::default()),
                "sampler" => BindingType::Sampler(SamplerBindingType::default()),
                _ => return Ok(()),
            };
            if result.type_.is_some() {
                return Err(
                    error_spanned!(attr_span => "multiple attributes for binding type is not allowed"),
                );
            }
            result.type_ = Some(type_);
        }
        Meta::List(metalist) if metalist.path.get_ident().is_some() => {
            let ident = metalist.path.get_ident().unwrap();
            let ident_str = ident.to_string();
            match ident_str.as_ref() {
                "binding" => parse_location(result, metalist.tokens.clone(), attr_span)?,
                "shader_stages" => parse_shader_stages(result, metalist.tokens.clone(), attr_span)?,
                "uniform" | "texture_view" | "sampler" => {
                    let type_ = match ident_str.as_ref() {
                        "uniform" => {
                            return Err(
                                error_spanned!(attr_span => "uniform binding does not accept any parameters yet"),
                            );
                        }
                        "texture_view" => {
                            BindingType::TextureView(parse2(metalist.tokens.clone())?)
                        }
                        "sampler" => BindingType::Sampler(parse2(metalist.tokens.clone())?),
                        _ => unreachable!(),
                    };
                    if result.type_.is_some() {
                        return Err(
                            error_spanned!(attr_span => "multiple attributes for binding type is not allowed"),
                        );
                    }
                    result.type_ = Some(type_);
                }
                _ => return Ok(()),
            }
        }
        _ => (),
    }
    Ok(())
}

fn parse_location(
    result: &mut BindingAttributes,
    tokens: TokenStream,
    attr_span: Span,
) -> DeriveResult<()> {
    let span = tokens.span();
    let int_literal = parse2::<LitInt>(tokens.clone())
        .map_err(|_| error_spanned!(span => "expect binding location"))?;
    let location = int_literal.base10_parse::<u32>()?;
    if result.location.is_some() {
        return Err(
            error_spanned!(attr_span => "multiple attributes for binding location is not allowed"),
        );
    }
    result.location = Some(location);
    Ok(())
}

fn parse_shader_stages(
    _result: &mut BindingAttributes,
    _tokens: TokenStream,
    _attr_span: Span,
) -> DeriveResult<()> {
    todo!("parse shader stages")
}

fn layout_entry(binding_attrs: ValidBindingAttributes, field: &Field) -> DeriveResult<TokenStream> {
    let location = binding_attrs.location;
    let ty = match binding_attrs.type_ {
        BindingType::Uniform => uniform_buffer_layout_ty(field.span()),
        BindingType::TextureView(params) => texture_view_layout_ty(field.span(), params),
        BindingType::Sampler(params) => sampler_layout_ty(field.span(), params),
    };
    Ok(quote_spanned! {field.span()=>
        wgpu::BindGroupLayoutEntry {
            binding: #location,
            visibility: ::wgpu::ShaderStages::all(),
            ty: #ty,
            count: None,
        }
    })
}

fn entry(binding_attrs: ValidBindingAttributes, field: &Field) -> DeriveResult<TokenStream> {
    let location = binding_attrs.location;
    let field_ident = field
        .ident
        .as_ref()
        .unwrap_or_else(|| panic!("[{}@{}] TODO: tuple structs", std::line!(), std::file!()));
    let resource = match binding_attrs.type_ {
        BindingType::Uniform => uniform_binding_resource(field.span(), field_ident),
        BindingType::TextureView(..) => texture_view_binding_resource(field.span(), field_ident),
        BindingType::Sampler(..) => sampler_binding_resource(field.span(), field_ident),
    };
    Ok(quote_spanned! {field.span()=>
        wgpu::BindGroupEntry {
            binding: #location,
            resource: #resource,
        }
    })
}

fn uniform_buffer_layout_ty(span: Span) -> TokenStream {
    quote_spanned! {span=>
        ::wgpu::BindingType::Buffer {
            ty: ::wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        }
    }
}

fn uniform_binding_resource(span: Span, field: &Ident) -> TokenStream {
    quote_spanned! {span=> self.#field.wgpu_buffer().as_entire_binding() }
}

fn texture_view_layout_ty(span: Span, params: TextureViewBindingParams) -> TokenStream {
    let sample_type = match params.sample_type {
        TextureViewSampleType::Float => quote! { Float { filterable: true } },
        TextureViewSampleType::FloatUnfilterable => quote! { Float { filterable: false } },
        TextureViewSampleType::Depth => quote! { Depth },
        TextureViewSampleType::Sint => quote! { Sint },
        TextureViewSampleType::Uint => quote! { Uint },
    };
    let view_dimension = match params.view_dimension {
        1 => quote! { D1 },
        2 => quote! { D2 },
        3 => quote! { D3 },
        _ => panic!("view_dimension can only be 1, 2 or 3"),
    };
    let multisampled = params.multisampled;
    quote_spanned! {span=>
        ::wgpu::BindingType::Texture {
            sample_type: ::wgpu::TextureSampleType::#sample_type,
            view_dimension: ::wgpu::TextureViewDimension::#view_dimension,
            multisampled: #multisampled,
        }
    }
}

fn texture_view_binding_resource(span: Span, field: &Ident) -> TokenStream {
    quote_spanned! {span=> wgpu::BindingResource::TextureView(&self.#field) }
}

fn sampler_layout_ty(span: Span, sample_binding_type: SamplerBindingType) -> TokenStream {
    let type_ = match sample_binding_type {
        SamplerBindingType::Filtering => quote! { Filtering },
        SamplerBindingType::NonFiltering => quote! { NonFiltering },
        SamplerBindingType::Comparison => quote! { Comparison },
    };
    quote_spanned! {span=>
        ::wgpu::BindingType::Sampler(::wgpu::SamplerBindingType::#type_)
    }
}

fn sampler_binding_resource(span: Span, field: &Ident) -> TokenStream {
    quote_spanned! {span=> wgpu::BindingResource::Sampler(&self.#field) }
}
