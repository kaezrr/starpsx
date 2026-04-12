use proc_macro::TokenStream;
use quote::quote;
use syn::DeriveInput;
use syn::parse_macro_input;

/// Derives back and forth infallible boolean conversions for Enums
#[proc_macro_derive(Boolable)]
pub fn boolable_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let BoolableEnum {
        enum_name,
        true_variant,
        false_variant,
    } = match BoolableEnum::parse(input) {
        Ok(ir) => ir,
        Err(err) => return err.to_compile_error().into(),
    };

    let implementation = quote! {
        impl ::core::convert::From<bool> for #enum_name {
            fn from(value: bool) -> Self {
                match value {
                    true => #enum_name::#true_variant,
                    false => #enum_name::#false_variant,
                }
            }
        }

        impl ::core::convert::From<#enum_name> for bool {
            fn from(value: #enum_name) -> bool {
                match value {
                    #enum_name::#true_variant => true,
                    #enum_name::#false_variant => false,
                }
            }
        }
    };

    implementation.into()
}

struct BoolableEnum {
    enum_name: syn::Ident,
    true_variant: syn::Ident,
    false_variant: syn::Ident,
}

impl BoolableEnum {
    fn parse(input: DeriveInput) -> Result<Self, syn::Error> {
        let enum_name = input.ident;
        let syn::Data::Enum(data_enum) = input.data else {
            return Err(syn::Error::new(enum_name.span(), "Only enums supported"));
        };

        let mut true_variant = None;
        let mut false_variant = None;

        if data_enum.variants.len() != 2 {
            return Err(syn::Error::new(
                enum_name.span(),
                "Enum must have exactly 2 variants",
            ));
        }

        for variant in &data_enum.variants {
            if !matches!(variant.fields, syn::Fields::Unit) {
                return Err(syn::Error::new(
                    variant.ident.span(),
                    "Variants cannot have fields",
                ));
            }

            if let Some((
                _,
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Int(lit_int),
                    ..
                }),
            )) = &variant.discriminant
            {
                let val: u8 = lit_int.base10_parse()?;
                match val {
                    0 => false_variant = Some(variant.ident.clone()),
                    1 => true_variant = Some(variant.ident.clone()),
                    _ => return Err(syn::Error::new(lit_int.span(), "Value must be 0 or 1")),
                }
            }
        }

        let Some(true_variant) = true_variant else {
            return Err(syn::Error::new(
                enum_name.span(),
                "Enum must have a true variant with = 1",
            ));
        };

        let Some(false_variant) = false_variant else {
            return Err(syn::Error::new(
                enum_name.span(),
                "Enum must have a false variant with = 0",
            ));
        };

        Ok(Self {
            enum_name,
            true_variant,
            false_variant,
        })
    }
}
