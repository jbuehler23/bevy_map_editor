//! Derive macros for bevy_map_editor entity spawning
//!
//! This crate provides the `#[derive(MapEntity)]` macro for automatically
//! implementing entity spawning from map data.
//!
//! # Example
//!
//! ```rust,ignore
//! use bevy::prelude::*;
//! use bevy_map_derive::MapEntity;
//!
//! #[derive(Component, MapEntity)]
//! #[map_entity(type_name = "NPC")]
//! pub struct Npc {
//!     #[map_prop]
//!     pub name: String,
//!     #[map_prop(default = 100)]
//!     pub health: i32,
//!     #[map_sprite("sprite")]  // Optional: receives sprite handle when loaded
//!     pub sprite_handle: Option<Handle<Image>>,
//! }
//! ```

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input, Attribute, Data, DeriveInput, Expr, ExprLit, Fields, Ident, Lit, Meta, Type,
};

/// Derive macro for creating map entities that can be spawned from EntityInstance data
///
/// # Container Attributes
///
/// - `#[map_entity(type_name = "TypeName")]` - The entity type name as used in the map editor
///
/// # Field Attributes
///
/// - `#[map_prop]` - Mark a field as coming from entity properties
/// - `#[map_prop(name = "property_name")]` - Use a different property name than the field name
/// - `#[map_prop(default = value)]` - Default value if property is missing
/// - `#[map_sprite]` - Mark a field to receive sprite handle injection (field must be `Option<Handle<Image>>`)
/// - `#[map_sprite("property_name")]` - Use a different property name than the field name
#[proc_macro_derive(MapEntity, attributes(map_entity, map_prop, map_sprite))]
pub fn derive_map_entity(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match impl_map_entity(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn impl_map_entity(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;

    // Parse container attributes
    let type_name = parse_type_name(&input.attrs)?;

    // Get fields
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    input,
                    "MapEntity can only be derived for structs with named fields",
                ))
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "MapEntity can only be derived for structs",
            ))
        }
    };

    // Collect sprite fields: (field_name, property_name)
    let mut sprite_fields: Vec<(Ident, String)> = Vec::new();

    // Generate field initialization code
    let field_inits: Vec<TokenStream2> = fields
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let field_type = &field.ty;

            // Check for #[map_sprite] attribute
            if let Some(attr) = field
                .attrs
                .iter()
                .find(|attr| attr.path().is_ident("map_sprite"))
            {
                let prop_name = parse_map_sprite_attr(attr, field_name)?;
                sprite_fields.push((field_name.clone(), prop_name));
                // Sprite fields initialize to None
                return Ok(quote! {
                    #field_name: None
                });
            }

            // Check for #[map_prop] attribute
            let map_prop_attr = field
                .attrs
                .iter()
                .find(|attr| attr.path().is_ident("map_prop"));

            if let Some(attr) = map_prop_attr {
                let (prop_name, default_value) = parse_map_prop_attr(attr, field_name)?;
                generate_field_init(field_name, field_type, &prop_name, default_value)
            } else {
                // Field without #[map_prop] - use Default::default()
                Ok(quote! {
                    #field_name: Default::default()
                })
            }
        })
        .collect::<syn::Result<Vec<_>>>()?;

    // Generate sprite_properties implementation
    let sprite_properties_impl = if sprite_fields.is_empty() {
        quote! {
            fn sprite_properties() -> &'static [&'static str] {
                &[]
            }
        }
    } else {
        let prop_names: Vec<&str> = sprite_fields.iter().map(|(_, p)| p.as_str()).collect();
        quote! {
            fn sprite_properties() -> &'static [&'static str] {
                &[#(#prop_names),*]
            }
        }
    };

    // Generate inject_sprite_handle implementation
    let inject_sprite_impl = if sprite_fields.is_empty() {
        quote! {
            fn inject_sprite_handle(&mut self, _property_name: &str, _handle: bevy::prelude::Handle<bevy::prelude::Image>) {
                // No sprite fields
            }
        }
    } else {
        let match_arms: Vec<TokenStream2> = sprite_fields
            .iter()
            .map(|(field_name, prop_name)| {
                quote! {
                    #prop_name => { self.#field_name = Some(handle.clone()); }
                }
            })
            .collect();
        quote! {
            fn inject_sprite_handle(&mut self, property_name: &str, handle: bevy::prelude::Handle<bevy::prelude::Image>) {
                match property_name {
                    #(#match_arms)*
                    _ => {}
                }
            }
        }
    };

    // Use bevy_map paths if available, otherwise fall back to direct crate paths
    // This allows both `bevy_map` umbrella crate users and direct crate users to work
    let expanded = quote! {
        impl bevy_map::runtime::MapEntityType for #name {
            fn type_name() -> &'static str {
                #type_name
            }

            fn from_instance(instance: &bevy_map::core::EntityInstance) -> Self {
                Self {
                    #(#field_inits),*
                }
            }

            #sprite_properties_impl

            #inject_sprite_impl
        }
    };

    Ok(expanded)
}

fn parse_type_name(attrs: &[Attribute]) -> syn::Result<String> {
    for attr in attrs {
        if attr.path().is_ident("map_entity") {
            let meta = attr.meta.require_list()?;
            let nested: syn::punctuated::Punctuated<Meta, syn::Token![,]> =
                meta.parse_args_with(syn::punctuated::Punctuated::parse_terminated)?;

            for meta in nested {
                if let Meta::NameValue(nv) = meta {
                    if nv.path.is_ident("type_name") {
                        if let Expr::Lit(ExprLit {
                            lit: Lit::Str(lit_str),
                            ..
                        }) = &nv.value
                        {
                            return Ok(lit_str.value());
                        }
                    }
                }
            }
        }
    }

    Err(syn::Error::new(
        proc_macro2::Span::call_site(),
        "MapEntity requires #[map_entity(type_name = \"...\")]",
    ))
}

fn parse_map_prop_attr(
    attr: &Attribute,
    field_name: &Ident,
) -> syn::Result<(String, Option<TokenStream2>)> {
    let mut prop_name = field_name.to_string();
    let mut default_value = None;

    // Handle both #[map_prop] and #[map_prop(...)]
    match &attr.meta {
        Meta::Path(_) => {
            // Just #[map_prop] with no arguments
        }
        Meta::List(list) => {
            let nested: syn::punctuated::Punctuated<Meta, syn::Token![,]> =
                list.parse_args_with(syn::punctuated::Punctuated::parse_terminated)?;

            for meta in nested {
                if let Meta::NameValue(nv) = meta {
                    if nv.path.is_ident("name") {
                        if let Expr::Lit(ExprLit {
                            lit: Lit::Str(lit_str),
                            ..
                        }) = &nv.value
                        {
                            prop_name = lit_str.value();
                        }
                    } else if nv.path.is_ident("default") {
                        default_value = Some(nv.value.to_token_stream());
                    }
                }
            }
        }
        Meta::NameValue(_) => {
            return Err(syn::Error::new_spanned(
                attr,
                "Expected #[map_prop] or #[map_prop(...)]",
            ))
        }
    }

    Ok((prop_name, default_value))
}

/// Parse #[map_sprite] or #[map_sprite("property_name")] attribute
fn parse_map_sprite_attr(attr: &Attribute, field_name: &Ident) -> syn::Result<String> {
    // Default to field name as property name
    let mut prop_name = field_name.to_string();

    match &attr.meta {
        Meta::Path(_) => {
            // Just #[map_sprite] with no arguments - use field name
        }
        Meta::List(list) => {
            // #[map_sprite("property_name")] - parse the string literal
            let lit: syn::LitStr = list.parse_args()?;
            prop_name = lit.value();
        }
        Meta::NameValue(_) => {
            return Err(syn::Error::new_spanned(
                attr,
                "Expected #[map_sprite] or #[map_sprite(\"property_name\")]",
            ))
        }
    }

    Ok(prop_name)
}

fn generate_field_init(
    field_name: &Ident,
    field_type: &Type,
    prop_name: &str,
    default_value: Option<TokenStream2>,
) -> syn::Result<TokenStream2> {
    let type_str = quote!(#field_type).to_string();

    // Generate the appropriate getter based on field type
    let getter = if type_str.contains("String") {
        if let Some(default) = default_value {
            quote! {
                instance.get_string(#prop_name)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| #default.to_string())
            }
        } else {
            quote! {
                instance.get_string(#prop_name)
                    .map(|s| s.to_string())
                    .unwrap_or_default()
            }
        }
    } else if type_str.contains("i32")
        || type_str.contains("i64")
        || type_str.contains("u32")
        || type_str.contains("u64")
        || type_str.contains("usize")
    {
        // All integer types use get_int and cast to the target type
        if let Some(default) = default_value {
            quote! {
                instance.get_int(#prop_name)
                    .map(|v| v as #field_type)
                    .unwrap_or(#default)
            }
        } else {
            quote! {
                instance.get_int(#prop_name)
                    .map(|v| v as #field_type)
                    .unwrap_or_default()
            }
        }
    } else if type_str.contains("f32") || type_str.contains("f64") {
        if let Some(default) = default_value {
            quote! {
                instance.get_float(#prop_name)
                    .map(|v| v as #field_type)
                    .unwrap_or(#default)
            }
        } else {
            quote! {
                instance.get_float(#prop_name)
                    .map(|v| v as #field_type)
                    .unwrap_or_default()
            }
        }
    } else if type_str.contains("bool") {
        if let Some(default) = default_value {
            quote! {
                instance.get_bool(#prop_name)
                    .unwrap_or(#default)
            }
        } else {
            quote! {
                instance.get_bool(#prop_name)
                    .unwrap_or_default()
            }
        }
    } else {
        // For custom types, try FromStr or Default
        if let Some(default) = default_value {
            quote! {
                instance.get_string(#prop_name)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(#default)
            }
        } else {
            quote! {
                instance.get_string(#prop_name)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or_default()
            }
        }
    };

    Ok(quote! {
        #field_name: #getter
    })
}
