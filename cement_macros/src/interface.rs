use proc_macro2::{Ident, TokenStream};
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
  parse2, parse_quote, parse_quote_spanned, DataStruct, DeriveInput, Field, LitInt,
  LitStr, Token,
};
use quote::quote;

pub(crate) fn ifc_flip_gen(ifc_input: DeriveInput) -> Result<DeriveInput, ()> {
  let DeriveInput { attrs, vis, ident, generics, data } = ifc_input;
  match data {
    syn::Data::Struct(ifc_struct) => {
      let ident_flip = Ident::new(&format!("{}Flip", ident), ident.span());

      let DataStruct { fields, .. } = ifc_struct;

      let fields = fields.into_iter().map(|f| {
        let Field { ty, .. } = f.to_owned();
        Field {
          ty: parse_quote_spanned! { ty.span() =>
              <#ty as Interface>::FlipT
          },
          ..f
        }
      });

      let where_clause = generics.where_clause.clone();
      Ok(parse_quote! {
          #(#attrs)*
          #vis struct #ident_flip #generics #where_clause {
              #(#fields,)*
          }
      })
    },
    _ => {
      ident
        .span()
        .unwrap()
        .error("only Struct is supported for Interface deriving")
        .emit();
      Err(())
    },
  }
}

pub(crate) fn ifc_with_trait_gen(ifc_input: DeriveInput) -> Result<TokenStream, ()> {
  let DeriveInput { ident, generics, data, .. } = ifc_input;
  match data {
    syn::Data::Struct(ifc_struct) => {
      let DataStruct { fields, .. } = ifc_struct;

      let flip_ident = if ident.to_string().ends_with("Flip") {
        Ident::new(&ident.to_string()[..ident.to_string().len() - 4], ident.span())
      } else {
        Ident::new((ident.to_string() + "Flip").as_str(), ident.span())
      };

      let impl_ident = Ident::new((ident.to_string() + "Impl").as_str(), ident.span());

      // let DataStruct { fields, .. } = ifc_struct;
      let name = LitStr::new(ident.to_string().as_str(), ident.span());

      let field_flip_reduced = fields
        .to_owned()
        .into_iter()
        .enumerate()
        .map(|(i, Field { ident, .. })| match ident {
          Some(ident) => {
            let name = Ident::new(ident.to_string().as_str(), ident.span());
            quote! {
                #name: self.#name.flip(),
            }
          },
          None => {
            let i = LitInt::new(format!("{}", i).as_str(), ident.span());
            quote! {
                self.#i.flip(),
            }
          },
        })
        .reduce(|a, b| quote! {#a #b})
        .unwrap();

      let field_flip_constructor = {
        if fields.to_owned().into_iter().next().unwrap().ident.is_some() {
          quote! {
              {
                  #field_flip_reduced
              }
          }
        } else {
          quote! {
              (
                  #field_flip_reduced
              )
          }
        }
      };

      let traverse_iter = fields.to_owned().into_iter().enumerate().map(|(i, field)| {
        let Field { ident, ty, .. } = field.to_owned();
        let name = match ident.to_owned() {
          Some(ident) => LitStr::new(ident.to_string().as_str(), ident.span()),
          None => LitStr::new(format!("{}", i).as_str(), field.span()),
        };
        quote! {
            v.push((#name.to_string(), <#ty as Interface>::traverse(&self.#ident)));
        }
      });

      let field_num = LitInt::new(format!("{}", fields.len()).as_str(), ident.span());

      let field_impl_with =
        fields.to_owned().into_iter().enumerate().map(|(i, Field { ident, .. })| {
          match ident {
            Some(ident) => {
              let name = Ident::new(ident.to_string().as_str(), ident.span());
              quote! {
                  #name: self.#name.impl_with(fields[#i].1.to_owned()),
              }
            },
            None => {
              let i = LitInt::new(format!("{}", i).as_str(), ident.span());
              quote! {
                  #i: self.#i.impl_with(fields[#i].1.to_owned()),
              }
            },
          }
        });

      let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

      Ok(quote! {
          impl #impl_generics Interface for #ident #ty_generics #where_clause {
              type FlipT = #flip_ident #ty_generics;
              type ImplT = #impl_ident #ty_generics;
              fn name() -> String {
                  #name.to_string()
              }
              fn flip(self) -> Self::FlipT {
                  Self::FlipT #field_flip_constructor
              }
              fn traverse(&self) -> IfcFields {
                  let mut v = Vec::new();
                  #(#traverse_iter)*
                  IfcFields::Branch(v)
              }
              fn impl_with(self, fields: IfcImplFields) -> Self::ImplT {
                  match fields {
                      IfcImplFields::Branch(fields) => {
                          assert_eq!(fields.len(), #field_num);

                          #impl_ident {
                              #(
                                  #field_impl_with
                              )*
                          }
                      },
                      _ => {
                          panic!("IfcImplFields::Branch expected")
                      },
                  }
              }
          }
      })
    },
    _ => {
      ident
        .span()
        .unwrap()
        .error("only Struct is supported for Interface deriving")
        .emit();
      Err(())
    },
  }
}

pub(crate) fn ifc_impl_gen(ifc_input: DeriveInput) -> Result<DeriveInput, ()> {
  let DeriveInput { attrs, vis, ident, generics, data } = ifc_input;
  match data {
    syn::Data::Struct(ifc_struct) => {
      let ident_impl = Ident::new((ident.to_string() + "Impl").as_str(), ident.span());
      let DataStruct { fields, .. } = ifc_struct;

      let fields = fields.into_iter().map(|f| {
        let Field { ty, .. } = f.to_owned();
        Field {
          ty: parse_quote! {
              <#ty as Interface>::ImplT
          },
          ..f
        }
      });

      let where_clause = generics.where_clause.clone();

      Ok(parse_quote! {
          #(#attrs)*
          #vis struct #ident_impl #generics #where_clause {
              #(#fields,)*
          }
      })
    },
    _ => {
      ident
        .span()
        .unwrap()
        .error("only Struct is supported for Interface deriving")
        .emit();
      Err(())
    },
  }
}

pub(crate) fn ifc_impl_with_trait_gen(ifc_impl_input: DeriveInput) -> Result<TokenStream, ()> {
  let DeriveInput { ident, generics, data, .. } = ifc_impl_input;
  match data {
    syn::Data::Struct(ifc_struct) => {
      let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

      let DataStruct { fields, .. } = ifc_struct;

      let ifc_ident =
        Ident::new(&ident.to_string()[..ident.to_string().len() - 4], ident.span());
      let flip_ifc_ident = if ifc_ident.to_string().ends_with("Flip") {
        Ident::new(
          &ifc_ident.to_string()[..ifc_ident.to_string().len() - 4],
          ifc_ident.span(),
        )
      } else {
        Ident::new((ifc_ident.to_string() + "Flip").as_str(), ifc_ident.span())
      };

      let flip_ident =
        Ident::new((flip_ifc_ident.to_string() + "Impl").as_str(), flip_ifc_ident.span());

      let field_flip_construct = {
        let field_flip =
          fields.to_owned().into_iter().enumerate().map(|(i, Field { ident, .. })| {
            match ident {
              Some(ident) => {
                let name = Ident::new(ident.to_string().as_str(), ident.span());
                quote! {
                    #name: self.#name.flip(),
                }
              },
              None => {
                let i = LitInt::new(format!("{}", i).as_str(), ident.span());
                quote! {
                    self.#i.flip(),
                }
              },
            }
          });
        if fields.to_owned().into_iter().next().unwrap().ident.is_some() {
          quote! {
              {
                  #(#field_flip)*
              }
          }
        } else {
          quote! {
              (
                  #(#field_flip)*
              )
          }
        }
      };

      let field_traverse = fields.to_owned().into_iter().enumerate().map(|(i, field)| {
        let Field { ident, ty, .. } = field.to_owned();
        let name = match ident.to_owned() {
          Some(ident) => LitStr::new(ident.to_string().as_str(), ident.span()),
          None => LitStr::new(format!("{}", i).as_str(), field.span()),
        };
        quote! {
            v.push((#name.to_string(), <#ty as InterfaceImpl>::traverse(&self.#ident)));
        }
      });

      let field_replace_with_fields =
        fields.to_owned().into_iter().enumerate().map(|(i, field)| {
          let Field { ident, .. } = field.to_owned();
          match ident.to_owned() {
            Some(ident) => {
              quote! {
                  #ident: self.#ident.replace_with_fields(fields[#i].1.to_owned()),
              }
            },
            None => {
              quote! {
                  self.#i.replace_with_fields(fields[#i].1.to_owned())
              }
            },
          }
        });

      let field_num = LitInt::new(format!("{}", fields.len()).as_str(), ident.span());

      let field_connect = fields.to_owned().into_iter().enumerate().map(|(i, field)| {
        let Field { ident, .. } = field.to_owned();
        match ident.to_owned() {
          Some(ident) => {
            quote! {
                self.#ident.connect(target.#ident, c);
            }
          },
          None => {
            quote! {
                self.#i.connect(target.#i, c);
            }
          },
        }
      });

      let ifc_method = {
        let field_ifc = fields.to_owned().into_iter().enumerate().map(|(i, field)| {
          let Field { ident, .. } = field.to_owned();
          match ident.to_owned() {
            Some(ident) => {
              quote! {
                  #ident: self.#ident.ifc(),
              }
            },
            None => {
              quote! {
                  self.#i.ifc(),
              }
            },
          }
        });
        if fields.to_owned().into_iter().next().unwrap().ident.is_some() {
          quote! {
              Self::IfcT {
                  #(
                      #field_ifc
                  )*
              }
          }
        } else {
          quote! {
              Self::IfcT (
                  #(
                      #field_ifc
                  )*
              )
          }
        }
      };

      Ok(quote! {
          impl #impl_generics InterfaceImpl for #ident #ty_generics #where_clause {
              type FlipT = #flip_ident #ty_generics;
              type IfcT = #ifc_ident #ty_generics;
              fn flip(self) -> Self::FlipT {
                  Self::FlipT #field_flip_construct
              }
              fn traverse(&self) -> IfcImplFields {
                  let mut v = Vec::new();
                  #(
                     #field_traverse;
                  )*
                  IfcImplFields::Branch(v)
              }

              fn replace_with_fields(self, fields: IfcImplFields) -> Self {
                  match fields {
                      IfcImplFields::Branch(fields) => {
                          assert_eq!(fields.len(), #field_num);

                          Self {
                              #(
                                  #field_replace_with_fields
                              )*
                          }
                      },
                      _ => {
                          panic!("IfcImplFields::Branch expected")
                      },
                  }
              }

              fn ifc(&self) -> Self::IfcT {
                  #ifc_method
              }
          }

          impl #impl_generics Connect<#flip_ident #ty_generics> for #ident #ty_generics #where_clause {
              fn connect(self, target: #flip_ident #ty_generics, c: &mut Cmtc) {
                  #(
                      #field_connect
                  )*
              }
          }

          impl #impl_generics ConnectExpr<#flip_ifc_ident #ty_generics> for #ident #ty_generics #where_clause {}
      })
    },
    _ => {
      ident
        .span()
        .unwrap()
        .error("only Struct is supported for Interface deriving")
        .emit();
      Err(())
    },
  }
}
#[derive(Default, Clone)]
pub(crate) struct Attrs {
  pub attrs: Punctuated<Ident, Token![,]>,
}

impl Parse for Attrs {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let attrs = Punctuated::<Ident, Token![,]>::parse_terminated(input)?;
    Ok(Attrs { attrs })
  }
}

pub(crate) fn interface_decl(attr: TokenStream, input: TokenStream) -> TokenStream {
  let attrs = parse2::<Attrs>(attr).unwrap_or_default().attrs;

  let interface_input: Result<DeriveInput, _> = parse2(input);
  match interface_input {
    Ok(ifc_input) => {
      let ifc_flip_input = match ifc_flip_gen(ifc_input.to_owned()) {
        Ok(ifc_flip_input) => ifc_flip_input,
        Err(_) => return TokenStream::default(),
      };

      let ifc_with_trait = match ifc_with_trait_gen(ifc_input.to_owned()) {
        Ok(ifc_with_trait) => ifc_with_trait,
        Err(_) => return TokenStream::default(),
      };

      let ifc_flip_with_trait = match ifc_with_trait_gen(ifc_flip_input.to_owned()) {
        Ok(ifc_flip_with_trait) => ifc_flip_with_trait,
        Err(_) => return TokenStream::default(),
      };

      let ifc_impl_input = match ifc_impl_gen(ifc_input.to_owned()) {
        Ok(ifc_impl_input) => ifc_impl_input,
        Err(_) => return TokenStream::default(),
      };

      let ifc_flip_impl_input = match ifc_impl_gen(ifc_flip_input.to_owned()) {
        Ok(ifc_flip_impl_input) => ifc_flip_impl_input,
        Err(_) => return TokenStream::default(),
      };

      let ifc_impl_with_trait = match ifc_impl_with_trait_gen(ifc_impl_input.to_owned()) {
        Ok(ifc_impl_with_trait) => ifc_impl_with_trait,
        Err(_) => return TokenStream::default(),
      };

      let ifc_flip_impl_with_trait =
        match ifc_impl_with_trait_gen(ifc_flip_impl_input.to_owned()) {
          Ok(ifc_flip_impl_with_trait) => ifc_flip_impl_with_trait,
          Err(_) => return TokenStream::default(),
        };

      quote! {
          #[derive(Debug, Clone, #attrs)]
          #ifc_input

          #ifc_with_trait

          #[derive(Debug, Clone, #attrs)]
          #ifc_flip_input

          #ifc_flip_with_trait

          // #[derive(Clone)]
          #ifc_impl_input

          #ifc_impl_with_trait

          // #[derive(Clone)]
          #ifc_flip_impl_input

          #ifc_flip_impl_with_trait
      }
    },
    Err(err) => err.into_compile_error(),
  }
}

#[cfg(test)]
mod tests {

  use std::str::FromStr;

  use super::*;

  #[test]
  fn test() -> syn::Result<()> {
    let str = "
        pub struct A {
            i: B<8>,
            o: (B<8>, B<8>),
        }
        ";
    let attrs = TokenStream::from_str("Default, Clone")?;
    let tokens = TokenStream::from_str(str)?;
    let derived = interface_decl(attrs, tokens);
    println!("{}", derived);

    Ok(())
  }
}
