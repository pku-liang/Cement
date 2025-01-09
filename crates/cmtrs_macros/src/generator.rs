use super::*;
use proc_macro2::TokenStream;
use quote::quote_spanned;
// use quote::format_ident;
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit_mut::{visit_macro_mut, visit_stmt_mut, VisitMut};
use syn::{
  bracketed, parenthesized, parse_quote, parse_quote_spanned, token, Block,
  Expr, ExprBlock, Ident, LitBool, Local, LocalInit, Macro, Path, ReturnType,
  Stmt, Token, Type,
};

pub struct GenInfo {
  iftc_ty: Type,
}

pub fn extract_info(gen_fn: &ItemFn) -> GenInfo {
  let iftc_ty = if let ReturnType::Type(_, ty) = &gen_fn.sig.output {
    *ty.clone()
  } else {
    abort!(
      gen_fn.sig,
      "Module generators should return a module interface"
    );
  };
  GenInfo { iftc_ty }
}

pub struct MacroFill {
  name: Option<Ident>,
}

impl MacroFill {
  pub fn new() -> Self {
    Self { name: None }
  }
}

impl VisitMut for MacroFill {
  fn visit_stmt_mut(&mut self, stmt: &mut Stmt) {
    let last_name = self.name.clone();
    if let Stmt::Local(local) = stmt {
      self.name = get_name_from_local(local);
    } else if macro_without_result(stmt, "if_") {
      let (tokens, path, semi) = clone_macro_without_result(stmt);
      *stmt = Stmt::Expr(
        parse_quote!(cmtrs::Itfc::statement(&__cmt_gen, #path!{#tokens})),
        semi,
      );
    }
    visit_stmt_mut(self, stmt);
    self.name = last_name;
  }

  fn visit_macro_mut(&mut self, macro_: &mut Macro) {
    if macro_.path.is_ident("instance") || macro_.path.is_ident("always") {
      if let Some(name) = &self.name {
        let tokens = &macro_.tokens;
        let name_tokens = quote_spanned! {macro_.span() => std::stringify!(#name).to_string(); };
        macro_.tokens = quote! { #name_tokens #tokens }.into();
      } else {
        abort!(macro_, "cannot infer the name for the macro that is not inside of a let expression");
      }
    } else if macro_.path.is_ident("method")
      || macro_.path.is_ident("ext_method")
    {
      if let Some(name) = &self.name {
        let tokens = &macro_.tokens;
        let name_tokens = quote_spanned! {macro_.span() => std::stringify!(#name).to_string(); };
        macro_.tokens = quote! { #name_tokens #tokens }.into();
      } else {
        abort!(macro_, "cannot infer the name for the macro that is not inside of a let expression");
      }
    } else if macro_.path.is_ident("var") {
      if let Some(name) = &self.name {
        let tokens = &macro_.tokens;
        let name_tokens = quote_spanned! {macro_.span() => std::stringify!(#name).to_string(); };
        macro_.tokens = quote! { #name_tokens #tokens }.into();
      }
    }
    visit_macro_mut(self, macro_);
  }
}

pub fn macro_fill(stmt: &mut Stmt) {
  MacroFill::new().visit_stmt_mut(stmt);
}

struct IOMacro {
  fields: Punctuated<ParamValue, Token![,]>,
}
impl Parse for IOMacro {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    Ok(IOMacro {
      fields: input.parse_terminated(ParamValue::parse, Token![,])?,
    })
  }
}

#[derive(Debug, Clone)]
pub struct ParamValue {
  pub ident: Ident,
  pub _colon: Token![:],
  pub value: Expr,
}
impl Parse for ParamValue {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    Ok(ParamValue {
      ident: input.parse()?,
      _colon: input.parse()?,
      value: input.parse()?,
    })
  }
}

pub fn io_macro(info: &GenInfo, gen_fn: &mut ItemFn) -> Result<(), syn::Error> {
  let mut pos = gen_fn.block.stmts.len();
  for (i, s) in gen_fn.block.stmts.iter().enumerate() {
    if macro_with_result(s, "io") || macro_without_result(s, "io") {
      pos = i;
      break;
    }
  }

  if pos == gen_fn.block.stmts.len() {
    abort!(gen_fn, "module generator requires a io! macro");
  }

  let s = gen_fn.block.stmts.remove(pos);
  let mut let_local = None;
  let mut init_eq = None;
  let mut init_diverge = None;
  let (io_macro, io_path): (IOMacro, Path) = if let Stmt::Local(local) = s {
    // let xxx = io!{}
    let_local = Some(Local {
      attrs: local.attrs,
      let_token: local.let_token,
      pat: local.pat,
      init: None,
      semi_token: local.semi_token,
    });
    let init = local.init.unwrap();
    init_eq = Some(init.eq_token);
    init_diverge = Some(init.diverge);
    if let Expr::Macro(macro_) = *init.expr {
      (syn::parse2(macro_.mac.tokens)?, macro_.mac.path)
    } else {
      panic!()
    }
  } else if let Stmt::Macro(macro_) = s {
    (syn::parse2(macro_.mac.tokens)?, macro_.mac.path)
  } else if let Stmt::Expr(Expr::Macro(macro_), _) = s {
    (syn::parse2(macro_.mac.tokens)?, macro_.mac.path)
  } else {
    panic!()
  };

  let io_stmt = if let Some(local) = let_local {
    Stmt::Local(Local {
      init: Some(LocalInit {
        eq_token: init_eq.unwrap(),
        expr: parse_quote!(#io_path!{}),
        diverge: init_diverge.unwrap(),
      }),
      ..local
    })
  } else {
    parse_quote!(#io_path!{};)
  };
  let span = io_path.span();
  gen_fn.block.stmts.insert(pos, io_stmt);

  let module_ty = &info.iftc_ty;
  let itfc_name = type_last_name_append(&module_ty, "Itfc", span.clone());
  let params_struct = new_params_struct(
    &type_last_name_append(&module_ty, "Params", span.clone()),
    &io_macro.fields,
  );

  let mk_func_span = utils::extract_span(gen_fn.span());

  let new_ctx_stmt: Stmt = parse_quote_spanned! {span =>
  let __cmt_gen = std::rc::Rc::new(std::cell::RefCell::new(Some(<#itfc_name as cmtrs::Itfc>::as_context(#params_struct, Some(#mk_func_span)))));};

  gen_fn.block.stmts.insert(pos, new_ctx_stmt);
  // let instantiate_subs_stmt: Stmt =
  //   parse_quote!(cmtrs::Itfc::instantiate_subs(&__cmt_gen););
  // gen_fn.block.stmts.insert(1, instantiate_subs_stmt);

  gen_fn
    .block
    .stmts
    .push(Stmt::Expr(parse_quote!(cmtrs::Itfc::dump(__cmt_gen)), None));

  Ok(())
}

pub fn new_params_struct(
  ty: &Type,
  fields: &Punctuated<ParamValue, Token![,]>,
) -> proc_macro2::TokenStream {
  let mut v = Vec::new();
  for field in fields {
    let name = type_param_name(&field.ident);
    let value = &field.value;
    v.push(quote! {#name: cmtrs::ToIOMacro::to_io_macro(#value)});
  }
  quote! {
    #ty {
      #(#v),*
    }
  }
}

pub struct InstanceMacro {
  pub name: Expr,
  pub _semi_token: Token![;],
  pub value: Expr,
}
impl Parse for InstanceMacro {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    Ok(InstanceMacro {
      name: input.parse()?,
      _semi_token: input.parse()?,
      value: input.parse()?,
    })
  }
}

pub mod rule_kw {
  use syn::custom_keyword;

  custom_keyword!(fsm);
  custom_keyword!(pipeline);
}

#[allow(unused)]
pub enum RuleTimingTy {
  FSM(rule_kw::fsm, Token![;]),
  Pipeline(rule_kw::pipeline, Token![;]),
  SingleCycle,
}

impl Parse for RuleTimingTy {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    if input.peek(rule_kw::fsm) {
      Ok(RuleTimingTy::FSM(input.parse()?, input.parse()?))
    } else if input.peek(rule_kw::pipeline) {
      Ok(RuleTimingTy::Pipeline(input.parse()?, input.parse()?))
    } else {
      Ok(RuleTimingTy::SingleCycle)
    }
  }
}

pub struct RuleMacro {
  name: Expr,
  _semi_token: Token![;],
  timing_ty: RuleTimingTy,
  _guard_bracket: Option<token::Bracket>,
  guard: Option<Expr>,
  _input_paren: token::Paren,
  inputs: Punctuated<Expr, Token![,]>,
  _rarrow_token: Option<Token![->]>,
  _output_paren: Option<token::Paren>,
  outputs: Option<Punctuated<Expr, Token![,]>>,
  body: Block,
}

impl Parse for RuleMacro {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let name = input.parse()?;
    let _semi_token = input.parse()?;
    let timing_ty = input.parse()?;
    let (_guard_bracket, guard) = if input.peek(token::Bracket) {
      let content;
      let _guard_bracket = bracketed!(content in input);
      (Some(_guard_bracket), Some(content.parse()?))
    } else {
      (None, None)
    };

    let input_content;
    let _input_paren = parenthesized!(input_content in input);
    let inputs = input_content.parse_terminated(Expr::parse, Token![,])?;

    let (_rarrow_token, _output_paren, outputs) = if input.peek(Token![->]) {
      let _rarrow_token = input.parse()?;
      let output_content;
      let _output_paren = parenthesized!(output_content in input);
      let outputs = output_content.parse_terminated(Expr::parse, Token![,])?;
      (Some(_rarrow_token), Some(_output_paren), Some(outputs))
    } else {
      (None, None, None)
    };

    Ok(RuleMacro {
      name,
      _semi_token,
      timing_ty,
      _guard_bracket,
      guard,
      _input_paren,
      inputs,
      _rarrow_token,
      _output_paren,
      outputs,
      body: input.parse()?,
    })
  }
}

pub enum RuleTy {
  Always,
  Method,
}

pub fn expand_rule_macro(
  ty: RuleTy,
  tokens: proc_macro2::TokenStream,
) -> Result<Expr, syn::Error> {
  let pm2_tokens: proc_macro2::TokenStream = tokens.clone().into();
  let RuleMacro {
    name,
    _semi_token: _semi_token1,
    timing_ty,
    guard,
    _guard_bracket: _semi_token2,
    _input_paren,
    inputs,
    _rarrow_token,
    _output_paren,
    outputs,
    body,
  } = syn::parse2(tokens)?;
  let mut rule_inputs = Vec::new();
  for i in inputs {
    rule_inputs.push(quote! {#i.value_id().unwrap()});
  }
  let mut rule_outputs = Vec::new();
  if let Some(out) = outputs {
    for o in out {
      rule_outputs.push(quote! {#o.value_id().unwrap()});
    }
  }
  let rule_guard = if let Some(guard) = guard {
    quote!(cmtrs::Itfc::rule_guard_var(&__cmt_gen, (#guard));)
  } else {
    quote!(cmtrs::Itfc::rule_guard(&__cmt_gen);)
  };

  let mut stmts: Vec<Stmt> = Vec::new();

  let sig_ident = match ty {
    RuleTy::Always => quote! {cmtrs::RuleSignature::Always},
    RuleTy::Method => quote! {cmtrs::RuleSignature::Method},
  };

  let side_effect = match ty {
    RuleTy::Always => quote! {},
    RuleTy::Method => quote! {side_effect: None},
  };

  let timing = match timing_ty {
    RuleTimingTy::FSM(_, _) => quote!(cmtrs::RuleTiming::FSM),
    RuleTimingTy::Pipeline(_, _) => quote!(cmtrs::RuleTiming::Pipeline),
    RuleTimingTy::SingleCycle => quote!(cmtrs::RuleTiming::SingleCycle),
  };

  let rule_span = extract_span(pm2_tokens.span());

  stmts.push(parse_quote! {
    cmtrs::Itfc::begin_rule(
    &__cmt_gen,
    false,
    false,
    #name,
    #sig_ident {
      inputs: vec![#(#rule_inputs),*],
      outputs: vec![#(#rule_outputs),*],
      #side_effect
    },
    #timing,
    None,
    None,
    Some(#rule_span),
  );
  });

  stmts.push(parse_quote!(#rule_guard));
  stmts.push(parse_quote!(#body;));
  stmts.push(Stmt::Expr(
    parse_quote!(cmtrs::Itfc::end_rule(&__cmt_gen)),
    None,
  ));
  let block: Block = parse_quote! {{
    #(#stmts)*
  }};

  Ok(Expr::Block(ExprBlock {
    attrs: Vec::new(),
    label: None,
    block,
  }))
}

struct ExtMethodMacro {
  name: Expr,
  _semi_token1: Option<Token![;]>,
  enable: Expr,
  _semi_token2: Option<Token![;]>,
  ready: Expr,
  _semi_token3: Option<Token![;]>,
  side_effect: Option<LitBool>,
  _semi_token4: Option<Token![;]>,
  guard: Option<Expr>,
  _semi_token5: Option<Token![;]>,
  _input_paren: token::Paren,
  inputs: Punctuated<Expr, Token![,]>,
  _rarrow_token: Option<Token![->]>,
  _output_paren: Option<token::Paren>,
  outputs: Option<Punctuated<Expr, Token![,]>>,
  body: Block,
}

impl Parse for ExtMethodMacro {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let name = input.parse()?;
    let _semi_token1 = input.parse()?;
    let enable = input.parse()?;
    let _semi_token2 = input.parse()?;
    let ready = input.parse()?;
    let _semi_token3 = input.parse()?;
    // if peek true/false, then parse side_effect
    let (side_effect, _semi_token4) = if input.peek(LitBool) {
      (Some(input.parse()?), Some(input.parse()?))
    } else {
      (None, None)
    };
    let (guard, _semi_token5) = if !input.peek(token::Paren) {
      (Some(input.parse()?), Some(input.parse()?))
    } else {
      (None, None)
    };

    let input_content;
    let _input_paren = parenthesized!(input_content in input);
    let inputs = input_content.parse_terminated(Expr::parse, Token![,])?;

    let (_rarrow_token, _output_paren, outputs) = if input.peek(Token![->]) {
      let _rarrow_token = input.parse()?;
      let output_content;
      let _output_paren = parenthesized!(output_content in input);
      let outputs = output_content.parse_terminated(Expr::parse, Token![,])?;
      (Some(_rarrow_token), Some(_output_paren), Some(outputs))
    } else {
      (None, None, None)
    };

    Ok(ExtMethodMacro {
      name,
      _semi_token1,
      enable,
      _semi_token2,
      ready,
      _semi_token3,
      side_effect,
      _semi_token4,
      guard,
      _semi_token5,
      _input_paren,
      inputs,
      _rarrow_token,
      _output_paren,
      outputs,
      body: input.parse()?,
    })
  }
}

pub fn expand_ext_method_macro(
  tokens: proc_macro2::TokenStream,
) -> Result<Expr, syn::Error> {
  let pm2_tokens: proc_macro2::TokenStream = tokens.clone().into();
  let ExtMethodMacro {
    name,
    _semi_token1,
    enable,
    _semi_token2,
    ready,
    _semi_token3,
    side_effect,
    _semi_token4,
    guard,
    _semi_token5,
    _input_paren,
    inputs,
    _rarrow_token,
    _output_paren,
    outputs,
    body,
  } = syn::parse2(tokens)?;

  let side_effect_is_some = LitBool::new(
    side_effect.is_some(),
    side_effect
      .clone()
      .map(|x| x.span())
      .unwrap_or(ready.span()),
  );
  let side_effect = side_effect.unwrap_or(LitBool::new(false, ready.span()));
  let mut rule_inputs = Vec::new();
  for i in inputs {
    rule_inputs.push(quote! {#i.value_id().unwrap()});
  }
  let mut rule_outputs = Vec::new();
  if let Some(out) = outputs {
    for o in out {
      rule_outputs.push(quote! {#o.value_id().unwrap()});
    }
  }
  let rule_guard = if let Some(guard) = guard {
    quote!(cmtrs::Itfc::rule_guard_var(&__cmt_gen, #guard);)
  } else {
    quote!(cmtrs::Itfc::rule_guard(&__cmt_gen);)
  };

  let mut stmts: Vec<Stmt> = Vec::new();

  let ext_method_span = extract_span(pm2_tokens.span());

  stmts.push(parse_quote! {
    cmtrs::Itfc::begin_rule(
    &__cmt_gen,
    true,
    false,
    #name,
    cmtrs::RuleSignature::Method {
      inputs: vec![#(#rule_inputs),*],
      outputs: vec![#(#rule_outputs),*],
      side_effect: if #side_effect_is_some { Some(#side_effect) } else { None },
    },
    cmtrs::RuleTiming::SingleCycle,
    #enable,
    #ready,
    Some(#ext_method_span),
  );
  });

  stmts.push(parse_quote!(#rule_guard));
  stmts.push(parse_quote!(#body;));
  stmts.push(Stmt::Expr(
    parse_quote!(cmtrs::Itfc::end_rule(&__cmt_gen)),
    None,
  ));
  let block: Block = parse_quote! {{
    #(#stmts)*
  }};

  Ok(Expr::Block(ExprBlock {
    attrs: Vec::new(),
    label: None,
    block,
  }))
}

pub mod method_rel_kw {
  use syn::custom_keyword;

  custom_keyword!(C);
  custom_keyword!(CF);
  custom_keyword!(SA);
  custom_keyword!(SB);
}

#[allow(unused)]
pub enum MethodRelTy {
  C(method_rel_kw::C),
  CF(method_rel_kw::CF),
  SA(method_rel_kw::SA),
  SB(method_rel_kw::SB),
}

impl Parse for MethodRelTy {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    if input.peek(method_rel_kw::C) {
      Ok(MethodRelTy::C(input.parse()?))
    } else if input.peek(method_rel_kw::CF) {
      Ok(MethodRelTy::CF(input.parse()?))
    } else if input.peek(method_rel_kw::SA) {
      Ok(MethodRelTy::SA(input.parse()?))
    } else if input.peek(method_rel_kw::SB) {
      Ok(MethodRelTy::SB(input.parse()?))
    } else {
      Err(syn::Error::new(input.span(), "Expect C / CF keyword"))
    }
  }
}

struct MethodRelMacro {
  _bracket_token1: Option<token::Bracket>,
  lhs: Punctuated<Expr, Token![,]>,
  ty: MethodRelTy,
  _bracket_token2: Option<token::Bracket>,
  rhs: Punctuated<Expr, Token![,]>,
}

impl Parse for MethodRelMacro {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let (_bracket_token1, lhs) = if input.peek(token::Bracket) {
      let content;
      let _bracket_token1 = bracketed!(content in input);
      let lhs = content.parse_terminated(Expr::parse, Token![,])?;
      (Some(_bracket_token1), lhs)
    } else {
      let val: Expr = input.parse()?;
      let mut lhs: Punctuated<Expr, token::Comma> = Punctuated::new();
      lhs.push(val);
      (None, lhs)
    };

    let ty = input.parse()?;

    let (_bracket_token2, rhs) = if input.peek(token::Bracket) {
      let content;
      let _bracket_token2 = bracketed!(content in input);
      let rhs = content.parse_terminated(Expr::parse, Token![,])?;
      (Some(_bracket_token2), rhs)
    } else {
      let val: Expr = input.parse()?;
      let mut rhs: Punctuated<Expr, token::Comma> = Punctuated::new();
      rhs.push(val);
      (None, rhs)
    };

    Ok(MethodRelMacro {
      _bracket_token1,
      lhs,
      ty,
      _bracket_token2,
      rhs,
    })
  }
}

pub fn expand_method_rel_macro(
  tokens: TokenStream,
) -> Result<Expr, syn::Error> {
  let MethodRelMacro {
    _bracket_token1,
    lhs,
    ty,
    _bracket_token2,
    rhs,
  } = syn::parse2(tokens)?;

  let ty = match ty {
    MethodRelTy::C(_) => quote! {cmtrs::MethodRel::C},
    MethodRelTy::CF(_) => quote! {cmtrs::MethodRel::CF},
    MethodRelTy::SA(_) => quote! {cmtrs::MethodRel::SA},
    MethodRelTy::SB(_) => quote! {cmtrs::MethodRel::SB},
  };

  Ok(parse_quote! {
    cmtrs::Itfc::method_rel(&__cmt_gen, #ty, &[#lhs], &[#rhs])
  })
}

struct MethodRelRawMacro {
  lhs: Expr,
  ty: MethodRelTy,
  rhs: Expr,
}
impl Parse for MethodRelRawMacro {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    Ok(MethodRelRawMacro {
      lhs: input.parse()?,
      ty: input.parse()?,
      rhs: input.parse()?,
    })
  }
}

pub fn expand_method_rel_raw_macro(
  tokens: TokenStream,
) -> Result<Expr, syn::Error> {
  let MethodRelRawMacro { lhs, ty, rhs } = syn::parse2(tokens)?;

  let ty = match ty {
    MethodRelTy::C(_) => quote! {cmtrs::MethodRel::C},
    MethodRelTy::CF(_) => quote! {cmtrs::MethodRel::CF},
    MethodRelTy::SA(_) => quote! {cmtrs::MethodRel::SA},
    MethodRelTy::SB(_) => quote! {cmtrs::MethodRel::SB},
  };

  Ok(parse_quote! {
    cmtrs::Itfc::method_rel(&__cmt_gen, #ty, #lhs, #rhs)
  })
}

struct ScheduleMacro {
  val: Punctuated<Expr, Token![,]>,
}
impl Parse for ScheduleMacro {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    Ok(ScheduleMacro {
      val: input.parse_terminated(Expr::parse, Token![,])?,
    })
  }
}

pub fn expand_schedule_macro(tokens: TokenStream) -> Result<Expr, syn::Error> {
  let ScheduleMacro { val } = syn::parse2(tokens)?;
  Ok(parse_quote! {
    cmtrs::Itfc::schedule(&__cmt_gen, &[#val])
  })
}

struct ExternalMacro {
  _in_bracket_token: token::Bracket,
  inputs: Punctuated<Expr, Token![,]>,
  _comma1: Token![,],
  _out_bracket_token: token::Bracket,
  outputs: Punctuated<Expr, Token![,]>,
  _comma2: Token![,],
  clock: Expr,
  _comma3: Token![,],
  reset: Expr,
  _comma4: Token![,],
  fir_str: Expr,
}
impl Parse for ExternalMacro {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let in_content;
    let out_content;
    let result = ExternalMacro {
      _in_bracket_token: bracketed!(in_content in input),
      inputs: in_content.parse_terminated(Expr::parse, Token![,])?,
      _comma1: input.parse()?,
      _out_bracket_token: bracketed!(out_content in input),
      outputs: out_content.parse_terminated(Expr::parse, Token![,])?,
      _comma2: input.parse()?,
      clock: input.parse()?,
      _comma3: input.parse()?,
      reset: input.parse()?,
      _comma4: input.parse()?,
      fir_str: input.parse()?,
    };
    if input.peek(Token![,]) {
      let _: Token![,] = input.parse()?;
    }
    Ok(result)
  }
}

pub fn expand_external_macro(tokens: TokenStream) -> Result<Expr, syn::Error> {
  let ExternalMacro {
    _in_bracket_token,
    inputs,
    _comma1,
    _out_bracket_token,
    outputs,
    _comma2,
    clock,
    _comma3,
    reset,
    _comma4,
    fir_str,
  } = syn::parse2(tokens)?;

  Ok(parse_quote!(
    cmtrs::Itfc::external(&__cmt_gen, vec![#inputs], vec![#outputs], #clock, #reset, #fir_str)
  ))
}

struct ModuleAnno {
  lhs: Expr,
  _comma_token: Token![:],
  rhs: Expr,
}
impl Parse for ModuleAnno {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    Ok(ModuleAnno {
      lhs: input.parse()?,
      _comma_token: input.parse()?,
      rhs: input.parse()?,
    })
  }
}
struct AnnoMacro {
  vals: Punctuated<ModuleAnno, Token![,]>,
}
impl Parse for AnnoMacro {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    Ok(AnnoMacro {
      vals: input.parse_terminated(ModuleAnno::parse, Token![,])?,
    })
  }
}

pub fn expand_anno_macro(tokens: TokenStream) -> Result<Expr, syn::Error> {
  let AnnoMacro { vals } = syn::parse2(tokens)?;
  let mut annos = Vec::new();
  for ModuleAnno {
    lhs,
    _comma_token,
    rhs,
  } in vals
  {
    annos.push(quote!(cmtrs::Itfc::add_annotation(&__cmt_gen, #lhs, #rhs)));
  }
  Ok(parse_quote!({
    #(#annos);*
  }))
}

mod op_kw {
  use syn::custom_keyword;

  custom_keyword!(loose);
}

struct VarMacro {
  name: Option<Expr>,
  _semi_token: Option<Token![;]>,
  value: Expr,
}
impl Parse for VarMacro {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let expr: Expr = input.parse()?;
    if input.peek(Token![;]) {
      Ok(VarMacro {
        name: Some(expr),
        _semi_token: Some(input.parse()?),
        value: input.parse()?,
      })
    } else {
      Ok(VarMacro {
        name: None,
        _semi_token: None,
        value: expr,
      })
    }
  }
}

pub fn expand_var_macro(tokens: TokenStream) -> Result<Expr, syn::Error> {
  let VarMacro {
    name,
    _semi_token,
    value,
  } = syn::parse2(tokens)?;
  let name = if let Some(name) = name {
    quote!(Some(#name))
  } else {
    quote!(None)
  };
  let span = extract_span(name.span());
  Ok(parse_quote!({
    let __var = cmtrs::Itfc::add_var(&__cmt_gen, #name);
    cmtrs::Itfc::assign(&__cmt_gen, &__var, #value, Some(#span));
    __var
  }))
}

struct IfMacro {
  cond: Expr,
  then_body: Block,
  _else_token: Option<Token![else]>,
  else_body: Option<Block>,
}

impl Parse for IfMacro {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let cond = input.parse()?;
    let then_body = input.parse()?;
    let (_else_token, else_body) = if input.peek(Token![else]) {
      (Some(input.parse()?), Some(input.parse()?))
    } else {
      (None, None)
    };
    Ok(IfMacro {
      cond,
      then_body,
      _else_token,
      else_body,
    })
  }
}
pub fn expand_if_macro(tokens: TokenStream) -> Result<Expr, syn::Error> {
  let span = extract_span(tokens.span());
  let IfMacro {
    cond,
    then_body,
    _else_token,
    else_body,
  } = syn::parse2(tokens)?;
  Ok(parse_quote!(
    {
      cmtrs::Itfc::if_(&__cmt_gen, Some(#span));
      cmtrs::Itfc::statement(&__cmt_gen, {#cond});
      cmtrs::Itfc::sep(&__cmt_gen);
      #then_body
      cmtrs::Itfc::sep(&__cmt_gen);
      #else_body
      cmtrs::Itfc::end(&__cmt_gen)
    }
  ))
}

// struct RetMacro {
//   exprs: Punctuated<Expr, Token![,]>,
// }
// impl Parse for RetMacro {
//   fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
//     Ok(RetMacro {
//       exprs: input.parse_terminated(Expr::parse, Token![,])?,
//     })
//   }
// }
pub fn expand_ret_macro(tokens: TokenStream) -> Result<Expr, syn::Error> {
  // let RetMacro { exprs } = syn::parse2(tokens)?;
  let span = extract_span(tokens.span());
  let expr: Expr = syn::parse2(tokens)?;
  if let Expr::Tuple(tup) = expr {
    let mut v = Vec::new();
    for e in tup.elems {
      v.push(quote!((#e).ast()));
    }
    Ok(parse_quote! {
      cmtrs::Itfc::ret(&__cmt_gen, vec![#(#v),*], Some(#span))
    })
  } else {
    Ok(parse_quote! {
      cmtrs::Itfc::ret(&__cmt_gen, vec![(#expr).ast()], Some(#span))
    })
  }
}

struct LooseKw {
  _tight: op_kw::loose,
  _semi_token: Token![;],
}
impl Parse for LooseKw {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    Ok(LooseKw {
      _tight: input.parse()?,
      _semi_token: input.parse()?,
    })
  }
}

impl LooseKw {
  fn parse_opt(input: syn::parse::ParseStream) -> syn::Result<Option<Self>> {
    if input.peek(op_kw::loose) {
      Ok(Some(input.parse()?))
    } else {
      Ok(None)
    }
  }
}

pub struct CtrlStmts {
  pub ops: Vec<Stmt>,
}
impl Parse for CtrlStmts {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    Ok(CtrlStmts {
      ops: input.call(Block::parse_within)?,
    })
  }
}

struct BranchMacro {
  loose: Option<LooseKw>,
  cond: Expr,
  then_body: Block,
  _else_token: Option<Token![else]>,
  else_body: Option<Block>,
}

impl Parse for BranchMacro {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let loose = LooseKw::parse_opt(input)?;
    let cond = input.parse()?;
    let then_body = input.parse()?;
    let (_else_token, else_body) = if input.peek(Token![else]) {
      (Some(input.parse()?), Some(input.parse()?))
    } else {
      (None, None)
    };
    Ok(BranchMacro {
      loose,
      cond,
      then_body,
      _else_token,
      else_body,
    })
  }
}

pub fn expand_branch_macro(tokens: TokenStream) -> Result<Expr, syn::Error> {
  let span = extract_span(tokens.span());
  let BranchMacro {
    loose,
    cond,
    then_body,
    _else_token,
    else_body,
  } = syn::parse2(tokens)?;
  let loose = if let Some(_) = loose {
    // emit_error!(tight._tight, "Tight timing is not supported yet!");
    quote!(true)
  } else {
    quote!(false)
  };

  Ok(parse_quote!(
    {
      cmtrs::Itfc::branch(&__cmt_gen, #loose, Some(#span));
      cmtrs::Itfc::statement(&__cmt_gen, {#cond});
      cmtrs::Itfc::sep(&__cmt_gen);
      {#then_body}
      cmtrs::Itfc::sep(&__cmt_gen);
      {#else_body}
      cmtrs::Itfc::statement(&__cmt_gen, cmtrs::Itfc::end(&__cmt_gen));
    }
  ))
}

struct LooseForMacro {
  _loose: LooseKw,
  _paren_token: token::Paren,
  init: Option<Expr>,
  _semi_token1: Token![;],
  cond: Expr,
  _semi_token2: Token![;],
  update: Option<Expr>,
  body: Block,
}

impl Parse for LooseForMacro {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let content;
    let _loose = input.parse()?;
    let _paren_token = parenthesized!(content in input);
    let init = if content.peek(Token![;]) {
      None
    } else {
      Some(content.parse()?)
    };
    let _semi_token1 = content.parse()?;
    let cond = content.parse()?;
    let _semi_token2 = content.parse()?;
    let update = if content.is_empty() {
      None
    } else {
      Some(content.parse()?)
    };
    if !content.is_empty() {
      return Err(content.error("Expect no more contents here"));
    }
    Ok(LooseForMacro {
      _loose,
      _paren_token,
      init,
      _semi_token1,
      cond,
      _semi_token2,
      update,
      body: input.parse()?,
    })
  }
}

struct TightForMacro {
  _paren_token: token::Paren,
  init: Option<Expr>,
  _semi_token1: Token![;],
  init_cond: Option<Expr>,
  _semi_token2: Token![;],
  update: Option<Expr>,
  _semi_token3: Token![;],
  update_cond: Expr,
  body: Block,
}

impl Parse for TightForMacro {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let content;
    let _paren_token = parenthesized!(content in input);
    let init = if content.peek(Token![;]) {
      None
    } else {
      Some(content.parse()?)
    };
    let _semi_token1 = content.parse()?;
    let init_cond = if content.peek(Token![;]) {
      None
    } else {
      Some(content.parse()?)
    };
    let _semi_token2 = content.parse()?;
    let update = if content.peek(Token![;]) {
      None
    } else {
      Some(content.parse()?)
    };
    let _semi_token3 = content.parse()?;
    let update_cond = content.parse()?;

    if !content.is_empty() {
      return Err(content.error("Expect no more contents here"));
    }
    Ok(TightForMacro {
      _paren_token,
      init,
      _semi_token1,
      init_cond,
      _semi_token2,
      update,
      _semi_token3,
      update_cond,
      body: input.parse()?,
    })
  }
}

enum ForMacro {
  Tight(TightForMacro),
  Loose(LooseForMacro),
}

impl Parse for ForMacro {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    if input.peek(op_kw::loose) {
      Ok(ForMacro::Loose(input.parse()?))
    } else {
      Ok(ForMacro::Tight(input.parse()?))
    }
  }
}

pub fn expand_for_macro(tokens: TokenStream) -> Result<Expr, syn::Error> {
  let span = extract_span(tokens.span());
  let for_macro: ForMacro = syn::parse2(tokens)?;

  match for_macro {
    ForMacro::Loose(LooseForMacro {
      _loose,
      _paren_token,
      init,
      _semi_token1,
      cond,
      _semi_token2,
      update,
      body,
    }) => Ok(parse_quote!( {
      cmtrs::Itfc::for_(&__cmt_gen, true, Some(#span));
      {#init}
      cmtrs::Itfc::sep(&__cmt_gen);
      cmtrs::Itfc::statement(&__cmt_gen, {#cond});
      cmtrs::Itfc::sep(&__cmt_gen);
      {#update}
      cmtrs::Itfc::sep(&__cmt_gen);
      {#body};
      cmtrs::Itfc::statement(&__cmt_gen, cmtrs::Itfc::end(&__cmt_gen));
    })),
    ForMacro::Tight(TightForMacro {
      _paren_token,
      init,
      _semi_token1,
      init_cond,
      _semi_token2,
      update,
      _semi_token3,
      update_cond,
      body,
    }) => {
      let init_cond = if let Some(init_cond) = init_cond {
        quote! {cmtrs::Itfc::statement(&__cmt_gen, {#init_cond});}
      } else {
        quote! {}
      };
      Ok(parse_quote!( {
        cmtrs::Itfc::for_(&__cmt_gen, false, Some(#span));
        {#init}
        cmtrs::Itfc::sep(&__cmt_gen);
        #init_cond
        cmtrs::Itfc::sep(&__cmt_gen);
        {#update}
        cmtrs::Itfc::sep(&__cmt_gen);
        cmtrs::Itfc::statement(&__cmt_gen, {#update_cond});
        cmtrs::Itfc::sep(&__cmt_gen);
        {#body};
        cmtrs::Itfc::statement(&__cmt_gen, cmtrs::Itfc::end(&__cmt_gen));
      }))
    }
  }
}

pub struct SimPrintMacro {
  pub exprs: Punctuated<Expr, Token![,]>,
}

impl Parse for SimPrintMacro {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    Ok(SimPrintMacro {
      exprs: input.parse_terminated(Expr::parse, Token![,])?,
    })
  }
}
