use super::*;
use itertools::Itertools;
use quote::{format_ident, quote};
use syn::{
  self, braced, parenthesized, parse::Parse, punctuated::Punctuated, token, Attribute, Expr, Ident, LitStr, Meta, Token, TypePath, Visibility
};

pub mod kw {
  syn::custom_keyword!(param);
  syn::custom_keyword!(method);
  syn::custom_keyword!(input);
  syn::custom_keyword!(output);
  syn::custom_keyword!(itfc);
}

#[derive(Debug, Clone)]
pub struct ItfcDecl {
  pub params: Punctuated<ParamDecl, Token![;]>,
  pub attrs: Vec<Attribute>,
  pub vis: Visibility,
  pub _struct_token: Token![struct],
  pub ident: Ident,
  pub _brace_token: token::Brace,
  pub fields: Punctuated<IOField, Token![,]>,
  pub _semi_token: Option<Token![;]>,
  pub methods: Punctuated<MethodDecl, Token![;]>,
}

impl Parse for ItfcDecl {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let mut params = Punctuated::new();
    let mut attrs = Vec::new();
    if input.peek(Token![#]) {
      attrs.extend(input.call(Attribute::parse_outer)?);
    }
    loop {
      let lookahead = input.lookahead1();
      if lookahead.peek(Token![struct]) || lookahead.peek(Token![pub]) {
        break;
      }
      let mut param= input.call(ParamDecl::parse_without_attrs)?;
      param.attrs = attrs.drain(..).collect(); 
      let semi_token: Token![;] = input.parse()?;
      params.push_value(param);
      params.push_punct(semi_token);
      if input.peek(Token![#]) {
        attrs.extend(input.call(Attribute::parse_outer)?);
      }
    }
    // let attrs = input.call(Attribute::parse_outer)?;
    let vis = if input.lookahead1().peek(Token![pub]) {
      input.parse()?
    } else {
      Visibility::Inherited
    };

    let content;
    Ok(ItfcDecl {
      params,
      attrs,
      vis,
      _struct_token: input.parse()?,
      ident: input.parse()?,
      _brace_token: braced!(content in input),
      fields: content.parse_terminated(IOField::parse, Token![,])?,
      _semi_token: if input.peek(Token![;])  {Some(input.parse()?)} else {None},
      methods: input.parse_terminated(MethodDecl::parse, Token![;])?,
    })
  }
}

#[derive(Debug, Clone)]
pub struct ParamDecl {
  pub attrs: Vec<Attribute>,
  pub _param_token: kw::param,
  pub ident: Ident,
}

impl Parse for ParamDecl {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    Ok(ParamDecl {
      attrs: input.call(Attribute::parse_outer)?,
      _param_token: input.parse()?,
      ident: input.parse()?,
    })
  }
}

impl ParamDecl {
  fn parse_without_attrs(input: syn::parse::ParseStream) -> syn::Result<Self> {
    Ok(ParamDecl {
      attrs: Vec::new(),
      _param_token: input.parse()?,
      ident: input.parse()?,
    })
  }
}

#[allow(unused)]
#[derive(Debug, Clone)]
pub struct NestedItfcParam {
  ident: Ident,
  _colon_token: Token![:],
  ty: FieldTy,
}

impl Parse for NestedItfcParam {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    Ok(NestedItfcParam {
      ident: input.parse()?,
      _colon_token: input.parse()?,
      ty: input.parse()?,
    })
  }
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct NestedItfcParams {
  _brace_token: token::Brace,
  params: Punctuated<NestedItfcParam, Token![,]>,
}

impl Parse for NestedItfcParams {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let content;
    Ok(NestedItfcParams {
      _brace_token: braced!(content in input),
      params: content.parse_terminated(NestedItfcParam::parse, Token![,])?,
    })
  }
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct NestedItfc {
  ty: TypePath,
  params: Option<NestedItfcParams>,
}

impl Parse for NestedItfc {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let ty = input.parse()?;
    let params = if input.peek(token::Brace) {
      Some(input.parse()?)
    } else {
      None
    };

    Ok(NestedItfc { ty, params })
  }
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub enum IOEnum {
  Input(kw::input, FieldTy),
  Output(kw::output, FieldTy),
  Itfc(kw::itfc, NestedItfc),
}

impl Parse for IOEnum {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    if input.peek(kw::input) {
      Ok(IOEnum::Input(input.parse()?, input.parse()?))
    } else if input.peek(kw::output) {
      Ok(IOEnum::Output(input.parse()?, input.parse()?))
    } else if input.peek(kw::itfc) {
      Ok(IOEnum::Itfc(input.parse()?, input.parse()?))
    } else {
      Err(input.error("Expect input/output/itfc keyword"))
    }
  }
}

#[derive(Debug, Clone)]
pub struct IOField {
  pub attrs: Vec<Attribute>,
  pub override_name: Option<LitStr>,
  pub ident: Ident,
  pub _colon_token: Token![:],
  pub ty: IOEnum,
}

impl Parse for IOField {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let mut override_name = None;
    let mut attrs = input.call(Attribute::parse_outer)?;
    let mut to_remove = Vec::new();
    for (i, attr) in attrs.iter().enumerate() {
      match &attr.meta {
        Meta::List(meta) => {
          if meta.path.is_ident("name") {
            override_name = Some(syn::parse2(meta.tokens.clone())?);
            to_remove.push(i);
          }
        },
        _ => {}
      }
    }
    for r in to_remove.into_iter().rev() {
      attrs.remove(r);
    }
    Ok(IOField {
      attrs,
      override_name,
      ident: input.parse()?,
      _colon_token: input.parse()?,
      ty: input.parse()?,
    })
  }
}

#[derive(Debug, Clone)]
pub enum FieldTy {
  Param(ParamDecl),
  Type(Expr),
}

impl Parse for FieldTy {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let lookahead = input.lookahead1();
    if lookahead.peek(kw::param) {
      Ok(FieldTy::Param(input.parse()?))
    } else {
      Ok(FieldTy::Type(input.parse()?))
    }
  }
}

#[derive(Debug, Clone)]
pub struct MethodDecl {
  pub attrs: Vec<Attribute>,
  pub _method_token: kw::method,
  pub name: Ident,
  pub _paren_token: token::Paren,
  pub inputs: Punctuated<Ident, Token![,]>,
  pub _rarrow_token: Option<Token![->]>,
  pub _paren_token2: Option<token::Paren>,
  pub outputs: Option<Punctuated<Ident, Token![,]>>,
}

impl Parse for MethodDecl {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let attrs = input.call(Attribute::parse_outer)?;
    let input_content;
    let output_content;
    let _method_token = input.parse()?;
    let name = input.parse()?;
    let _paren_token = parenthesized!(input_content in input);
    let inputs = input_content.parse_terminated(Ident::parse, Token![,])?;

    if input.peek(Token![->]) {
      Ok(MethodDecl {
        attrs,
        _method_token,
        name,
        _paren_token,
        inputs,
        _rarrow_token: Some(input.parse()?),
        _paren_token2: Some(parenthesized!(output_content in input)),
        outputs: Some(
          output_content.parse_terminated(Ident::parse, Token![,])?,
        ),
      })
    } else {
      Ok(MethodDecl {
        attrs,
        _method_token,
        name,
        _paren_token,
        inputs,
        _rarrow_token: None,
        _paren_token2: None,
        outputs: None,
      })
    }
  }
}

pub fn gen_io_struct(decl: &ItfcDecl) -> (Ident, proc_macro2::TokenStream) {
  let ident = format_ident!("{}IO", decl.ident);
  let mut fields = Vec::new();
  for f in &decl.fields {
    let name = &f.ident;
    let attrs = &f.attrs;
    if let IOEnum::Itfc(_, itfc) = &f.ty {
      let ty = &itfc.ty;
      fields.push(quote! { #(#attrs)* pub #name: #ty });
    } else {
      fields.push(quote! { #(#attrs)* pub #name: cmtrs::Var});
    }
  }
  let vis = &decl.vis;
  let attrs = &decl.attrs;

  let result = quote! {
    #(#attrs)*
    /// Cmtrs generated IO struct.
    #[doc(hidden)]
    #[allow(non_snake_case)]
    #vis struct #ident {
      #(#fields),*
    }
    #[automatically_derived]
    impl cmtrs::ItfcIO for #ident {}
  };
  (ident, result)
}

pub fn gen_params_struct(decl: &ItfcDecl) -> (Ident, proc_macro2::TokenStream) {
  let ident = format_ident!("{}Params", decl.ident);
  let mut fields = Vec::new();
  let mut param_str = Vec::new();
  for p in &decl.params {
    let name = type_param_name(&p.ident);
    let attrs = &p.attrs;
    fields.push(quote! {#(#attrs)* pub #name: cmtrs::Type});
    param_str.push(quote! {cmtrs::Type::name(&self.#name)});
  }
  for f in &decl.fields {
    let name = &f.ident;
    let attrs = &f.attrs;
    if let IOEnum::Itfc(_, itfc) = &f.ty {
      let ty = &itfc.ty;
      fields.push(quote! { #(#attrs)* pub #name: #ty });
    }
  }

  let fmt_str = decl.params.iter().map(|_| "{}").join("_");
  let id_str = if param_str.is_empty() {
    quote!("".to_string())
  } else {
    quote!(format!(#fmt_str, #(#param_str),*))
  };

  let vis = &decl.vis;
  let attrs = &decl.attrs;
  let result = quote! {
    #(#attrs)*
    /// Cmtrs generated parameter struct.
    #[doc(hidden)]
    #[allow(non_snake_case)]
    #vis struct #ident {
      #(#fields),*
    }
    #[automatically_derived]
    impl cmtrs::ItfcParams for #ident {
      fn id_str(&self) -> String {
        #id_str
      }
    }
  };
  (ident, result)
}


pub fn gen_iftc_struct(
  decl: &ItfcDecl,
  params_name: &Ident,
) -> (Ident, proc_macro2::TokenStream) {
  let ident = format_ident!("{}Itfc", &decl.ident);

  let attrs = &decl.attrs;
  let vis = &decl.vis;
  let result = quote! {
    #(#attrs)*
    /// Cmtrs generated module generator interface. Used only in module generator function with [`#[module]`](`module!`) attribute.
    #[doc(hidden)]
    #vis struct #ident {
      __module: cmtrs::gen_ir::ModuleMold,
      __params: #params_name,
    }
  };
  (ident, result)
}

pub fn impl_itfc(
  decl: &ItfcDecl,
  itfc_name: &Ident,
  io_name: &Ident,
  params_name: &Ident,
) -> proc_macro2::TokenStream {
  let instance_name = &decl.ident;

  let mut method_names = Vec::new();
  for method in &decl.methods {
    let name = &method.name;
    method_names.push(quote! {std::stringify!(#name)});
  }

  let mut io_fields = Vec::new();
  let mut make_io_fields = Vec::new();
  for field in &decl.fields {
    let name = &field.ident;
    match &field.ty {
      IOEnum::Input(_, ty) => {
        let t = match &ty {
          FieldTy::Param(param) => {
            let t_name = type_param_name(&param.ident);
            quote! {{let t=__cmt_gen.borrow().as_ref().unwrap().params().#t_name.clone();t}}
          }
          FieldTy::Type(expr) => quote! {#expr},
        };
        if let Some(s_name) = &field.override_name {
          io_fields.push(quote! {
            #name: cmtrs::input!(#s_name.to_string(), #t)
          });
        } else {
          io_fields.push(quote! {
            #name: cmtrs::input!(std::stringify!(#name).to_string(), #t)
          });
        }
      }
      IOEnum::Output(_, ty) => {
        let t = match &ty {
          FieldTy::Param(param) => {
            let t_name = type_param_name(&param.ident);
            quote! {{let t=__cmt_gen.borrow().as_ref().unwrap().params().#t_name.clone();t}}
          }
          FieldTy::Type(expr) => quote! {#expr},
        };
        if let Some(s_name) = &field.override_name {
          io_fields.push(quote! {
            #name: cmtrs::output!(#s_name.to_string(), #t)
          });
        } else {
          io_fields.push(quote! {
            #name: cmtrs::output!(std::stringify!(#name).to_string(), #t)
          });
        }
      }
      IOEnum::Itfc(_, _) => {
        io_fields.push(quote!{#name});
        make_io_fields.push(quote!{
          let #name = {
            let _name = std::stringify!(#name).to_string();
            let _nested_names = cmtrs::Instance::nested_names(&__cmt_gen.borrow().as_ref().unwrap().params().#name); 
            let _views = cmtrs::Itfc::make_views(&__cmt_gen, _name.clone(), _nested_names);
            let _instance = {
              let _itfc = __cmt_gen.borrow();
              let _module_name = _itfc.as_ref().unwrap().module().get_name().to_string();
              cmtrs::Instance::as_nested_itfc_io(&_itfc.as_ref().unwrap().params().#name, _module_name, &mut _views.into_iter())
            };
            cmtrs::Instance::make_nested_methods(&_instance, _name, __cmt_gen.clone());
            _instance
          };
        });
      }
    }
  }

  let mut add_sub_itfc = Vec::new();
  for field in &decl.fields {
    if let IOEnum::Itfc(_, _) = &field.ty {
      let name = &field.ident;
      add_sub_itfc.push(quote!{
        cmtrs::module_add_instance(&mut __module, std::stringify!(#name).to_string(), &mut __params.#name);
      })
    }
  }

  let module_name = if decl.params.len() == 0 {
    quote!(std::stringify!(#instance_name).to_string())
  } else {
    quote!(format!(
      "{}_{}",
      std::stringify!(#instance_name),
      cmtrs::ItfcParams::id_str(&__params)
    ))
  };



  quote! {
    #[automatically_derived]
    impl cmtrs::Itfc for #itfc_name {
      type IO = #io_name;
      type Params = #params_name;
      type Inst = #instance_name;

      fn as_context(mut __params: Self::Params, span: Option<MySpan>) -> Self {
        let mut __module = cmtrs::gen_ir::ModuleMold::new(#module_name, span);
        #(#add_sub_itfc)*
        Self {
          __module,
          __params,
        }
      }

      fn module_mut(&mut self) -> &mut cmtrs::gen_ir::ModuleMold { &mut self.__module }

      fn module(&self) -> &cmtrs::gen_ir::ModuleMold { &self.__module }

      fn methods(&self) -> &'static [&'static str] { &[#(#method_names),*] }

      fn params(&self) -> &Self::Params { &self.__params }

      fn io(__cmt_gen: cmtrs::ContextItfc<Self>) -> Self::IO {
        #(#make_io_fields)*
        Self::IO {
          #(#io_fields),*
        }
      }

      fn into_inner(self) -> (cmtrs::gen_ir::ModuleMold, Self::Params) {
        (self.__module, self.__params)
      }
    }
  }
}

pub fn gen_instance_struct(decl: &ItfcDecl) -> proc_macro2::TokenStream {
  let mut fields = Vec::new();
  for p in &decl.params {
    let name = type_param_name(&p.ident);
    let attrs = &p.attrs;
    fields.push(quote! {#(#attrs)* pub #name: cmtrs::Type});
  }
  for f in &decl.fields {
    let name = &f.ident;
    let attrs = &f.attrs;
    if let IOEnum::Itfc(_, itfc) = &f.ty {
      let ty = &itfc.ty;
      fields.push(quote! { #(#attrs)* pub #name: #ty });
    }
  }
  let name = &decl.ident;
  let attrs = &decl.attrs;
  let vis = &decl.vis;
  quote!{
    #(#attrs)*
    #[allow(non_snake_case)]
    #vis struct #name {
      #(#fields,)*
      #[doc(hidden)]
      __module: cmtrs::gen_ir::ModuleMold,
      #[doc(hidden)]
      __ctx_view: Box<dyn cmtrs::InstanceView>,
    }
  }
}

pub fn impl_instance(decl: &ItfcDecl, itfc_name: &Ident, params_name: &Ident) -> proc_macro2::TokenStream {
  let mut from_params= Vec::new();
  let mut clone_params= Vec::new();
  let mut nested_names = Vec::new();
  for p in &decl.params {
    let name = type_param_name(&p.ident);
    from_params.push(quote!{#name: params.#name});
    clone_params.push(quote!{#name: self.#name.clone()});
  }
  for f in &decl.fields {
    if let IOEnum::Itfc(_, _) = &f.ty {
      let name = &f.ident;
      from_params.push(quote!{#name: params.#name});
      clone_params.push(quote!{#name: cmtrs::Instance::as_nested_itfc_io(&self.#name, format!("{}_{}", name, std::stringify!(#name)), views)});
      nested_names
        .push(quote! {result.push(std::stringify!(#name).to_string());});
      nested_names
        .push(quote! {result.extend(cmtrs::Instance::nested_names(&self.#name));});
    }
  }

  let mut nested_as_instance = Vec::new();
  for field in &decl.fields {
    if let IOEnum::Itfc(_, _) = &field.ty {
      let name = &field.ident;
      nested_as_instance
        .push(quote! {cmtrs::Instance::as_instance(&mut self.#name, views);});
    }
  }

  let mut nested_io_fields = Vec::new();
  let mut nested_nested_itfc = Vec::new();
  let mut in_names = Vec::new();
  let mut out_names = Vec::new();
  for field in &decl.fields {
    let name = &field.ident;

    match &field.ty {
      IOEnum::Input(_, ty) => {
        let t = match &ty {
          FieldTy::Param(param) => {
            let t_name = type_param_name(&param.ident);
            quote! {self.#t_name.clone()}
          }
          FieldTy::Type(expr) => quote! {#expr},
        };
        if let Some(s_name) = &field.override_name {
          nested_io_fields.push(quote!{
            let #name = cmtrs::input!(format!("{}_{}", name, #s_name), #t);
          });
          in_names.push(quote!(#s_name.to_string()));
        } else {
          nested_io_fields.push(quote!{
            let #name = cmtrs::input!(format!("{}_{}", name, std::stringify!(#name)), #t);
          });
          in_names.push(quote!(std::stringify!(#name).to_string()));
        }
      }
      IOEnum::Output(_, ty) => {
        let t = match &ty {
          FieldTy::Param(param) => {
            let t_name = type_param_name(&param.ident);
            quote! {self.#t_name.clone()}
          }
          FieldTy::Type(expr) => quote! {#expr},
        };
        if let Some(s_name) = &field.override_name {
          nested_io_fields.push(quote!{
            let #name = cmtrs::output!(format!("{}_{}", name, #s_name), #t);
          });
          out_names.push(quote!(#s_name.to_string()));
        } else {
          nested_io_fields.push(quote!{
            let #name = cmtrs::output!(format!("{}_{}", name, std::stringify!(#name)), #t);
          });
          out_names.push(quote!(std::stringify!(#name).to_string()));
        }
      }
      IOEnum::Itfc(_, _) => { 
        nested_nested_itfc.push(quote!{
          cmtrs::Instance::make_nested_methods(&self.#name, format!("{}_{}", name, std::stringify!(#name)), __cmt_gen.clone());
        })
      }
    }
  }

  let mut nested_methods = Vec::new();
  let mut method_names = Vec::new();
  for method in &decl.methods {
    let name = &method.name;
    let mut inputs = Vec::new();
    method_names.push(quote!{std::stringify!(#name).to_string()});
    for i in &method.inputs {
      inputs.push(quote! {#i});
    }

    let outputs = if let Some(outputs) = &method.outputs {
      let mut v = Vec::new();
      for o in outputs {
        v.push(quote!{#o});
      }
      quote!(-> (#(#v),*))
    } else {
      quote!()
    };
    nested_methods.push(
      quote!{
        cmtrs::named_method!(format!("{}_{}", name, std::stringify!(#name)); (#(#inputs),*) #outputs { self.#name(#(#inputs),*) });
      }
    );
  }

  let name = &decl.ident;
  quote!{
    #[automatically_derived]
    impl cmtrs::Instance for #name {
      type Params = #params_name;

      fn type_name() -> &'static str { &std::stringify!(#name) }

      fn from_itfc(__module: cmtrs::gen_ir::ModuleMold, params: Self::Params) -> Self {
        Self {
          __module,
          __ctx_view: Box::new(cmtrs::InstanceViewStruct::<#itfc_name>::new()),
          #(#from_params),* 
        }
      }

      fn as_instance(&mut self, views: &mut cmtrs::InstanceViewIter) {
        self.__ctx_view = views.next().unwrap();
        #(#nested_as_instance)*
      }

      fn as_nested_itfc_io(&self, name: String, views: &mut cmtrs::InstanceViewIter) -> Self {
        let instance = Self {
          __module: cmtrs::gen_ir::ModuleMold::clone_with_new_name(&self.__module, name.clone()),
          __ctx_view: views.next().unwrap(),
          #(#clone_params),*
        };
        instance
      }

      fn make_nested_methods(&self, name: String, __cmt_gen: cmtrs::ContextItfc<impl Itfc>) {
        #(#nested_io_fields)*
        #(#nested_methods)*
        cmtrs::Instance::add_generated_io_methods(
          self, 
          name.clone(),
          std::collections::HashSet::from_iter([#(#in_names),*]),
          std::collections::HashSet::from_iter([#(#out_names),*]),
          std::collections::HashSet::from_iter([#(#method_names),*]),
          __cmt_gen.clone()
        );
        #(#nested_nested_itfc)*
      }

      fn module_mut(&mut self) -> &mut cmtrs::gen_ir::ModuleMold { &mut self.__module }

      fn module(&self) -> &cmtrs::gen_ir::ModuleMold { &self.__module }

      fn nested_names(&self) -> Vec<String> {
        let mut result = Vec::new();
        #(#nested_names)*
        result
      }

      fn invoke(&self, name: &str, args: &[&cmtrs::Var], num_res: usize, span: Option<MySpan>) -> Vars {
        Vars(self.__ctx_view.invoke(name.to_string(), args.iter().map(|x|x.ast()).collect(), num_res, span))
      }
    }

    impl cmtrs::ToIOMacro for #name {
      type Output = Self;

      fn to_io_macro(self) -> Self::Output { self }
    }
  }
}


pub fn impl_methods(
  decl: &ItfcDecl,
  // params_name: &Ident,
) -> proc_macro2::TokenStream {
  let itfc_name = &decl.ident;

  // let mut params_def = Vec::new();
  // let mut params = Vec::new();
  // for param in &decl.params {
  //   let t_name = type_param_name(&param.ident);
  //   params_def.push(quote! {#t_name: cmtrs::Type});
  //   params.push(quote! {#t_name});
  // }

  let mut methods = Vec::new();
  for method in &decl.methods {
    let name = &method.name;
    let attrs = &method.attrs;
    let mut inputs_def = Vec::new();
    let mut inputs_ast = Vec::new();
    for input in &method.inputs {
      inputs_def.push(quote! {#input: impl cmtrs::CmtAST});
      inputs_ast.push(quote! {#input.ast()});
    }
    if let Some(out) = &method.outputs {
      if out.len() == 1 {
        methods.push(quote! {
          #(#attrs)*
          #[track_caller]
          pub fn #name(&self, #(#inputs_def),*) -> cmtrs::Var {
            let span = extract_span_from_location(Location::caller());
            self.__ctx_view.invoke(
              std::stringify!(#name).to_string(),
              vec![#(#inputs_ast),*],
              1,
              Some(span),
            ).into_iter().next().unwrap()
        }});
      } else {
        let out_vars: Vec<_> =
          (0..out.len()).map(|_| quote! {cmtrs::Var}).collect();
        let out_nexts: Vec<_> = (0..out.len())
          .map(|_| quote! {ret.next().unwrap()})
          .collect();
        let num_res = out.len();
        methods.push(quote! {
          #(#attrs)*
          #[track_caller]
          pub fn #name(&self, #(#inputs_def),*) -> (#(#out_vars),*) {
            let span = extract_span_from_location(Location::caller());
            let mut ret = self.__ctx_view.invoke(
              std::stringify!(#name).to_string(),
              vec![#(#inputs_ast),*],
              #num_res,
              Some(span),
            ).into_iter();
            (#(#out_nexts),*)
        }});
      }
    } else {
      methods.push(quote! {
        #(#attrs)*
        #[track_caller]
        pub fn #name(&self, #(#inputs_def),*) {
          let span = extract_span_from_location(Location::caller());
          self.__ctx_view.invoke(
            std::stringify!(#name).to_string(),
            vec![#(#inputs_ast),*],
            0,
            Some(span),
          );
        }
      });
    }
  }

  quote! {
    impl #itfc_name {
      #(#methods)*
    }
  }
}
