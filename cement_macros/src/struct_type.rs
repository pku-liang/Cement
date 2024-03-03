use itertools::multiunzip;
use proc_macro2::{Ident, TokenStream};
use syn::punctuated::Punctuated;
use syn::token::Plus;
use syn::{
  parse2, DataStruct, DeriveInput, Field, GenericParam, Token, TypeParam, TypeParamBound,
};
use quote::quote;

pub(crate) fn struct_decl(input: TokenStream) -> TokenStream {
  let derive_input: Result<DeriveInput, _> = parse2(input);

  let DeriveInput { ident, generics, data, .. } = match derive_input {
    Ok(derive_input) => derive_input,
    Err(err) => return err.into_compile_error(),
  };

  let DataStruct { fields, .. } = match data {
    syn::Data::Struct(strt) => strt,
    _ => {
      ident.span().unwrap().error("only struct is available");
      return quote!();
    },
  };

  let generics_def = generics.to_owned();
  let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

  let field_ident = fields.to_owned().into_iter().enumerate().map(|(i, field)| {
    let Field { ident, .. } = field;
    match ident {
      Some(ident) => quote! {#ident},
      None => quote! {#i},
    }
  });

  let field_ir_type = fields.to_owned().into_iter().enumerate().map(|(i, field)| {
    let Field { ident, ty, .. } = field;
    let ident = match ident {
      Some(ident) => quote! {#ident},
      None => quote! {#i}
    };
    quote! { (stringify!(#ident).to_string(), Box::new(<#ty as DataTypeTrait>::ir_type(&self.#ident))) }
  });

  let field_ty = fields.to_owned().into_iter().map(|field| {
    let Field { ty, .. } = field;
    quote! {#ty}
  });

  let field_generics = fields
    .to_owned()
    .into_iter()
    .enumerate()
    .map(|(i, f)| {
      let ty = f.ty;
      let gident = Ident::new(&format!("T{}", i), proc_macro2::Span::call_site());
      quote! { #gident : ToExpr<#ty> }
    })
    .reduce(|acc, x| quote! {#acc, #x})
    .unwrap();

  let field_args = fields
    .to_owned()
    .into_iter()
    .enumerate()
    .map(|(i, f)| {
      let ident = f.ident.unwrap();
      let gident = Ident::new(&format!("T{}", i), proc_macro2::Span::call_site());
      quote! { #ident : #gident }
    })
    .reduce(|acc, x| quote! {#acc, #x})
    .unwrap();

  let field_ast = fields
    .to_owned()
    .into_iter()
    .map(|f| {
      let ident = f.ident.unwrap();
      quote! { #ident.expr().ast }
    })
    .reduce(|acc, x| quote! {#acc, #x})
    .unwrap();

  let extract_trait_ident = field_ident.to_owned().map(|field_ident| {
    let gident = Ident::new(
      &format!("{}Extract{}Trait", ident.to_string(), field_ident.to_string()),
      proc_macro2::Span::call_site(),
    );
    quote! { #gident }
  });

  let field_ident_extract = field_ident.to_owned();
  let field_ty_extract = field_ty.to_owned();

  let inject_trait_ident = field_ident.to_owned().map(|field_ident| {
    let gident = Ident::new(
      &format!("{}Inject{}Trait", ident.to_string(), field_ident.to_string()),
      proc_macro2::Span::call_site(),
    );
    quote! { #gident }
  });

  let with_field_ident = field_ident.to_owned().map(|field_ident| {
    let with_ident = Ident::new(
      &format!("with_{}", field_ident.to_string()),
      proc_macro2::Span::call_site(),
    );
    quote! { #with_ident }
  });

  let field_ident_inject = field_ident.clone();

  let inject_generics = fields.to_owned().into_iter().map(|f| {
    let mut generics = generics_def.clone();
    let mut bounds = Punctuated::<TypeParamBound, Plus>::new();
    let ty = f.ty.clone();
    bounds.push(TypeParamBound::Verbatim(quote! { ToExpr<#ty> }));

    generics.params.push(GenericParam::Type(TypeParam {
      attrs: Vec::new(),
      ident: Ident::new("RHS", proc_macro2::Span::call_site()),
      colon_token: Some(Token![:]([proc_macro2::Span::call_site()])),
      bounds,
      eq_token: None,
      default: None,
    }));
    generics
  });

  let (inject_impl_generics, inject_ty_generics, inject_where_clause): (
    Vec<_>,
    Vec<_>,
    Vec<_>,
  ) = multiunzip(inject_generics.clone().map(|x| {
    let (impl_generics, ty_generics, where_clause) = x.split_for_impl();
    (quote! { #impl_generics }, quote! { #ty_generics }, quote! { #where_clause })
  }));

  let explode_trait_ident = Ident::new(
    &format!("{}ExplodeTrait", ident.to_string()),
    proc_macro2::Span::call_site(),
  );

  let tuple_ifc_ty = fields
    .to_owned()
    .into_iter()
    .map(|f| {
      let ty = &f.ty;
      quote! { #ty}
    })
    .reduce(|acc, x| quote! { #acc, #x })
    .unwrap();

  let expr_tuple_ty = quote! {Expr<(#tuple_ifc_ty)>};

  let tuple_field_ifc = field_ident
    .to_owned()
    .map(|field_ident| {
      quote! { self_expr.ifc.#field_ident.to_owned() }
    })
    .reduce(|acc, x| quote! { #acc, #x })
    .unwrap();

  let tuple_ifc = quote! { (#tuple_field_ifc) };
  quote! {
      impl #impl_generics DataTypeTrait for #ident #ty_generics #where_clause {
          fn width(&self) -> usize {
              let mut width = 0;
              #(
                  width += self.#field_ident.width();
              )*
              width
          }

          fn ir_type(&self) -> irony_cmt::DataTypeEnum {
              irony_cmt::DataTypeEnum::Struct(StructType (
                  vec![
                      #(#field_ir_type,)*
                  ]
              ))
          }
      }

      impl #impl_generics #ident #ty_generics #where_clause {
          fn struct_create<#field_generics>(self, #field_args) -> Expr<Self> {
              let DataTypeEnum::Struct(sig) = self.ir_type() else {panic!("struct_type must have Struct data type")};
              Expr {
                  ifc: self.to_owned(),
                  ast: ExprAst::Branch(ExprNode::StructCreate(sig), vec![#field_ast], None, self.traverse())
              }
          }
      }

      #(
          pub trait #extract_trait_ident #generics_def:  ToExpr<#ident #ty_generics > + Sized {
              fn #field_ident_extract(self) -> Expr<#field_ty_extract> {
                  let self_expr = self.expr();
                  let field_ifc = self_expr.to_owned().ifc.#field_ident_extract;
                  Expr {
                      ifc: field_ifc.to_owned(),
                      ast: ExprAst::Branch(ExprNode::StructExtract(stringify!(#field_ident_extract).to_string()), vec![self_expr.ast], None, field_ifc.traverse()),
                  }
              }
          }
          impl #impl_generics #extract_trait_ident #ty_generics for Expr<#ident #ty_generics> #where_clause {}
          impl #impl_generics #extract_trait_ident #ty_generics for I<#ident #ty_generics> #where_clause {}
      )*

      #(
          pub trait #inject_trait_ident #inject_generics: ToExpr<#ident #ty_generics> + Sized {
              fn #with_field_ident(self, x: RHS) -> Expr<#ident #ty_generics> {
                  let self_expr = self.expr();
                  Expr {
                      ifc: self_expr.ifc.to_owned(),
                      ast: ExprAst::Branch(ExprNode::StructInject(stringify!(#field_ident_inject).to_string()), vec![self_expr.ast, x.expr().ast], None, self_expr.ifc.traverse())
                  }
              }
          }
          impl #inject_impl_generics #inject_trait_ident #inject_ty_generics for Expr<#ident #ty_generics> #inject_where_clause {}
          impl #inject_impl_generics #inject_trait_ident #inject_ty_generics for I<#ident #ty_generics> #inject_where_clause {}
      )*


      pub trait #explode_trait_ident #generics_def : ToExpr<#ident #ty_generics> + Sized {
          fn explode(self) -> #expr_tuple_ty {
              let self_expr = self.expr();
              let tuple_ifc = #tuple_ifc;
              Expr {
                  ifc: tuple_ifc.to_owned(),
                  ast: ExprAst::Branch(ExprNode::StructExplode, vec![self_expr.ast], None, tuple_ifc.traverse())
              }
          }
      }

      impl #impl_generics #explode_trait_ident #ty_generics for Expr<#ident #ty_generics> #where_clause {}
      impl #impl_generics #explode_trait_ident #ty_generics for I<#ident #ty_generics> #where_clause {}


  }
}

#[cfg(test)]
mod test {
  use std::str::FromStr;

  use proc_macro2::{LexError, TokenStream};

  use crate::struct_type::struct_decl;

  #[test]
  fn test_struct_decl() -> Result<(), LexError> {
    let str = "struct A { x: B<4>, y: B<4> }";
    let token_stream = TokenStream::from_str(str)?;

    println!("{}", struct_decl(token_stream));

    Ok(())
  }
}
