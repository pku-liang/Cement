use proc_macro2::TokenStream;
use syn::visit_mut::VisitMut;
use syn::{parse2, ItemFn};
use quote::quote;

use crate::visitor::HwVisitor;

pub(crate) fn cmtfn_decl(attrs: TokenStream, input: TokenStream) -> TokenStream {
  let attrs = if attrs.is_empty() {
    quote! {c}
  } else {
    attrs
  };

  let input_fn = parse2::<syn::ItemFn>(input).unwrap();

  let ItemFn { mut block, .. } = input_fn.to_owned();

  let mut hw_visitor = HwVisitor { c: attrs };
  hw_visitor.visit_block_mut(&mut block);

  let new_fn = ItemFn { block, ..input_fn };

  quote! {
    #new_fn
  }
}
