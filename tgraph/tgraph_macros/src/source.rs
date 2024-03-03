use change_case::pascal_case;
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{parse_quote, Fields, Generics, Ident, ItemStruct, PathArguments, Type, Visibility};

#[derive(Debug)]
pub enum ConnectType {
    Direct(Ident, Ident),
    Set(Ident, Ident),
    Vec(Ident, Ident),
    Enum(Ident, Ident),
    Empty,
}

pub fn get_source(input: &ItemStruct) -> Vec<ConnectType> {
    let Fields::Named(fields) = & input.fields else {panic!("Impossible!")};
    // eprintln!("{:?}", fields.named);
    let mut result = Vec::new();
    let direct_paths = vec![
        parse_quote!(tgraph::typed_graph::NodeIndex),
        parse_quote!(typed_graph::NodeIndex),
        parse_quote!(NodeIndex),
    ];
    let mut set_paths = Vec::new();
    for dpath in &direct_paths {
        set_paths.push(parse_quote!(std::collections::HashSet<#dpath>));
        set_paths.push(parse_quote!(collections::HashSet<#dpath>));
        set_paths.push(parse_quote!(HashSet<#dpath>));
    }
    let mut vec_paths = Vec::new();
    for dpath in &direct_paths {
        vec_paths.push(parse_quote!(std::vec::Vec<#dpath>));
        vec_paths.push(parse_quote!(vec::Vec<#dpath>));
        vec_paths.push(parse_quote!(Vec<#dpath>));
    }
    for f in &fields.named {
        let ident = f.ident.clone().unwrap();
        if let Type::Path(p) = &f.ty {
            if direct_paths.contains(p) {
                result.push(ConnectType::Direct(ident.clone(), upper_camel(&ident)))
            } else if set_paths.contains(p) {
                result.push(ConnectType::Set(ident.clone(), upper_camel(&ident)))
            } else if vec_paths.contains(p) {
                result.push(ConnectType::Vec(ident.clone(), upper_camel(&ident)))
            } else if let PathArguments::AngleBracketed(a) =
                &p.path.segments.last().unwrap().arguments
            {
                let path1 = parse_quote!(tgraph::typed_graph::NIEWrap #a);
                let path2 = parse_quote!(typed_graph::NIEWrap #a);
                let path3 = parse_quote!(NIEWrap #a);
                if p.path == path1 || p.path == path2 || p.path == path3 {
                    result.push(ConnectType::Enum(ident.clone(), upper_camel(&ident)))
                }
            }
        }
    }
    if result.len() == 0 {
        result.push(ConnectType::Empty);
    }
    result
}

pub fn make_enum(
    result: &mut TokenStream,
    sources: &Vec<ConnectType>,
    name: &Ident,
    vis: &Visibility,
) -> Ident {
    let source_enum = format_ident!("{}Source", name);
    let mut vars = Vec::new();
    for s in sources {
        match &s {
            ConnectType::Direct(_, camel) => vars.push(quote! {#camel}),
            ConnectType::Set(_, camel) => vars.push(quote! {#camel}),
            ConnectType::Vec(_, camel) => vars.push(quote! {#camel(usize)}),
            ConnectType::Enum(_, camel) => vars.push(quote! {#camel}),
            ConnectType::Empty => vars.push(quote! {Empty}),
        }
    }
    quote! {
        #[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
        #vis enum #source_enum{
            #(#vars),*
        }
    }
    .to_tokens(result);

    source_enum
}

pub fn make_iter(
    result: &mut TokenStream,
    sources: &Vec<ConnectType>,
    name: &Ident,
    vis: &Visibility,
    generics: &Generics,
    source_enum: &Ident,
) {
    let iterator_ident = format_ident!("{}SourceIterator", name);
    let mut add_source_ops = Vec::new();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    for s in sources {
        match s {
            ConnectType::Direct(ident, camel) => add_source_ops.push(quote! {
                sources.push((node.#ident, #source_enum::#camel));
            }),
            ConnectType::Set(ident, camel) => add_source_ops.push(quote! {
                for i in node.#ident.iter() {
                    sources.push((*i, #source_enum::#camel));
                }
            }),
            ConnectType::Vec(ident, camel) => add_source_ops.push(quote! {
                for (idx, i) in node.#ident.iter().enumerate() {
                    sources.push((*i, #source_enum::#camel(idx)));
                }
            }),
            ConnectType::Enum(ident, camel) => add_source_ops.push(quote! {
                sources.push((tgraph::typed_graph::IndexEnum::index(&node.#ident.value), #source_enum::#camel));
            }),
            ConnectType::Empty => {}
        }
    }

    let mut modify_arms = Vec::new();
    for s in sources {
        modify_arms.push(match s {
            ConnectType::Direct(ident, camel) => quote! {
                #source_enum::#camel => self.#ident = new_idx,
            },
            ConnectType::Set(ident, camel) => quote! {
                #source_enum::#camel => {
                    self.#ident.remove(&old_idx);
                    if !new_idx.is_empty() {
                        self.#ident.insert(new_idx);
                    }
                },
            },
            ConnectType::Vec(ident, camel) => quote! {
                #source_enum::#camel(idx) => {
                    self.#ident[idx] = new_idx;
                },
            },
            ConnectType::Enum(ident, camel) => quote! {
                #source_enum::#camel => {
                    tgraph::typed_graph::IndexEnum::modify(&mut self.#ident.value, new_idx);
                }
            },
            ConnectType::Empty => quote! {
                #source_enum::Empty => {}
            },
        })
    }
    quote! {
        #vis struct #iterator_ident {
            sources: Vec<(NodeIndex, #source_enum)>,
            cur: usize
        }
        impl #impl_generics tgraph::typed_graph::SourceIterator<#name #ty_generics> for #iterator_ident #where_clause{
            type Source = #source_enum;
            fn new(node: &#name #ty_generics) -> Self{
                let mut sources = Vec::new();
                #(#add_source_ops)*
                #iterator_ident{ sources, cur: 0 }
            }
        }
        impl std::iter::Iterator for #iterator_ident {
            type Item = (NodeIndex, #source_enum);
            fn next(&mut self) -> Option<Self::Item> {
                if self.cur == self.sources.len() {
                    None
                } else {
                    let result = self.sources[self.cur].clone();
                    self.cur += 1;
                    Some(result)
                }
            }
        }
        impl #impl_generics tgraph::typed_graph::TypedNode for #name #ty_generics #where_clause {
            type Source = #source_enum;
            type Iter = #iterator_ident;
            fn iter_source(&self) -> Self::Iter {
                #iterator_ident::new(&self)
            }
            fn modify(&mut self, source: Self::Source, old_idx:NodeIndex, new_idx: NodeIndex) {
                match source{
                    #(#modify_arms)*
                }
            }
        }
    }
    .to_tokens(result);
}

fn upper_camel(ident: &Ident) -> Ident {
    format_ident!("{}", pascal_case(&ident.to_string()))
}
