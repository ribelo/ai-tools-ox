use darling::{ast, util, FromDeriveInput, FromField};
use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use syn::{Data, DeriveInput, Fields};

#[derive(Debug, FromField)]
#[darling(attributes(description))]
struct StructField {
    ident: Option<syn::Ident>,
    ty: syn::Type,
    #[darling(default)]
    description: Option<String>,
}

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(description), supports(struct_any))]
struct ObjectReceiver {
    ident: syn::Ident,
    generics: syn::Generics,
    data: ast::Data<(), StructField>,
}

impl ToTokens for ObjectReceiver {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ObjectReceiver {
            ref ident,
            ref generics,
            ref data,
            ..
        } = *self;

        let (imp, ty, wher) = generics.split_for_impl();

        let extracted_fields = data
            .as_ref()
            .take_struct()
            .unwrap()
            .fields
            .iter()
            .map(|f| {
                let name = f.ident.as_ref().unwrap();
                let ty = &f.ty;
                if let Some(description) = &f.description {
                    quote! {
                        stringify!(#name): serde_json::json!({
                            "type": <#ty as Jsonify>::jsonify(),
                            "description": #description
                        })
                    }
                } else {
                    quote! {
                        stringify!(#name): serde_json::json!({
                            "type": <#ty as Jsonify>::jsonify(),
                        })
                    }
                }
            })
            .collect::<Vec<_>>();

        tokens.extend(quote! {
            impl #imp Jsonify for #ident #ty #wher {
                fn jsonify() -> serde_json::Value {
                    serde_json::json!({ #(#extracted_fields),* })
                }
            }
        });
    }
}

pub fn expand(input: &DeriveInput) -> TokenStream {
    match ObjectReceiver::from_derive_input(input) {
        Ok(receiver) => {
            let tokens = quote!(#receiver);
            dbg!(&tokens);
            tokens
        }
        Err(e) => {
            // Możesz obsłużyć błąd w bardziej złożony sposób, np. zwracając TokenStream z błędem
            panic!("Error parsing derive input: {}", e);
        }
    }
}
