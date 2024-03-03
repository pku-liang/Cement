use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{Ident, Visibility};

pub fn make_index_enum_trait(
    result: &mut TokenStream,
    vars: &Vec<Ident>,
    name: &Ident,
    vis: &Visibility,
) {
    let mut modify_arms = Vec::new();
    for v in vars {
        modify_arms.push(quote! {
            #name::#v(idx) => #name::#v(new_idx),
        })
    }
    let mut index_arms = Vec::new();
    for v in vars {
        index_arms.push(quote! {
            #name::#v(idx) => idx.clone(),
        })
    }
    quote!(
        #vis impl tgraph::typed_graph::IndexEnum for #name{
            fn modify(&mut self, new_idx:NodeIndex) {
                *self = match self {
                    #(#modify_arms)*
                }
            }
            fn index(&self) -> NodeIndex {
                match self {
                    #(#index_arms)*
                }
            }
        }
    )
    .to_tokens(result);
}
