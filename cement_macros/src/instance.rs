use proc_macro2::{Ident, TokenStream};
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{parenthesized, parse2, Expr, Token};
use quote::quote;

struct Instance {
  module_name: Ident,
  ifc: Expr,
  args: Punctuated<Expr, Token![,]>,
}

impl Parse for Instance {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let module_name: Ident = input.parse()?;
    let arg_list;
    let _paren = parenthesized!(arg_list in input);

    let ifc: Expr = arg_list.parse()?;
    let args = if arg_list.peek(Token![,]) {
      let _: Token![,] = arg_list.parse()?;
      arg_list.parse_terminated(Expr::parse, Comma)?
    } else {
      Punctuated::default()
    };

    Ok(Instance { module_name, ifc, args })
  }
}
pub(crate) fn instance_decl(
  tokens: TokenStream, name: TokenStream, c: TokenStream,
) -> TokenStream {
  let Instance { module_name, ifc, args } = match parse2(tokens) {
    Ok(instance) => instance,
    Err(err) => return err.to_compile_error().into(),
  };

  quote! {
      #ifc.#module_name(#c,#args).instance(#c, stringify!(#name).to_string())
  }
}

#[cfg(test)]
mod test {
  use super::instance_decl;

  #[test]
  fn test_instance_decl() -> Result<(), syn::Error> {
    let str = "pass_m(PassIfc::default(), 1)";
    let tokens = syn::parse_str::<proc_macro2::TokenStream>(str)?;
    let transformed = instance_decl(tokens, quote! {pass}, quote! {c});
    println!("{}", transformed);
    Ok(())
  }
}
