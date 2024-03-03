use proc_macro::TokenStream;
use proc_macro2;
use proc_macro_error::*;

use quote::{format_ident, quote, ToTokens};
use syn::{parse_macro_input, Fields, ItemEnum, ItemStruct};

mod node_enum;
use node_enum::*;

mod source;
use source::*;

mod index_enum;
use index_enum::*;

#[proc_macro_derive(NodeEnum)]
#[proc_macro_error]
pub fn node_enum(the_enum: TokenStream) -> TokenStream {
    let the_enum: ItemEnum = parse_macro_input!(the_enum);
    let enumt = the_enum.ident.clone();
    let vis = the_enum.vis.clone();

    let mut vars = Vec::new();
    for var in &the_enum.variants {
        let ident = var.ident.clone();
        if let Fields::Unnamed(f) = &var.fields {
            if f.unnamed.len() != 1 {
                emit_error! {f,
                    "variants in node_enum should have only one unnamed field"
                };
            } else {
                vars.push((ident, f.unnamed.first().unwrap().ty.clone()));
            }
        } else {
            emit_error!(
                var,
                "variants in node_enum should have a node type as unnamed field"
            );
        }
    }

    let mut result = proc_macro2::TokenStream::new();
    // the_enum.to_tokens(&mut stream);
    let trait_ident = format_ident!("TGGenTrait{}", enumt);
    quote! {
        #vis trait #trait_ident<'a, IterT> {
            fn iter_by_type(graph: &'a tgraph::typed_graph::Graph<#enumt>) -> IterT;
            fn get_by_type<'b>(graph: &'b tgraph::typed_graph::Graph<#enumt>, idx: tgraph::typed_graph::NodeIndex) -> Option<&'b Self>;
        }
    }
    .to_tokens(&mut result);

    for (ident, ty) in &vars {
        make_iterator_impl(&mut result, &enumt, &ident, &ty, &trait_ident, &vis);
    }
    make_source_enum(&mut result, &vars, &enumt, &vis);

    result.into()
}

#[proc_macro_derive(TypedNode)]
#[proc_macro_error]
pub fn typed_node(input: TokenStream) -> TokenStream {
    let input: ItemStruct = parse_macro_input!(input);
    let name = input.ident.clone();
    let vis = input.vis.clone();
    let generics = input.generics.clone();
    let sources = get_source(&input);
    let mut result = proc_macro2::TokenStream::new();

    let source_enum = make_enum(&mut result, &sources, &name, &vis);
    make_iter(&mut result, &sources, &name, &vis, &generics, &source_enum);

    result.into()
}

#[proc_macro_derive(IndexEnum)]
#[proc_macro_error]
pub fn node_index_enum(input: TokenStream) -> TokenStream {
    let input: ItemEnum = parse_macro_input!(input);
    let name = input.ident.clone();
    let vis = input.vis.clone();

    let mut vars = Vec::new();
    for var in &input.variants {
        let ident = var.ident.clone();
        if let Fields::Unnamed(f) = &var.fields {
            if f.unnamed.len() != 1 {
                emit_error! {f,
                    "variants in index_enum should have only one unnamed field"
                };
            } else {
                vars.push(ident);
            }
        } else {
            emit_error!(
                var,
                "variants in index_enum should have a node type as unnamed field"
            );
        }
    }

    let mut result = proc_macro2::TokenStream::new();
    make_index_enum_trait(&mut result, &vars, &name, &vis);

    result.into()
}
