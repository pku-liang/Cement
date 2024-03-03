use proc_macro2::{self, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{Ident, Type, Visibility};

pub fn make_iterator_impl(
    result: &mut TokenStream,
    enumt: &Ident,
    ident: &Ident,
    ty: &Type,
    trait_ident: &Ident,
    vis: &Visibility,
) {
    let iter_ident = format_ident!("Iter{}", ident);

    quote! {
        impl<'a> #trait_ident<'a, #iter_ident<'a>> for #ty{
            fn iter_by_type(graph: &'a tgraph::typed_graph::Graph<#enumt>) -> #iter_ident<'a>{
                #iter_ident{
                    it: graph.iter_nodes(),
                }
            }
            fn get_by_type<'b>(graph: &'b tgraph::typed_graph::Graph<#enumt>, idx: tgraph::typed_graph::NodeIndex) -> Option<&#ty>{
                graph.get_node(idx).and_then(|x| if let #enumt::#ident(y) = x { Some(y) } else { None })
            }
        }

        #vis struct #iter_ident<'a> {
            it: tgraph::typed_graph::Iter<'a, #enumt>
        }
        impl<'a> std::iter::Iterator for #iter_ident<'a>{
            type Item = (NodeIndex, &'a #ty);
            fn next(&mut self) -> Option<Self::Item> {
                self.it.next().and_then(|(idx, node)|
                    if let #enumt::#ident(x) = node {
                        Some((*idx, x))
                    } else {
                        self.next()
                    }
                )
            }
        }
        impl<'a> std::iter::FusedIterator for #iter_ident<'a>{}
    }
    .to_tokens(result);
}

pub fn make_source_enum(
    result: &mut TokenStream,
    vars: &Vec<(Ident, Type)>,
    enumt: &Ident,
    vis: &Visibility,
) {
    let enum_name = format_ident!("{}SourceEnum", enumt);
    let mut v = Vec::new();
    for (ident, ty) in vars {
        v.push(quote! {#ident(<#ty as TypedNode>::Source),});
    }
    let mut iter_arms = Vec::new();
    for (ident, ty) in vars {
        iter_arms.push(quote! { Self::#ident(x) => Box::new(
                <#ty as TypedNode>::iter_source(&x).map(|(idx, src)| (idx, #enum_name::#ident(src)))
            ),
        })
    }
    let mut mod_arms = Vec::new();
    for (ident, ty) in vars {
        mod_arms.push(quote! {
            Self::#ident(x) => {
                if let #enum_name::#ident(src) = source {
                    <#ty as TypedNode>::modify(x, src ,old_idx, new_idx)
                } else {
                    panic!("Unmatched node type and source type!")
                }
            },
        })
    }
    quote! {
        #[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
        #vis enum #enum_name{
            #(#v)*
        }

        impl NodeEnum for #enumt {
            type SourceEnum = #enum_name;
            fn iter_source(&self) -> Box<dyn Iterator<Item = (NodeIndex, Self::SourceEnum)>> {
                match self {
                    #(#iter_arms)*
                }
            }
            fn modify(&mut self, source: Self::SourceEnum, old_idx: NodeIndex, new_idx: NodeIndex) {
                match self{
                    #(#mod_arms)*
                }
            }
        }
    }
    .to_tokens(result);
}
