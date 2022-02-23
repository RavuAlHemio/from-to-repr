use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Meta, NestedMeta, parse_macro_input};


/// Derives [`TryFrom`] and [`From`] implementations for the representation type of an enumeration.
///
/// ```
/// use bomsfront_proc_macros::FromToRepr;
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
/// impl ::std::convert::TryFrom<u8> for ColorChannel {
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
/// impl ::std::convert::From<ColorChannel> for u8 {
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
        impl ::std::convert::TryFrom<#inner_type> for #enum_name {
            type Error = #inner_type;
            fn try_from(value: #inner_type) -> Result<Self, Self::Error> {
                #(#try_from_inner_ifs)*
                {
                    Err(value)
                }
            }
        }
        impl ::std::convert::From<#enum_name> for #inner_type {
            fn from(value: #enum_name) -> Self {
                match value {
                    #(#from_enum_arms)*
                }
            }
        }
    };

    TokenStream::from(expanded)
}
