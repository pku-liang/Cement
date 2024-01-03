use proc_macro2::{Ident, TokenStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Plus;
use syn::{parse2, parse_quote_spanned, DataStruct, DeriveInput, Field, TypeParamBound, GenericParam, TypeParam, Token};

use crate::interface::*;

fn signal_lit_gen(signal_input: DeriveInput) -> Result<TokenStream, ()> {
  let DeriveInput { attrs, vis, ident, generics, data } = signal_input;
  match data {
    syn::Data::Struct(signal_struct) => {
      let ident_lit = Ident::new(&format!("{}Lit", ident), ident.span());
      let DataStruct { fields, .. } = signal_struct;

      let markers = generics.params.to_owned().into_iter().enumerate().map(|(i, ty_param)| {
        if let GenericParam::Type(TypeParam { ident: ty_ident, ..}) = ty_param.to_owned() {
          let field_ident = Ident::new(&format!("_marker{}", i), ty_param.span());
          Some(quote! { pub #field_ident: PhantomData<#ty_ident> })
        } else {
          None
        }
      }).filter_map(|x| x);

      let new_fields = fields.to_owned().into_iter().enumerate().map(|(i, f)| {
        let ty_ident = Ident::new(&format!("T{}", i), f.span());
        let ty_ident_as_type = parse_quote_spanned!(f.span()=> #ty_ident);

        Field { ty: ty_ident_as_type, ..f }
      });

      let mut new_generics = generics.to_owned();
      for (i,field) in fields.to_owned().into_iter().enumerate() {
        let ty_ident = Ident::new(&format!("T{}", i), field.span());
        let Field {ty, ..} = field;
        let mut bounds = Punctuated::<TypeParamBound, Plus>::new();
        bounds.push(TypeParamBound::Verbatim(quote! {IntoValue<#ty>}));
        new_generics.params.push(GenericParam::Type(TypeParam {
          attrs: vec![],
          ident: ty_ident,
          colon_token: Some(Token![:]([proc_macro2::Span::call_site()])),
          bounds,
          eq_token: None,
          default: None,
        }));
      } 
    
      let fields = fields.to_owned().into_iter().enumerate().map(|(i, field)| {
        let Field { ident, .. } = field;
        match ident {
          Some(ident) => quote! { #ident },
          None => quote! { #i },
        }
      });

      let ifc_ty_generics = generics.split_for_impl().1;
      let (impl_generics, ty_generics, where_clause) = new_generics.split_for_impl();
      Ok(quote! {
        #(#attrs)*
        #vis struct #ident_lit #new_generics #where_clause {
          #(#new_fields,)*
          #(#markers,)*
        }
        
        impl #impl_generics IntoValue<#ident #ifc_ty_generics> for #ident_lit #ty_generics #where_clause {
          fn into_value(self, signal: #ident #ifc_ty_generics) -> SignalValue {
            let mut v = vec![];
            #(
              v.extend(self.#fields.into_value(signal.#fields).v_data);
            )*
            SignalValue {
              v_data: v,
              name: format!("{}", stringify!(#ident)),
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

fn signal_with_trait_gen(signal_input: DeriveInput) -> Result<TokenStream, ()> {
  let DeriveInput { ident, generics, data, .. } = signal_input;
  match data {
    syn::Data::Struct(signal_struct) => {
      let DataStruct { fields, .. } = signal_struct;

      let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

      let fields = fields.to_owned().into_iter().enumerate().map(|(i, field)| {
        let Field { ident, .. } = field;
        match ident {
          Some(ident) => quote! { #ident },
          None => quote! { #i },
        }
      });
      let fields_clone = fields.clone();

      Ok(quote! {
        impl #impl_generics SignalTrait for #ident #ty_generics #where_clause {
          fn total_width(&self) -> usize {
            let mut total_width = 0;
            #(
              total_width += self.#fields.total_width();
            )*
            total_width
          }

          fn v_ir_type(&self) -> Vec<irony_cmt::DataTypeEnum> {
            let mut v_ir_type = vec![];
            #(
              v_ir_type.extend(self.#fields_clone.v_ir_type());
            )*
            v_ir_type
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

pub(crate) fn signal_decl(attr: TokenStream, input: TokenStream) -> TokenStream {
  let attrs = parse2::<Attrs>(attr).unwrap_or_default().attrs;

  let interface_input: Result<DeriveInput, _> = parse2(input);
  match interface_input {
    Ok(ifc_input) => {
      let signal_lit = match signal_lit_gen(ifc_input.to_owned()) {
        Ok(signal_lit) => signal_lit,
        Err(_) => return TokenStream::default(),
      };
      let signal_with_trait = match signal_with_trait_gen(ifc_input.to_owned()) {
        Ok(signal_with_trait) => signal_with_trait,
        Err(_) => return TokenStream::default(),
      };

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

          #[derive(Debug, Clone)]
          #signal_lit

          #signal_with_trait

          #ifc_with_trait

          #[derive(Debug, Clone, #attrs)]
          // #[derive(Debug, Clone)]
          #ifc_flip_input

          #ifc_flip_with_trait

          #ifc_impl_input

          #ifc_impl_with_trait

          #ifc_flip_impl_input

          #ifc_flip_impl_with_trait
      }
    },
    Err(err) => err.into_compile_error(),
  }
}
