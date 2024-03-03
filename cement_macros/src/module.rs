use proc_macro2::{Ident, TokenStream};
use syn::parse::{self, Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit_mut::VisitMut;
use syn::{parse2, token, Block, Generics, Pat, PatType, Token, Type, Expr};

use quote::quote;

use crate::visitor::HwVisitor;

pub(crate) struct IdentEqExpr {
  ident: Ident,
  expr: Expr,
}

impl Parse for IdentEqExpr {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let ident: Ident = input.parse()?;
    let _eq: Token![=] = input.parse()?;
    let expr: Expr = input.parse()?;
    Ok(IdentEqExpr { ident, expr })
  }
}

pub(crate) struct Module {
  pub(crate) ifc_generics: Generics,
  pub(crate) ifc_type: Type,
  pub(crate) where_clause: Option<syn::WhereClause>,
  pub(crate) ifc_impl: Ident,
  pub(crate) c: Ident,
  pub(crate) module_name: Ident,
  pub(crate) args: Punctuated<PatType, token::Comma>,
  pub(crate) tcl: Option<Expr>,
  pub(crate) ext_sv: Option<Expr>,
  pub(crate) body: Block,
}

fn try_parse<T: Parse>(input: &parse::ParseStream, msg: Option<&str>) -> syn::Result<T> {
  let c: syn::Result<T> = input.parse();

  let c = match c {
    Ok(c) => c,
    Err(err) => {
      return Err(match msg {
        Some(msg) => syn::Error::new(input.span(), msg),
        None => err,
      })
    },
  };
  Ok(c)
}

fn parse_fn_arg(input: ParseStream) -> syn::Result<PatType> {
  let pat = Box::new(Pat::parse_single(input)?);
  let colon_token: Token![:] = input.parse()?;
  let ty: Type = input.parse()?;

  Ok(PatType {
    attrs: Vec::new(),
    pat,
    colon_token,
    ty: Box::new(ty),
  })
}

impl Parse for Module {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let has_generics = input.peek(Token![<]);
    let ifc_generics: Generics =
      if has_generics { input.parse()? } else { Generics::default() };

    let ifc_type: Type = try_parse(&input, Some("Expected interface type"))?;
    let c: Ident = if input.peek(token::Paren) {
      let content;
      let _paren = syn::parenthesized!(content in input);
      try_parse(&&content, Some("Expected Cmtc name"))?
    } else {
      Ident::new("c", ifc_type.span())
    };

    let where_clause =
      if input.peek(Token![where]) { Some(input.parse()?) } else { None };

    let _: Token![=>] = try_parse(&input, Some("Expected =>"))?;

    let module_name: Ident = try_parse(&input, Some("Expected module name"))?;

    let arg_list;
    let _ = syn::parenthesized!(arg_list in input);
    let ifc_impl: Ident = try_parse(&&arg_list, Some("Expected ifc_impl name"))?;

    let args = if arg_list.peek(token::Comma) {
      let _: syn::Token![,] = try_parse(&&arg_list, Some("Expected comma"))?;
      arg_list.parse_terminated(parse_fn_arg, token::Comma)?
    } else {
      Punctuated::new()
    };

    let (tcl, ext_sv) = if input.peek(token::Bracket){
      let content;
      let _bracket = syn::bracketed!(content in input);
      let mut tcl = None;
      let mut ext_sv = None;
      
      let list = Punctuated::<IdentEqExpr, Token![,]>::parse_terminated(&content)?;

      for ident_eq_expr in list {
        if ident_eq_expr.ident == "tcl" {
          tcl = Some(ident_eq_expr.expr);
        } else if ident_eq_expr.ident == "ext_sv" {
          ext_sv = Some(ident_eq_expr.expr);
        } else {
          return Err(syn::Error::new(ident_eq_expr.ident.span(), "Expected tcl or ext_sv"));
        }
      }
      (tcl, ext_sv)
    } else {
      (None, None)
    };

    let body: Block = try_parse(&input, None)?;

    Ok(Module {
      ifc_generics,
      ifc_type,
      where_clause,
      ifc_impl,
      c,
      module_name,
      args,
      tcl,
      ext_sv,
      body,
    })
  }
}

pub(crate) fn module_decl(input: TokenStream) -> TokenStream {
  let module: Result<Module, _> = parse2(input);
  let Module {
    ifc_generics,
    ifc_type,
    where_clause,
    ifc_impl,
    c,
    module_name,
    args,
    tcl: None,
    ext_sv: None,
    mut body,
  } = (match module {
    Ok(module) => module,
    Err(e) => return e.to_compile_error().into(),
  }) else {
    return syn::Error::new(
      proc_macro2::Span::call_site(),
      "Expected module declaration without [tcl=...] or [ext_sv=...]",
    )
    .to_compile_error()
    .into();
  };

  let mut hw_visitor = HwVisitor::new(c.to_owned());
  hw_visitor.visit_block_mut(&mut body);

  let (impl_generics, ..) = ifc_generics.split_for_impl();
  quote! {

      impl #impl_generics #ifc_type #where_clause {
          pub fn #module_name(self, #c: &mut Cmtc, #args) -> <Self as Interface>::ImplT {
              let #ifc_impl = #c.begin_module(self, stringify!(#module_name).to_string(), false);

              #body

              #c.end_module::<Self>(false)
          }
      }

  }
}

#[cfg(test)]
mod test {
  use super::module_decl;

  #[test]
  fn test_module_syntax() -> Result<(), syn::Error> {
    let str = "PassIfc(c) => pass_m(module, a:u32)  {  module.o.connect(module.i, c); }";
    let token_stream = syn::parse_str::<proc_macro2::TokenStream>(str)?;
    let transformed = module_decl(token_stream);
    println!("{}", transformed);
    Ok(())
  }

  #[test]
  fn test_wire_macro() -> Result<(), syn::Error> {
    let str = "PassIfc(c) => pass_m(module, a:u32)  {  
            let one = wire!(1.lit(B::<4>));
            module.o.connect(one, c);
        }";
    let token_stream = syn::parse_str::<proc_macro2::TokenStream>(str)?;
    let transformed = module_decl(token_stream);
    println!("{}", transformed);
    Ok(())
  }

  #[test]
  fn test_instance_macro() -> Result<(), syn::Error> {
    let str = "PassIfc(c) => pass_m(module, a:u32)  {  
            let pass = instance!(pass_m(PassIfc::default(), 1));
            module.o.connect(one, c);
        }";
    let token_stream = syn::parse_str::<proc_macro2::TokenStream>(str)?;
    let transformed = module_decl(token_stream);
    println!("{}", transformed);
    Ok(())
  }

  #[test]
  fn test_connect_sugar() -> Result<(), syn::Error> {
    let str = "PassIfc(c) => pass_m(module, a:u32)  {  
            let pass = instance!(pass_m(PassIfc::default(), 1));
            module.o %= module.i.x;
        }";
    let token_stream = syn::parse_str::<proc_macro2::TokenStream>(str)?;
    let transformed = module_decl(token_stream);
    println!("{}", transformed);
    Ok(())
  }
}
