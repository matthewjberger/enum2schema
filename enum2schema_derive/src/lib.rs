use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Data, DeriveInput, Fields, FieldsNamed, LitStr, Variant, parse_macro_input, spanned::Spanned,
};

#[proc_macro_derive(Schema, attributes(schema, serde))]
pub fn derive_schema(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn expand(input: DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let container = parse_serde_attrs(&input.attrs)?;
    let container_schema = parse_field_schema_attrs(&input.attrs)?;
    let description = container_schema
        .description
        .clone()
        .or_else(|| extract_doc(&input.attrs));

    let body = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(named) => struct_object_schema(
                named,
                container.rename_all.as_deref(),
                description.as_deref(),
            )?,
            _ => {
                return Err(syn::Error::new(
                    input.span(),
                    "enum2schema only supports structs with named fields",
                ));
            }
        },
        Data::Enum(data) => {
            let description_insert = match &description {
                Some(text) => quote! {
                    __map.insert(
                        "description".to_string(),
                        enum2schema::serde_json::Value::String(#text.to_string()),
                    );
                },
                None => quote! {},
            };
            if container_schema.string_enum {
                let mut tags = Vec::new();
                for variant in &data.variants {
                    if parse_field_schema_attrs(&variant.attrs)?.skip {
                        continue;
                    }
                    if !matches!(variant.fields, Fields::Unit) {
                        return Err(syn::Error::new(
                            variant.span(),
                            "schema(string_enum) requires every variant to be a unit variant",
                        ));
                    }
                    let serde_attrs = parse_serde_attrs(&variant.attrs)?;
                    let tag = apply_name(
                        &variant.ident.to_string(),
                        serde_attrs.rename.as_deref(),
                        container.rename_all.as_deref(),
                    );
                    tags.push(LitStr::new(&tag, variant.ident.span()));
                }
                quote! {
                    {
                        let mut __map = enum2schema::serde_json::Map::new();
                        #description_insert
                        __map.insert(
                            "type".to_string(),
                            enum2schema::serde_json::Value::String("string".to_string()),
                        );
                        __map.insert(
                            "enum".to_string(),
                            enum2schema::serde_json::Value::Array(vec![
                                #(enum2schema::serde_json::Value::String(#tags.to_string())),*
                            ]),
                        );
                        enum2schema::serde_json::Value::Object(__map)
                    }
                }
            } else {
                let mut variants = Vec::new();
                for variant in &data.variants {
                    if parse_field_schema_attrs(&variant.attrs)?.skip {
                        continue;
                    }
                    variants.push(variant_schema_expr(
                        variant,
                        container.rename_all.as_deref(),
                    )?);
                }
                quote! {
                    {
                        let mut __map = enum2schema::serde_json::Map::new();
                        #description_insert
                        __map.insert(
                            "oneOf".to_string(),
                            enum2schema::serde_json::Value::Array(vec![ #(#variants),* ]),
                        );
                        enum2schema::serde_json::Value::Object(__map)
                    }
                }
            }
        }
        Data::Union(_) => {
            return Err(syn::Error::new(
                input.span(),
                "enum2schema does not support unions",
            ));
        }
    };

    let generics = add_trait_bounds(input.generics.clone());
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics enum2schema::Schema for #name #ty_generics #where_clause {
            fn schema() -> enum2schema::serde_json::Value {
                #body
            }
        }
    })
}

fn struct_object_schema(
    fields: &FieldsNamed,
    container_rename_all: Option<&str>,
    description: Option<&str>,
) -> syn::Result<TokenStream2> {
    let mut inserts = Vec::new();
    let mut required = Vec::new();

    for field in &fields.named {
        let schema_attrs = parse_field_schema_attrs(&field.attrs)?;
        if schema_attrs.skip {
            continue;
        }
        let serde_attrs = parse_serde_attrs(&field.attrs)?;
        let ident = field.ident.as_ref().unwrap();
        let property = apply_name(
            &ident.to_string(),
            serde_attrs.rename.as_deref(),
            container_rename_all,
        );
        let is_required = !is_option(&field.ty) && !serde_attrs.default;
        let field_description = schema_attrs
            .description
            .clone()
            .or_else(|| extract_doc(&field.attrs));

        let schema_expr = field_schema_expr(&field.ty, &schema_attrs);
        let schema_expr = match &field_description {
            Some(text) => quote! { enum2schema::set_description(#schema_expr, #text) },
            None => schema_expr,
        };

        inserts.push(quote! {
            __properties.insert(#property.to_string(), #schema_expr);
        });
        if is_required {
            required.push(quote! {
                enum2schema::serde_json::Value::String(#property.to_string())
            });
        }
    }

    let description_insert = match description {
        Some(text) => quote! {
            __map.insert(
                "description".to_string(),
                enum2schema::serde_json::Value::String(#text.to_string()),
            );
        },
        None => quote! {},
    };

    let required_insert = if required.is_empty() {
        quote! {}
    } else {
        quote! {
            __map.insert(
                "required".to_string(),
                enum2schema::serde_json::Value::Array(vec![ #(#required),* ]),
            );
        }
    };

    Ok(quote! {
        {
            let mut __map = enum2schema::serde_json::Map::new();
            __map.insert(
                "type".to_string(),
                enum2schema::serde_json::Value::String("object".to_string()),
            );
            #description_insert
            let mut __properties = enum2schema::serde_json::Map::new();
            #(#inserts)*
            __map.insert(
                "properties".to_string(),
                enum2schema::serde_json::Value::Object(__properties),
            );
            #required_insert
            enum2schema::serde_json::Value::Object(__map)
        }
    })
}

fn variant_schema_expr(
    variant: &Variant,
    container_rename_all: Option<&str>,
) -> syn::Result<TokenStream2> {
    let serde_attrs = parse_serde_attrs(&variant.attrs)?;
    let schema_attrs = parse_field_schema_attrs(&variant.attrs)?;
    let tag = apply_name(
        &variant.ident.to_string(),
        serde_attrs.rename.as_deref(),
        container_rename_all,
    );
    let tag_lit = LitStr::new(&tag, variant.ident.span());
    let description = schema_attrs
        .description
        .clone()
        .or_else(|| extract_doc(&variant.attrs));

    let base = match &variant.fields {
        Fields::Unit => quote! {
            {
                let mut __map = enum2schema::serde_json::Map::new();
                __map.insert(
                    "type".to_string(),
                    enum2schema::serde_json::Value::String("string".to_string()),
                );
                __map.insert(
                    "const".to_string(),
                    enum2schema::serde_json::Value::String(#tag_lit.to_string()),
                );
                enum2schema::serde_json::Value::Object(__map)
            }
        },
        Fields::Named(named) => {
            let inner = struct_object_schema(named, None, None)?;
            wrap_externally_tagged(&tag_lit, inner)
        }
        Fields::Unnamed(unnamed) => {
            let unnamed_fields: Vec<_> = unnamed.unnamed.iter().collect();
            if unnamed_fields.len() == 1 {
                let field_type = &unnamed_fields[0].ty;
                let inner = quote! { <#field_type as enum2schema::Schema>::schema() };
                wrap_externally_tagged(&tag_lit, inner)
            } else {
                let count = unnamed_fields.len() as u64;
                let item_schemas = unnamed_fields.iter().map(|field| {
                    let field_type = &field.ty;
                    quote! { <#field_type as enum2schema::Schema>::schema() }
                });
                let inner = quote! {
                    {
                        let mut __array = enum2schema::serde_json::Map::new();
                        __array.insert(
                            "type".to_string(),
                            enum2schema::serde_json::Value::String("array".to_string()),
                        );
                        __array.insert(
                            "items".to_string(),
                            enum2schema::serde_json::Value::Array(vec![ #(#item_schemas),* ]),
                        );
                        __array.insert(
                            "minItems".to_string(),
                            enum2schema::serde_json::Value::from(#count),
                        );
                        __array.insert(
                            "maxItems".to_string(),
                            enum2schema::serde_json::Value::from(#count),
                        );
                        enum2schema::serde_json::Value::Object(__array)
                    }
                };
                wrap_externally_tagged(&tag_lit, inner)
            }
        }
    };

    Ok(match description {
        Some(text) => quote! { enum2schema::set_description(#base, #text) },
        None => base,
    })
}

fn wrap_externally_tagged(tag_lit: &LitStr, inner: TokenStream2) -> TokenStream2 {
    quote! {
        {
            let mut __outer = enum2schema::serde_json::Map::new();
            __outer.insert(
                "type".to_string(),
                enum2schema::serde_json::Value::String("object".to_string()),
            );
            let mut __outer_properties = enum2schema::serde_json::Map::new();
            __outer_properties.insert(#tag_lit.to_string(), #inner);
            __outer.insert(
                "properties".to_string(),
                enum2schema::serde_json::Value::Object(__outer_properties),
            );
            __outer.insert(
                "required".to_string(),
                enum2schema::serde_json::Value::Array(vec![
                    enum2schema::serde_json::Value::String(#tag_lit.to_string())
                ]),
            );
            enum2schema::serde_json::Value::Object(__outer)
        }
    }
}

fn field_schema_expr(field_type: &syn::Type, attrs: &FieldSchemaAttrs) -> TokenStream2 {
    if let Some(path) = &attrs.with {
        return quote! { #path() };
    }
    if let Some(type_override) = &attrs.type_override {
        let mut inserts = Vec::new();
        inserts.push(quote! {
            __override.insert(
                "type".to_string(),
                enum2schema::serde_json::Value::String(#type_override.to_string()),
            );
        });
        if let Some(items) = &attrs.items_override {
            inserts.push(quote! {
                let mut __items = enum2schema::serde_json::Map::new();
                __items.insert(
                    "type".to_string(),
                    enum2schema::serde_json::Value::String(#items.to_string()),
                );
                __override.insert(
                    "items".to_string(),
                    enum2schema::serde_json::Value::Object(__items),
                );
            });
        }
        if let Some(length) = attrs.len_override {
            let length = length as u64;
            inserts.push(quote! {
                __override.insert(
                    "minItems".to_string(),
                    enum2schema::serde_json::Value::from(#length),
                );
                __override.insert(
                    "maxItems".to_string(),
                    enum2schema::serde_json::Value::from(#length),
                );
            });
        }
        return quote! {
            {
                let mut __override = enum2schema::serde_json::Map::new();
                #(#inserts)*
                enum2schema::serde_json::Value::Object(__override)
            }
        };
    }
    quote! { <#field_type as enum2schema::Schema>::schema() }
}

#[derive(Default)]
struct FieldSchemaAttrs {
    skip: bool,
    string_enum: bool,
    with: Option<syn::Path>,
    description: Option<String>,
    type_override: Option<String>,
    items_override: Option<String>,
    len_override: Option<usize>,
}

fn parse_field_schema_attrs(attrs: &[syn::Attribute]) -> syn::Result<FieldSchemaAttrs> {
    let mut result = FieldSchemaAttrs::default();
    for attr in attrs {
        if !attr.path().is_ident("schema") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip") {
                result.skip = true;
                return Ok(());
            }
            if meta.path.is_ident("string_enum") {
                result.string_enum = true;
                return Ok(());
            }
            if meta.path.is_ident("with") {
                result.with = Some(meta.value()?.parse()?);
                return Ok(());
            }
            if meta.path.is_ident("description") {
                let literal: LitStr = meta.value()?.parse()?;
                result.description = Some(literal.value());
                return Ok(());
            }
            if meta.path.is_ident("type") {
                let literal: LitStr = meta.value()?.parse()?;
                result.type_override = Some(literal.value());
                return Ok(());
            }
            if meta.path.is_ident("items") {
                let literal: LitStr = meta.value()?.parse()?;
                result.items_override = Some(literal.value());
                return Ok(());
            }
            if meta.path.is_ident("len") {
                let literal: syn::LitInt = meta.value()?.parse()?;
                result.len_override = Some(literal.base10_parse()?);
                return Ok(());
            }
            Err(meta.error("unknown enum2schema attribute"))
        })?;
    }
    Ok(result)
}

#[derive(Default)]
struct SerdeAttrs {
    rename: Option<String>,
    rename_all: Option<String>,
    default: bool,
}

fn parse_serde_attrs(attrs: &[syn::Attribute]) -> syn::Result<SerdeAttrs> {
    let mut result = SerdeAttrs::default();
    for attr in attrs {
        if !attr.path().is_ident("serde") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            let is_rename = meta.path.is_ident("rename");
            let is_rename_all = meta.path.is_ident("rename_all");
            if meta.path.is_ident("default") {
                result.default = true;
            }
            if meta.input.peek(syn::Token![=]) {
                let value: syn::Lit = meta.value()?.parse()?;
                if let syn::Lit::Str(literal) = value {
                    if is_rename {
                        result.rename = Some(literal.value());
                    } else if is_rename_all {
                        result.rename_all = Some(literal.value());
                    }
                }
            } else if meta.input.peek(syn::token::Paren) {
                let content;
                syn::parenthesized!(content in meta.input);
                let _: TokenStream2 = content.parse()?;
            }
            Ok(())
        })?;
    }
    Ok(result)
}

fn extract_doc(attrs: &[syn::Attribute]) -> Option<String> {
    let mut lines = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("doc") {
            continue;
        }
        if let syn::Meta::NameValue(name_value) = &attr.meta
            && let syn::Expr::Lit(expr_lit) = &name_value.value
            && let syn::Lit::Str(literal) = &expr_lit.lit
        {
            lines.push(literal.value().trim().to_string());
        }
    }
    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

fn is_option(field_type: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = field_type
        && let Some(segment) = type_path.path.segments.last()
    {
        return segment.ident == "Option";
    }
    false
}

fn add_trait_bounds(mut generics: syn::Generics) -> syn::Generics {
    for param in &mut generics.params {
        if let syn::GenericParam::Type(type_param) = param {
            type_param
                .bounds
                .push(syn::parse_quote!(enum2schema::Schema));
        }
    }
    generics
}

fn apply_name(original: &str, rename: Option<&str>, rename_all: Option<&str>) -> String {
    if let Some(rename) = rename {
        return rename.to_string();
    }
    if let Some(rule) = rename_all {
        return apply_rename_all(original, rule);
    }
    original.to_string()
}

fn apply_rename_all(name: &str, rule: &str) -> String {
    match rule {
        "lowercase" => name.to_lowercase(),
        "UPPERCASE" => name.to_uppercase(),
        "PascalCase" => to_pascal_case(name),
        "camelCase" => to_camel_case(name),
        "snake_case" => to_snake_case(name),
        "SCREAMING_SNAKE_CASE" => to_snake_case(name).to_uppercase(),
        "kebab-case" => to_snake_case(name).replace('_', "-"),
        "SCREAMING-KEBAB-CASE" => to_snake_case(name).to_uppercase().replace('_', "-"),
        _ => name.to_string(),
    }
}

fn to_snake_case(name: &str) -> String {
    let mut result = String::new();
    for (index, character) in name.chars().enumerate() {
        if character.is_uppercase() {
            if index != 0 {
                result.push('_');
            }
            result.extend(character.to_lowercase());
        } else {
            result.push(character);
        }
    }
    result
}

fn to_pascal_case(name: &str) -> String {
    name.split('_')
        .map(|part| {
            let mut characters = part.chars();
            match characters.next() {
                Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

fn to_camel_case(name: &str) -> String {
    let pascal = to_pascal_case(name);
    let mut characters = pascal.chars();
    match characters.next() {
        Some(first) => first.to_lowercase().collect::<String>() + characters.as_str(),
        None => String::new(),
    }
}
