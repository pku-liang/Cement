#![feature(proc_macro_diagnostic)]

use proc_macro::TokenStream;
use proc_macro2::Ident;
use syn::parse::Parse;
use syn::{LitInt, Token};

#[macro_use]
extern crate proc_macro_error;

mod signal;
mod interface;
mod event;
mod instance;
mod module;
mod module_ext;
mod struct_type;
mod visitor;
mod cmtfn;

#[proc_macro]
pub fn echo(input: TokenStream) -> TokenStream {
  input
}

#[proc_macro_error]
#[proc_macro_attribute]
pub fn signal(attr: TokenStream, input: TokenStream) -> TokenStream {
  signal::signal_decl(attr.into(), input.into()).into()
}

#[proc_macro_attribute]
pub fn interface(attr: TokenStream, input: TokenStream) -> TokenStream {
  interface::interface_decl(attr.into(), input.into()).into()
}

#[proc_macro_derive(Struct)]
pub fn derive_struct_type(input: TokenStream) -> TokenStream {
  struct_type::struct_decl(input.into()).into()
}

#[proc_macro]
pub fn module(input: TokenStream) -> TokenStream {
  module::module_decl(input.into()).into()
}

#[proc_macro]
pub fn module_ext(input: TokenStream) -> TokenStream {
  module_ext::module_ext_decl(input.into()).into()
}

#[proc_macro_attribute]
pub fn cmt_fn(attr: TokenStream, input: TokenStream) -> TokenStream {
  cmtfn::cmtfn_decl(attr.into(), input.into()).into()
}

#[proc_macro_error]
#[proc_macro]
pub fn event(input: TokenStream) -> TokenStream { event::event(input.into()).into() }

#[proc_macro_error]
#[proc_macro]
pub fn guard(input: TokenStream) -> TokenStream { event::guard(input.into()).into() }

#[proc_macro_error]
#[proc_macro]
pub fn def_const_1d(input: TokenStream) -> TokenStream {
  let n: LitInt = syn::parse(input).unwrap();
  let n = n.base10_parse::<i32>().unwrap();

  let defs = (1..=n).map(|i| {
    let defed = Ident::new(&format!("B{}", i), proc_macro2::Span::call_site());
    let num = LitInt::new(&format!("{}", i), proc_macro2::Span::call_site());
    quote! {
        pub const #defed: B<#num> = B::<#num>;
    }
  });

  quote! {
      #(#defs)*
  }
  .into()
}

struct I32x2 {
  i: i32,
  j: i32,
}

impl Parse for I32x2 {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let i: LitInt = input.parse()?;
    input.parse::<Token![,]>()?;
    let j: LitInt = input.parse()?;
    Ok(I32x2 {
      i: i.base10_parse::<i32>().unwrap(),
      j: j.base10_parse::<i32>().unwrap(),
    })
  }
}

#[proc_macro_error]
#[proc_macro]
pub fn def_const_2d(input: TokenStream) -> TokenStream {
  let I32x2 { i, j } = syn::parse(input).unwrap();
  let defs = (1..=i).flat_map(|i| {
    (1..=j).map(move |j| {
      let defed = Ident::new(&format!("B{}x{}", j, i), proc_macro2::Span::call_site());
      let num_i = LitInt::new(&format!("{}", i), proc_macro2::Span::call_site());
      let num_j = LitInt::new(&format!("{}", j), proc_macro2::Span::call_site());
      quote! {
          pub const #defed: Arr<#num_i, U<#num_j>> = Arr(B::<#num_j>);
      }
    })
  });
  quote! {
      #(#defs)*
  }
  .into()
}
