use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::quote;
use syn::{Field, Ident, ItemStruct, Type, Visibility, parse_macro_input};

use syn::Token;

#[proc_macro_attribute]
pub fn glideable(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemStruct);
    let ident = input.ident.clone();
    match input.fields {
        syn::Fields::Named(ref mut named) => {
            let new_field = Field {
                mutability: syn::FieldMutability::None,
                attrs: Vec::new(),
                vis: Visibility::Inherited,
                ident: Some(Ident::new("sys_id", Span::call_site())),
                colon_token: Some(Token![:](Span::call_site())),
                ty: syn::parse_str::<Type>("String").unwrap(),
            };
            let has_sys_id = named
                .named
                .iter()
                .any(|field| field.ident.as_ref().is_some_and(|ident| ident == "sys_id"));
            if !has_sys_id {
                named.named.push(new_field);
            }
        }
        _ => {}
    }

    let found_crate = crate_name("snow_api").expect("serde is present in `Cargo.toml`");

    let (serde, serde_att) = match found_crate {
        FoundCrate::Itself => (
            quote!(crate::serde),
            quote!(#[serde(crate = "crate::serde")]),
        ),
        FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            let att = format!("{}::serde", name);
            (quote!(#ident::serde), quote!(#[serde(crate = #att)]))
        }
    };
    let expanded = quote! {
        #[derive(Debug, #serde::Serialize, #serde::Deserialize)]
        #serde_att
        #input
        impl Glideable for #ident {
            fn sys_id(&self) -> &String {
                &self.sys_id
            }
            fn update(&self, gr: &GlideRecord<Self>) -> Option<Self> {
                gr.update(&self, self.sys_id())
            }
            fn insert(&self, gr: &GlideRecord<Self>) -> Option<Self> {
                gr.insert(&self)
            }
            fn delete(&self, gr: &GlideRecord<Self>) -> Option<Self> {
                gr.delete(self.sys_id())
            }
        }
    };
    eprintln!("{}", expanded);

    TokenStream::from(expanded)
}
