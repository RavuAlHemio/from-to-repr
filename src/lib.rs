#[cfg(feature = "from_to_other")]
mod from_to_other_impl;


use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Meta, NestedMeta, parse_macro_input};


/// Derives [`TryFrom`] and [`From`] implementations for the representation type of an enumeration.
///
/// ```
/// use from_to_repr::FromToRepr;
///
/// #[derive(FromToRepr)]
/// #[repr(u8)]
/// enum ColorChannel {
///     RED = 0,
///     GREEN = 1,
///     BLUE = 2,
/// }
/// ```
/// is equivalent to
/// ```
/// #[repr(u8)]
/// enum ColorChannel {
///     RED = 0,
///     GREEN = 1,
///     BLUE = 2,
/// }
/// impl ::core::convert::TryFrom<u8> for ColorChannel {
///     type Error = u8;
///     fn try_from(value: u8) -> Result<Self, Self::Error> {
///         if value == 0 {
///             Ok(Self::RED)
///         } else if value == 1 {
///             Ok(Self::GREEN)
///         } else if value == 2 {
///             Ok(Self::BLUE)
///         } else {
///             Err(value)
///         }
///     }
/// }
/// impl ::core::convert::From<ColorChannel> for u8 {
///     fn from(value: ColorChannel) -> Self {
///         match value {
///             ColorChannel::RED => 0,
///             ColorChannel::GREEN => 1,
///             ColorChannel::BLUE => 2,
///         }
///     }
/// }
/// ```
#[proc_macro_derive(FromToRepr)]
pub fn derive_from_to_repr(item: TokenStream) -> TokenStream {
    let ast: DeriveInput = parse_macro_input!(item);
    let enum_name = ast.ident;

    let enum_data = match ast.data {
        Data::Enum(ed) => ed,
        _ => panic!("#[derive(FromToPrimitive)] can only be applied to enums"),
    };

    let enum_repr = ast.attrs.iter()
        .filter(|attr| attr.path.is_ident("repr"))
        .nth(0)
        .expect("#[derive(FromToPrimitive)] can only be applied to enums with a #[repr(...)] attribute");
    let enum_repr_type = enum_repr.parse_meta()
        .expect("#[derive(FromToPrimitive)] failed to parse #[repr(...)] attribute");
    let enum_list = match enum_repr_type {
        Meta::List(el) => el,
        _ => panic!("#[derive(FromToPrimitive)] failed to parse #[repr(...)] attribute as a list"),
    };
    let reprs = enum_list.nested;
    let mut inner_type_opt = None;
    for repr in reprs {
        if let NestedMeta::Meta(repr_meta) = repr {
            if let Meta::Path(repr_type_path) = repr_meta {
                if let Some(ident) = repr_type_path.get_ident() {
                    if ident == "u8" || ident == "u16" || ident == "u32" || ident == "u64" || ident == "u128" || ident == "usize"
                            || ident == "i8" || ident == "i16" || ident == "i32" || ident == "i64" || ident == "i128" || ident == "isize" {
                        if let Some(existing_type) = &inner_type_opt {
                            panic!(
                                "#[derive(FromToPrimitive)] found multiple types in #[repr(...)] -- at least {:?} and {:?}",
                                existing_type, ident,
                            );
                        } else {
                            inner_type_opt = Some(ident.clone());
                        }
                    }
                }
            }
        }
    }

    let inner_type = match inner_type_opt {
        Some(it) => it,
        None => panic!("#[derive(FromToPrimitive)] did not find a type in #[repr(...)]"),
    };

    let mut from_enum_arms: Vec<proc_macro2::TokenStream> = Vec::with_capacity(enum_data.variants.len());
    let mut try_from_inner_ifs: Vec<proc_macro2::TokenStream> = Vec::with_capacity(enum_data.variants.len());
    for variant in enum_data.variants {
        let variant_name = variant.ident;

        if variant.fields.len() > 0 {
            panic!("#[derive(FromToPrimitive)] cannot be used on enums whose variants have fields");
        }

        let discriminant = match variant.discriminant {
            Some((_eq_sign, d)) => d,
            None => panic!("#[derive(FromToPrimitive)] requires that all enum entries have explicit discriminants"),
        };

        from_enum_arms.push(quote!{
            #enum_name::#variant_name => #discriminant,
        });
        try_from_inner_ifs.push(quote!{
            if value == #discriminant {
                Ok(Self::#variant_name)
            } else
        });
    }

    let expanded = quote! {
        impl ::core::convert::TryFrom<#inner_type> for #enum_name {
            type Error = #inner_type;
            fn try_from(value: #inner_type) -> Result<Self, Self::Error> {
                #(#try_from_inner_ifs)*
                {
                    Err(value)
                }
            }
        }
        impl ::core::convert::From<#enum_name> for #inner_type {
            fn from(value: #enum_name) -> Self {
                match value {
                    #(#from_enum_arms)*
                }
            }
        }
    };

    TokenStream::from(expanded)
}


/// Derives [`From`] implementations for an enumeration and a specified type. Unknown values are
/// mapped to an _Other_ variant (which can have a custom name).
///
/// The following arguments can be passed to this macro:
///
/// * `base_type` (required): The type used to represent values of this enumeration.
///
/// * `derive_compare` (optional): Automatically derives comparison traits ([`PartialEq`],
///   [`PartialOrd`], [`Eq`], [`Ord`], [`core::hash::Hash`]) according to the value.
///
///     * If the value is `"none"` (the default), no comparison traits are derived.
/// 
///     * If the value is `"as_enum"`, the comparison traits are derived as is common in Rust
///       (equivalent to specifying `#[derive(PartialEq, PartialOrd, Eq, Ord, Hash)]` on the enum),
///       which means that every `Other(_)` value will compare not-equal to every known value (e.g.
///       `Self::Other(12) != Self::Twelve` where `Twelve = 12`).
///
///     * If the value is `"as_int"`, a custom derivation of these five traits is used, which
///       compares the base-type representations. This means that an `Other(_)` value will compare
///       equal to the matching known value (e.g. `Self::Other(12) == Self::Twelve` where
///       `Twelve = 12`).
///
/// ```
/// use from_to_repr::from_to_other;
///
/// #[from_to_other(base_type = u8)]
/// enum ColorCommand {
///     SetRed = 0,
///     SetGreen = 1,
///     SetBlue = 2,
///     Other(u8),
/// }
/// ```
/// is equivalent to
/// ```
/// enum ColorCommand {
///     SetRed,
///     SetGreen,
///     SetBlue,
///     Other(u8),
/// }
/// impl ::core::convert::From<u8> for ColorCommand {
///     fn from(base_value: u8) -> Self {
///         if base_value == 0 {
///             Self::SetRed
///         } else if base_value == 1 {
///             Self::SetGreen
///         } else if base_value == 2 {
///             Self::SetBlue
///         } else {
///             Self::Other(value)
///         }
///     }
/// }
/// impl ::core::convert::From<ColorCommand> for u8 {
///     fn from(enum_value: ColorCommand) -> Self {
///         match enum_value {
///             ColorCommand::SetRed => 0,
///             ColorCommand::SetGreen => 1,
///             ColorCommand::SetBlue => 2,
///             ColorCommand::Other(other) => other,
///         }
///     }
/// }
/// ```
#[cfg(feature = "from_to_other")]
#[cfg_attr(docsrs, doc(cfg(feature = "from_to_other")))]
#[proc_macro_attribute]
pub fn from_to_other(attr: TokenStream, item: TokenStream) -> TokenStream {
    use proc_macro2::{TokenStream as TokenStream2, TokenTree};
    use syn::{Error, Expr, Ident, ItemEnum, LitStr, Type, Variant};
    use syn::punctuated::Punctuated;
    use syn::spanned::Spanned;

    use crate::from_to_other_impl::KeyValuePairs;

    let enum_def = parse_macro_input!(item as ItemEnum);
    let args = parse_macro_input!(attr as KeyValuePairs);

    let enum_name = enum_def.ident.clone();

    enum DeriveCompareMode {
        None,
        AsEnum,
        AsInt,
    }
    let mut derive_compare_mode = DeriveCompareMode::None;
    let mut derive_compare_mode_set = false;

    let mut base_type_opt = None;
    for arg in args.kvps {
        if arg.path.is_ident("base_type") {
            if base_type_opt.is_some() {
                return Error::new(arg.path.span(), "cannot set \"base_type\" more than once")
                    .to_compile_error()
                    .into();
            } else if let TokenTree::Ident(ident) = arg.token_tree {
                let lit_string = ident.to_string();
                if lit_string == "u8" || lit_string == "u16" || lit_string == "u16" || lit_string == "u32" || lit_string == "u64" || lit_string == "u128" || lit_string == "usize"
                        || lit_string == "i8" || lit_string == "i16" || lit_string == "i16" || lit_string == "i32" || lit_string == "i64" || lit_string == "i128" || lit_string == "isize" {
                    base_type_opt = Some(ident)
                } else {
                    return Error::new(ident.span(), "\"base_type\" value must be an integral type like u8")
                        .to_compile_error()
                        .into();
                }
            } else {
                return Error::new(arg.token_tree.span(), "\"base_type\" value must be an integral type like u8")
                    .to_compile_error()
                    .into();
            }
        } else if arg.path.is_ident("derive_compare") {
            if derive_compare_mode_set {
                return Error::new(arg.path.span(), "cannot set \"derive_compare\" more than once")
                    .to_compile_error()
                    .into();
            } else if let TokenTree::Literal(ident_literal) = arg.token_tree {
                let ident_stream: TokenStream2 = TokenTree::from(ident_literal.clone()).into();
                let ident_result: Result<LitStr, _> = syn::parse2(ident_stream);
                if let Ok(ident) = ident_result {
                    match ident.value().as_str() {
                        "none" => {
                            derive_compare_mode = DeriveCompareMode::None;
                            derive_compare_mode_set = true;
                        },
                        "as_enum" => {
                            derive_compare_mode = DeriveCompareMode::AsEnum;
                            derive_compare_mode_set = true;
                        },
                        "as_int" => {
                            derive_compare_mode = DeriveCompareMode::AsInt;
                            derive_compare_mode_set = true;
                        },
                        _ => {
                            return Error::new(ident.span(), "\"derive_compare\" value must be one of: \"none\", \"as_enum\", \"as_int\"")
                                .to_compile_error()
                                .into();
                        },
                    }
                } else {
                    return Error::new(ident_literal.span(), "\"derive_compare\" value must be a string literal")
                        .to_compile_error()
                        .into();
                };
            } else {
                return Error::new(arg.token_tree.span(), "\"derive_compare\" value must be a string literal")
                    .to_compile_error()
                    .into();
            }
        } else {
            return Error::new(arg.eq_token.span, "unknown argument")
                .to_compile_error()
                .into();
        }
    }

    let base_type = match base_type_opt {
        Some(bt) => bt,
        None => return Error::new(enum_def.enum_token.span, "\"base_type\" argument on \"from_to_other\" attribute is required, e.g. #[from_to_other(base_type = u8)]")
            .to_compile_error()
            .into(),
    };

    // process the enum's variants
    let mut enum_key_to_value: Vec<(Ident, Expr)> = Vec::new();
    let mut cut_variants = Punctuated::new();
    let mut other_value_name_opt = None;
    for variant in enum_def.variants.iter() {
        if let Some((_, discr)) = &variant.discriminant {
            if !variant.fields.is_empty() {
                return Error::new(variant.span(), "enum variant must have either a field or a discriminant, not both")
                    .to_compile_error()
                    .into();
            }

            // remember the discriminant for later; we will be removing it now
            enum_key_to_value.push((variant.ident.clone(), discr.clone()));
        } else {
            if other_value_name_opt.is_some() {
                return Error::new(variant.span(), "only one value (the \"other\" value) may contain a field instead of a discriminant")
                    .to_compile_error()
                    .into();
            }
            if variant.fields.len() != 1 {
                return Error::new(variant.span(), "all values must contain a discriminant, except the \"other\" value, which must contain exactly one field")
                    .to_compile_error()
                    .into();
            }
            let one_field = variant.fields.iter().nth(0).unwrap();
            if one_field.ident.is_some() {
                return Error::new(one_field.span(), "the \"other\" value's field must be unnamed")
                    .to_compile_error()
                    .into();
            }
            if let Type::Path(one_field_type) = &one_field.ty {
                if !one_field_type.path.is_ident(&base_type) {
                    return Error::new(one_field_type.span(), "the \"other\" value's field must be of the enum's base type")
                        .to_compile_error()
                        .into();
                }
                if one_field_type.qself.is_some() {
                    return Error::new(one_field_type.span(), "the \"other\" value's field's type must not have a self-qualifier")
                        .to_compile_error()
                        .into();
                }
            }
            other_value_name_opt = Some(variant.ident.clone());
        }
        cut_variants.push(Variant {
            attrs: variant.attrs.clone(),
            ident: variant.ident.clone(),
            discriminant: None,
            fields: variant.fields.clone(),
        });
    }

    let other_value_name = match other_value_name_opt {
        Some(ovn) => ovn,
        None => return Error::new(enum_def.ident.span(), "the enumeration does not have an \"other\" value for unmatched values")
            .to_compile_error()
            .into(),
    };

    // prepare the enum (without the discriminants)
    let cut_enum = ItemEnum {
        attrs: enum_def.attrs.clone(),
        vis: enum_def.vis,
        enum_token: enum_def.enum_token,
        ident: enum_def.ident,
        generics: enum_def.generics,
        brace_token: enum_def.brace_token,
        variants: cut_variants,
    };

    // implement the conversion from the base type
    let from_base_type_impl = if enum_key_to_value.is_empty() {
        // there's only the "other" variant
        quote! {
            impl ::core::convert::From<#base_type> for #enum_name {
                fn from(base_value: #base_type) -> Self {
                    Self::#other_value_name(base_value)
                }
            }
        }
    } else {
        let pieces: Vec<TokenStream2> = enum_key_to_value.iter().map(
            |(key, value)| quote! {
                if base_value == #value {
                    Self::#key
                } else
            }
        )
            .collect();
        quote! {
            impl ::core::convert::From<#base_type> for #enum_name {
                fn from(base_value: #base_type) -> Self {
                    #(#pieces)*
                    {
                        Self::#other_value_name(base_value)
                    }
                }
            }
        }
    };

    // implement the conversion to the base type
    let to_base_type_impl = if enum_key_to_value.is_empty() {
        // there's only the "other" variant
        quote! {
            impl ::core::convert::From<#enum_name> for #base_type {
                fn from(enum_value: #enum_name) -> Self {
                    match enum_value {
                        Self::#other_value_name(v) => v,
                    }
                }
            }
        }
    } else {
        let variants = enum_key_to_value.into_iter().map(
            |(key, value)| quote! {
                #enum_name::#key => #value,
            }
        );
        quote! {
            impl ::core::convert::From<#enum_name> for #base_type {
                fn from(enum_value: #enum_name) -> Self {
                    match enum_value {
                        #(#variants)*
                        #enum_name::#other_value_name(v) => v,
                    }
                }
            }
        }
    };

    // derive or implement the comparisons
    let derive_compare_top = if let DeriveCompareMode::AsEnum = derive_compare_mode {
        // standard derived implementation of Eq and Ord
        // (Other always compares not-equal with a matching known value)
        quote! {
            #[derive(Eq, Hash, Ord, PartialEq, PartialOrd)]
        }
    } else {
        quote! {}
    };
    let derive_compare_impl = if let DeriveCompareMode::AsInt = derive_compare_mode {
        // integer-based implementation of Eq and Ord
        // (Other compares equal with a matching known value)
        quote! {
            impl ::core::cmp::PartialEq for #enum_name {
                fn eq(&self, other: &Self) -> bool {
                    #base_type::from(*self).eq(&#base_type::from(*other))
                }
            }
            impl ::core::cmp::Eq for #enum_name {}
            impl ::core::cmp::PartialOrd for #enum_name {
                fn partial_cmp(&self, other: &Self) -> Option<::core::cmp::Ordering> {
                    #base_type::from(*self).partial_cmp(&#base_type::from(*other))
                }
            }
            impl ::core::cmp::Ord for #enum_name {
                fn cmp(&self, other: &Self) -> ::core::cmp::Ordering {
                    #base_type::from(*self).cmp(&#base_type::from(*other))
                }
            }
            impl ::core::hash::Hash for #enum_name {
                fn hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
                    #base_type::from(*self).hash(state)
                }
            }
        }
    } else {
        quote! {}
    };

    let output = quote! {
        #derive_compare_top
        #cut_enum
        #from_base_type_impl
        #to_base_type_impl
        #derive_compare_impl
    };
    TokenStream::from(output)
}
