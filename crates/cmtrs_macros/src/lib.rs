#![crate_type = "proc-macro"]

use proc_macro::TokenStream;
use proc_macro2;
use proc_macro_error::*;
use quote::{quote, ToTokens};
use syn::{
  parse_macro_input, parse_quote, spanned::Spanned, visit_mut::VisitMut, Expr,
  ExprClosure, ItemFn,
};

mod itfc;
use itfc::*;

mod generator;
use generator::*;

mod utils;
use utils::*;

/// Declare an module interface. Type parameters, IO ports, nested interfaces
/// and method signatures can be declared.
///
/// Any module instance that have such interface will have the same rust type,
/// no matter how it is implemented inside.
///
/// # Language
/// ```"no rust"
/// itfc_declare{
///   [(param #param_name;)*] // declare type parameters
///   [pub] struct #itfc_name {
///     // declare input or output port with type, can be parameter or exact type
///     #io_name: (input | output) ((param #param_name) | #type_expr),
///     // declare nested interfaces with parameters
///     #nested_itfc: itfc #itfc_name { #param_name: ((param #param_name) | #type_expr) },
///     *
///   }
///   // declare method signature.
///   method #name (#inputs,*) [-> (#outputs,*)];
///   *
/// }
/// ```
///
/// # Examples
///
/// ## Counter
///
/// A counter with value type T, can read or set its value.
///
/// ```ignore
/// use cmtrs::*;
/// itfc_declare!(
///   param T;
///   struct Counter {
///     set_val: input param T,
///     count: output param T,
///   }
///   method read() -> (count);
///   method set(set_val);
/// );
///
/// /// #[module]
/// fn make_counter(t: &Type) -> Counter {
///   // ...
/// }
///
/// let counter = instance!(make_counter(&Type::Int(4)));
/// counter.set(literal(3, &Type::Int(4)));
/// let value = var!(counter.read());
/// ```
///
/// ## Sequence Merger
///
/// A sequence merger with value type T, which reads value from FIFOs a & b and
/// merged into FIFO c. It has no methods for itself, but the methods of a, b
/// and c can be directly called.
///
/// ```ignore
/// use cmtrs::*;
/// itfc_declare! {
///   param T;
///   pub struct SeqMerge {
///     a: itfc stl::FIFO{T: param T},
///     b: itfc stl::FIFO{T: param T},
///     c: itfc stl::FIFO{T: param T},
///   }
/// }
///
/// #[module]
/// fn make_seqmerge(t: &Type) -> SeqMerge {
///   // ...
/// }
///
/// let seq_merger = instance!(make_seqmerge(&Type::Int(4)));
/// seq_merger.a.enq(1.uint(4));
/// seq_merger.b.enq(2.uint(4));
/// let out = var!(seq_merger.c.deq());
/// ``````
///
/// # Notices
///
/// + Currently, method signature is not checked during generation. The
///   generator only checks if all methods declared in the interface is
///   implemented with the same name.
/// + The type parameters in the nested interfaces is not checked as well.
/// ```
#[proc_macro]
#[proc_macro_error]
pub fn itfc_declare(macro_input: TokenStream) -> TokenStream {
  let decl = parse_macro_input!(macro_input as ItfcDecl);

  let (io_name, io_struct) = gen_io_struct(&decl);
  let (params_name, params_struct) = gen_params_struct(&decl);
  let (itfc_name, itfc_struct) = gen_iftc_struct(&decl, &params_name);
  let itfc_impl = impl_itfc(&decl, &itfc_name, &io_name, &params_name);
  let instance_struct = gen_instance_struct(&decl);
  let instance_impl = impl_instance(&decl, &itfc_name, &params_name);
  let method_impl = impl_methods(&decl);

  abort_if_dirty();

  quote! {
    #io_struct
    #params_struct
    #itfc_struct
    #itfc_impl
    #instance_struct
    #instance_impl
    #method_impl
  }
  .into()
}

/// Implement a module generator for certain interface.
///
/// `#[module]` is an attribute for a function, which generates a hardware
/// module.
///
/// # Language
/// ```ignore
/// #[module]
/// fn #module_generator(...) -> #itfc_name {
///   // io! statement is always required
///   let io = io!{...};
///   // other statements
///   // automatically returns
/// }
/// ```
///
///
/// # Example
///
/// ```
/// use cmtrs::*;
/// itfc_declare! {
///   param T;
///   struct Counter {
///     set_val: input param T,
///     count: output param T,
///   }
///   method read() -> (count);
///   method set(set_val);
/// }
///
/// #[module]
/// fn make_counter(lim: usize, ty: &Type) -> Counter {
///   let io = io! {
///     T: ty
///   };
///   anno!("synthesis": "true");
///
///   let reg_i = instance!(stl::reg(&ty));
///
///   let inc = always! {
///     () {
///       reg_i.write( if_!(
///         reg_i.read().lt(lim.lit(ty)) {
///           let x = var!(reg_i.read() + 1.lit(ty));
///           ret!(x);
///         } else {
///           ret!(0.lit(ty));
///         }
///       ))
///     }
///   };
///
///   let set = method!(
///     (io.set_val) { reg_i.write(io.set_val); }
///   );
///
///   let read = method! {
///     () -> (io.count) { reg_i.read(); }
///   };
///
///   schedule!(inc, set, read);
/// }
/// ```
#[proc_macro_attribute]
#[proc_macro_error]
pub fn module(_attr: TokenStream, macro_input: TokenStream) -> TokenStream {
  let mut gen_fn = parse_macro_input!(macro_input as ItemFn);

  let info = extract_info(&gen_fn);
  for stmt in gen_fn.block.stmts.iter_mut() {
    macro_fill(stmt);
  }
  abort_if_dirty();

  if let Err(e) = io_macro(&info, &mut gen_fn) {
    return e.to_compile_error().into();
  }
  abort_if_dirty();

  gen_fn.to_token_stream().into()
}

/// `let io = io! { (#param: #harware_type),* (#nested_itfc: #instance),* };`
///
/// Provide parameters for the generator. Returns a struct that contains the
/// IOs.
///
/// Every module generater requires a io! macro call to work, even if it has no
/// parameters or IOs.
///
/// * Important: * The io! macro must be the first Cement macro in a module
///   generator.
///
/// # Examples
///
/// ## Counter
///
/// ```
/// use cmtrs::*;
/// itfc_declare! {
///   param T;
///   struct Counter {
///     set_val: input param T,
///     count: output param T,
///   }
///   method read() -> (count);
///   method set(set_val);
/// }
///
/// #[module]
/// fn counter_generator(t: &Type) -> Counter {
///   let io = io!{ T: t };
///   let set_val = io.set_val;
///   let count = io.count;
/// }
/// ```
///
/// ## Sequence Merger
///
/// ```
/// use cmtrs::*;
/// itfc_declare! {
///   param T;
///   pub struct SeqMerge {
///     a: itfc stl::FIFO{T: param T},
///     b: itfc stl::FIFO{T: param T},
///     c: itfc stl::FIFO{T: param T},
///   }
/// }
///
/// #[module]
/// fn seq_merge(t: &Type) -> SeqMerge {
///   let io = io!{
///     T:t.clone(),
///     a: stl::FIFO::new(4, t),
///     b: stl::FIFO::new(10, t),
///     c: stl::FIFO::new(1, t)
///   };
///   io.a.enq(literal(3, t));
///   io.b.deq();
/// }
/// ```
#[proc_macro]
#[proc_macro_error]
pub fn io(_macro_input: TokenStream) -> TokenStream {
  quote!(cmtrs::Itfc::io(__cmt_gen.clone())).into()
}

/// `let #instance_name = instance!(#module);`
///
/// Instance a submodule.
///
/// # Example
///
/// ```
/// use cmtrs::*;
/// # itfc_declare! {
/// #   param T;
/// #   struct Counter {
/// #     set_val: input param T,
/// #     count: output param T,
/// #   }
/// #   method read() -> (count);
/// #   method set(set_val);
/// # }
///
/// #[module]
/// fn counter_generator(lim: u32, t: &Type) -> Counter {
///   let io = io!{ T: t };
///
///   let reg_i = instance!(stl::reg(t));
/// }
/// ```
#[proc_macro]
#[proc_macro_error]
pub fn instance(macro_input: TokenStream) -> TokenStream {
  let InstanceMacro {
    name,
    _semi_token,
    value,
  } = parse_macro_input!(macro_input);
  let mut expr: Expr =
    parse_quote!(cmtrs::Itfc::instantiate(&__cmt_gen, #name, #value));
  MacroFill::new().visit_expr_mut(&mut expr);
  abort_if_dirty();
  quote!(#expr).into()
}

/// `named_instance(#instance_name; module);`
///
/// Similar to [instance!] but can specify the instance name.
/// `#instance_name` is a [String]
#[proc_macro]
#[proc_macro_error]
pub fn named_instance(macro_input: TokenStream) -> TokenStream {
  let InstanceMacro {
    name,
    _semi_token,
    value,
  } = parse_macro_input!(macro_input);
  let mut expr: Expr =
    parse_quote!(cmtrs::Itfc::instantiate(&__cmt_gen, #name, #value));
  MacroFill::new().visit_expr_mut(&mut expr);
  abort_if_dirty();
  quote!(#expr).into()
}

/// `let #rule_name = always! { [fsm; | pipeline;] [\[#guard\]] (#inputs) [->
/// (#outputs)] { #body } };`
///
/// Make an always rule for the module.
/// `fsm;` or `pipeline;` are optional keywords, which enable the use of
/// sequential control statements. `\[#guard\]` is a optional expression to
/// describe the fire condition of the rule. `#inputs` and `#outputs` are IOs of
/// the module.
///
/// Use [named_always!] to specify the rule name in [String] form.
///
/// ```
/// use cmtrs::*;
/// # itfc_declare! {
/// #   param T;
/// #   struct Counter {
/// #     set_val: input param T,
/// #     count: output param T,
/// #   }
/// #   method read() -> (count);
/// #   method set(set_val);
/// # }
///
/// #[module]
/// fn counter_generator(lim: i32, t: &Type) -> Counter {
///   let io = io!{ T: t };
///
///   let mut reg_i = instance!(stl::reg(t));
///
///   // if i<=lim { i = i + 1; }
///   let inc = always! {
///     [reg_i.lt(literal(lim, t))]
///     () { reg_i %= &reg_i + literal(1, t); }
///   };
///
///   // if i > lim { i = 0; }
///   let rst = always! {
///     [reg_i.ge(literal(lim, t))]
///     () { reg_i %= literal(0, t); }
///   };
///
///   // ...
/// }
/// ```
#[proc_macro]
#[proc_macro_error]
pub fn always(macro_input: TokenStream) -> TokenStream {
  let tokens: proc_macro2::TokenStream = macro_input.into();
  let mut expanded = match expand_rule_macro(RuleTy::Always, tokens) {
    Ok(e) => e,
    Err(e) => return e.to_compile_error().into(),
  };
  MacroFill::new().visit_expr_mut(&mut expanded);
  abort_if_dirty();
  quote!(#expanded).into()
}

/// `let rule = named_always! { #name; [\[#guard\]] (#inputs) [-> (#outputs)] {
/// #body }   };`
///
/// Similar to [always!], but can specify a name. `#name` is a [String].
#[proc_macro]
#[proc_macro_error]
pub fn named_always(macro_input: TokenStream) -> TokenStream {
  let tokens: proc_macro2::TokenStream = macro_input.into();
  let mut expanded = match expand_rule_macro(RuleTy::Always, tokens) {
    Ok(e) => e,
    Err(e) => return e.to_compile_error().into(),
  };
  MacroFill::new().visit_expr_mut(&mut expanded);
  abort_if_dirty();
  quote!(#expanded).into()
}

/// `let #method_name = method! { [fsm; | pipeline;] [\[#guard\]] (#inputs) [->
/// (#outputs)] { #body } };`
///
/// Make a method for the module.
/// `fsm;` or `pipeline;` are optional keywords, which enable the use of
/// sequential control statements. `\[#guard\]` is a optional expression to
/// describe the fire condition of the rule. `#inputs` and `#outputs` are IOs of
/// the module.
///
/// Use [named_method!] to specify the method name in [String] form.
///
/// ```
/// use cmtrs::*;
/// itfc_declare! {
///   param T;
///   struct Counter {
///     set_val: input param T,
///     count: output param T,
///   }
///   method read() -> (count);
///   method set(set_val);
/// }
///
/// #[module]
/// fn counter_generator(lim: u32, t: &Type) -> Counter {
///   let io = io!{ T: t };
///
///   let mut reg_i = instance!(stl::reg(t));
///
///   let read = method! {
///     () -> (io.count) {
///       ret!(reg_i)
///     }
///   };
///   let set = method!(
///     (io.set_val) {
///       reg_i %= io.set_val;
///     }
///   );
///
///   // ...
/// }
/// ```
#[proc_macro]
#[proc_macro_error]
pub fn method(macro_input: TokenStream) -> TokenStream {
  let tokens: proc_macro2::TokenStream = macro_input.into();
  let mut expanded = match expand_rule_macro(RuleTy::Method, tokens) {
    Ok(e) => e,
    Err(e) => return e.to_compile_error().into(),
  };
  MacroFill::new().visit_expr_mut(&mut expanded);
  abort_if_dirty();
  quote!(#expanded).into()
}

/// `let method = named_method! { #name; [\[#guard\]] (#inputs) [-> (#outputs)]
/// { #body }   };`
///
/// Similar to [method!], but can specify a name. `#name` is a [String].
#[proc_macro]
#[proc_macro_error]
pub fn named_method(macro_input: TokenStream) -> TokenStream {
  let tokens: proc_macro2::TokenStream = macro_input.into();
  let mut expanded = match expand_rule_macro(RuleTy::Method, tokens) {
    Ok(e) => e,
    Err(e) => return e.to_compile_error().into(),
  };
  MacroFill::new().visit_expr_mut(&mut expanded);
  abort_if_dirty();
  quote!(#expanded).into()
}

/// `let #method_name = ext_method!{ #enable; #ready; [\[#guard\]] (#inputs) [->
/// (#outputs)] { #body }  };`
///
/// Declare a external method, which can override the name of the enable and
/// ready signal. #enable and #ready are `Option<String>`
#[proc_macro]
#[proc_macro_error]
pub fn ext_method(macro_input: TokenStream) -> TokenStream {
  let tokens: proc_macro2::TokenStream = macro_input.into();
  let mut expanded = match expand_ext_method_macro(tokens) {
    Ok(e) => e,
    Err(e) => return e.to_compile_error().into(),
  };
  MacroFill::new().visit_expr_mut(&mut expanded);
  abort_if_dirty();
  quote!(#expanded).into()
}

/// `let method = ext_method!{ #name; #enable; #ready; [[#guard\]] (#inputs) [->
/// (#outputs)] { #body }  };`
///
/// Similar to [ext_method!], but can specify a name. `#name` is a [String].
#[proc_macro]
#[proc_macro_error]
pub fn named_ext_method(macro_input: TokenStream) -> TokenStream {
  let tokens: proc_macro2::TokenStream = macro_input.into();
  let mut expanded = match expand_ext_method_macro(tokens) {
    Ok(e) => e,
    Err(e) => return e.to_compile_error().into(),
  };
  MacroFill::new().visit_expr_mut(&mut expanded);
  abort_if_dirty();
  quote!(#expanded).into()
}

/// `method_rel!(#method1 C #method2)`, conflicts.
///
/// `method_rel!(#method1 CF #method2)`, conflict free.
///
/// `method_rel!(#method1 SA #method2)`, #method1 is scheduled ahead of
/// #method2.
///
/// `method_rel!(#method1 SB #method2)`, #method1 is scheduled behind of
/// #method2.
///
/// `method_rel!([#method, *] #rel [#method, *])`, groups of methods have a
/// relationship with the other group of methods.
///
/// Specify the relationship between methods.
#[proc_macro]
#[proc_macro_error]
pub fn method_rel(macro_input: TokenStream) -> TokenStream {
  let tokens: proc_macro2::TokenStream = macro_input.into();
  let mut expanded = match expand_method_rel_macro(tokens) {
    Ok(e) => e,
    Err(e) => return e.to_compile_error().into(),
  };
  MacroFill::new().visit_expr_mut(&mut expanded);
  abort_if_dirty();
  quote!(#expanded).into()
}

/// `method_rel_raw!(#methods1 #rel #methods2)`. `#methods` is `&[RuleHandle]`
#[proc_macro]
#[proc_macro_error]
pub fn method_rel_raw(macro_input: TokenStream) -> TokenStream {
  let tokens: proc_macro2::TokenStream = macro_input.into();
  let mut expanded = match expand_method_rel_raw_macro(tokens) {
    Ok(e) => e,
    Err(e) => return e.to_compile_error().into(),
  };
  MacroFill::new().visit_expr_mut(&mut expanded);
  abort_if_dirty();
  quote!(#expanded).into()
}

/// `schedule!(#rule_or_method, *)`
///
/// Specify the combinational order of the rules and methods.
#[proc_macro]
#[proc_macro_error]
pub fn schedule(macro_input: TokenStream) -> TokenStream {
  let tokens: proc_macro2::TokenStream = macro_input.into();
  let mut expanded = match expand_schedule_macro(tokens) {
    Ok(e) => e,
    Err(e) => return e.to_compile_error().into(),
  };
  MacroFill::new().visit_expr_mut(&mut expanded);
  abort_if_dirty();
  quote!(#expanded).into()
}

/// `schedule_raw!(#rules_or_methods)`, accepts `&[RuleHandle]`
#[proc_macro]
#[proc_macro_error]
pub fn schedule_raw(macro_input: TokenStream) -> TokenStream {
  let mut expr: Expr = parse_macro_input!(macro_input);
  MacroFill::new().visit_expr_mut(&mut expr);
  abort_if_dirty();
  quote!(cmtrs::Itfc::schedule(&__cmt_gen, #expr)).into()
}

/// `anno!("key" : "value", *);`
///
/// Set annotiontations for the module.
///
/// # Commonly used:
/// + `"synthesis": "true"` Modules without this annotation will be inlined by
///   the compiler if possible.
/// + `"is_tb": "true"` Modules with this annotation will become a testbench.
#[proc_macro]
#[proc_macro_error]
pub fn anno(macro_input: TokenStream) -> TokenStream {
  let tokens: proc_macro2::TokenStream = macro_input.into();
  let expanded = match expand_anno_macro(tokens) {
    Ok(e) => e,
    Err(e) => return e.to_compile_error().into(),
  };
  quote!(#expanded).into()
}

/// `external!([inputs_names], [output_names], Option<clock_name>,
/// Option<reset_name>, firrtl_string);`
#[proc_macro]
#[proc_macro_error]
pub fn external(macro_input: TokenStream) -> TokenStream {
  let tokens: proc_macro2::TokenStream = macro_input.into();
  let expanded = match expand_external_macro(tokens) {
    Ok(e) => e,
    Err(e) => return e.to_compile_error().into(),
  };
  quote!(#expanded).into()
}

/// `input!(#name, #type)`
///
/// Add an additional input into to the module.
/// `#name` is a [String].
/// `#type` is a [Type](`crate::Type`)
#[proc_macro]
#[proc_macro_error]
pub fn input(macro_input: TokenStream) -> TokenStream {
  let tokens: proc_macro2::TokenStream = macro_input.into();
  quote! {
    cmtrs::Itfc::add_input(&__cmt_gen, #tokens)
  }
  .into()
}

/// `output!(#name, #type)`
///
/// Add an additional output into to the module.
/// `#name` is a [String].
/// `#type` is a [Type](`crate::Type`)
#[proc_macro]
#[proc_macro_error]
pub fn output(macro_input: TokenStream) -> TokenStream {
  let tokens: proc_macro2::TokenStream = macro_input.into();
  quote! {
    cmtrs::Itfc::add_output(&__cmt_gen, #tokens)
  }
  .into()
}

/// `set_name!(#name)`
///
/// Override the name of the module. `#name`` is a [String].
#[proc_macro]
#[proc_macro_error]
pub fn set_name(macro_input: TokenStream) -> TokenStream {
  let tokens: proc_macro2::TokenStream = macro_input.into();
  quote! {
    cmtrs::Itfc::set_name(&__cmt_gen, #tokens)
  }
  .into()
}

/// `distinguisher!(#distinguish_str)`
///
/// Append a disinguish string to name of the module to make it unique.
/// `#distinguish_str` is a [&str].
///
/// # Example
///
/// ```
/// use cmtrs::*;
/// itfc_declare! {
///  param T;
///  pub struct FIFO {
///    #[name("in")]
///    in_: input param T,
///    out: output param T,
///    full: output Type::UInt(1)
///  }
///  method full()->(full);
///  method enq(in_);
///  method deq()->(out);
/// }
///
/// #[module]
/// pub fn fifo_push(t: &Type, depth: usize) -> FIFO {
///   let io = io! { T: t };
///   // distinguish by depth and type
///   distinguisher!(&format!("{depth}_push"));
///   // ...
/// }
///
/// #[module]
/// pub fn fifo1_pull(t: &Type, depth: usize) -> FIFO {
///   let io = io! { T: t };
///   // distinguish by depth and type
///   distinguisher!(&format!("{depth}_pull"));
///   // ...
/// }
/// ```
#[proc_macro]
#[proc_macro_error]
pub fn distinguisher(macro_input: TokenStream) -> TokenStream {
  let tokens: proc_macro2::TokenStream = macro_input.into();
  quote! {
    cmtrs::Itfc::distinguisher(&__cmt_gen, #tokens)
  }
  .into()
}

/// `var!(#value)`
///
/// Make a variable (wire). Can be used within a rule.
///
/// # Example
/// ```
/// # use cmtrs::*;
/// # itfc_declare! {
/// #   param T;
/// #   struct Counter {
/// #     set_val: input param T,
/// #     count: output param T,
/// #   }
/// #   method read() -> (count);
/// #   method set(set_val);
/// # }
/// #
/// # #[module]
/// # fn make_counter(lim: usize, ty: &Type) -> Counter {
/// #   let io = io! {
/// #     T: ty
/// #   };
/// #  let reg_i = instance!(stl::reg(&ty));
/// #  let mut reg_a = instance!(stl::reg(&ty));
/// #  let mut reg_b = instance!(stl::reg(&ty));
/// let broadcast = always! {
///   () {
///     let x = var!(reg_i.read() + 1.lit(ty));
///     reg_a %= &x;
///     reg_b %= &x;
///   }
/// };
/// # }
/// ```
#[proc_macro]
#[proc_macro_error]
pub fn var(macro_input: TokenStream) -> TokenStream {
  let tokens: proc_macro2::TokenStream = macro_input.into();
  let mut expanded = match expand_var_macro(tokens) {
    Ok(e) => e,
    Err(e) => return e.to_compile_error().into(),
  };
  MacroFill::new().visit_expr_mut(&mut expanded);
  abort_if_dirty();
  quote!(#expanded).into()
}

/// `named_var!(#name; #value)`
///
/// Make a var with certain name. #name is [`Option<String>`]
#[proc_macro]
#[proc_macro_error]
pub fn named_var(macro_input: TokenStream) -> TokenStream {
  let tokens: proc_macro2::TokenStream = macro_input.into();
  let mut expanded = match expand_var_macro(tokens) {
    Ok(e) => e,
    Err(e) => return e.to_compile_error().into(),
  };
  MacroFill::new().visit_expr_mut(&mut expanded);
  abort_if_dirty();
  quote!(#expanded).into()
}

/// `if_!{#cond {#then_body} [else {#else_body}]}`
///
/// A combinational "if" statment in hardware. Can have return value.
///
/// Difference from [`branch!`]: `if_!` can not have sequential control
/// statements within its body.
///
/// # Example
/// ```
/// # use cmtrs::*;
/// # itfc_declare!(struct Top{});
/// # #[module] fn make_top() -> Top {
/// # io!{};
/// # let ty = Type::UInt(4);
/// # let mut reg_a = instance!(stl::reg(&ty));
/// # let mut reg_b = instance!(stl::reg(&ty));
/// # let mut reg_c = instance!(stl::reg(&ty));
/// let get_small = always!(
///   () {
///     reg_c %= if_!( &reg_a.lt(&reg_b) {
///       ret!(reg_a.read());
///     } else {
///       ret!(reg_b.read());
///     } );
///   }
/// );
/// // The same
/// let get_small2 = always!(
///   () {
///     if_!( &reg_a.lt(&reg_b) {
///       reg_c %= reg_a.read();
///     } else {
///       reg_c %= reg_b.read();
///     } );
///   }
/// );
/// # }
/// ```
#[proc_macro]
#[proc_macro_error]
pub fn if_(macro_input: TokenStream) -> TokenStream {
  let tokens: proc_macro2::TokenStream = macro_input.into();
  let mut expanded = match expand_if_macro(tokens) {
    Ok(e) => e,
    Err(e) => return e.to_compile_error().into(),
  };
  MacroFill::new().visit_expr_mut(&mut expanded);
  abort_if_dirty();
  quote!(#expanded).into()
}

/// `ret!(#value);`
///
/// `ret!((#value, *));`
///
/// Return a value or multiple values. A expression will not be recorded if it
/// is not returned.
///
/// Methods calls can be returned without `ret!`.
///
/// # Example
/// ```
/// # use cmtrs::*;
/// # itfc_declare!(struct Top{out: output Type::UInt(4)});
/// # #[module] fn make_top() -> Top {
/// # let io = io!{};
/// # let ty = Type::UInt(4);
/// # let mut reg_a = instance!(stl::reg(&ty));
/// let output_plus_one = method!(
///   () -> (io.out) {
///     // ret is required for expressions
///     ret!(reg_a.read() + 1.lit(&ty));
///   }
/// );
/// let output = method!(
///   () -> (io.out) {
///     // ret! is optional here
///     reg_a.read()
///   }
/// );
/// # }
/// ```
#[proc_macro]
#[proc_macro_error]
pub fn ret(macro_input: TokenStream) -> TokenStream {
  let tokens = macro_input.into();
  let mut expanded = match expand_ret_macro(tokens) {
    Ok(e) => e,
    Err(e) => return e.to_compile_error().into(),
  };
  MacroFill::new().visit_expr_mut(&mut expanded);
  abort_if_dirty();
  quote!(#expanded).into()
}

/// `ret_raw!(#values)`
///
/// Return multiples values. Similar to [`ret`]. `#values` is `&[Var]`.
#[proc_macro]
#[proc_macro_error]
pub fn ret_raw(macro_input: TokenStream) -> TokenStream {
  let mut expr: Expr = parse_macro_input!(macro_input);
  MacroFill::new().visit_expr_mut(&mut expr);
  abort_if_dirty();
  quote!(cmtrs::Itfc::ret(&__cmt_gen, #expr)).into()
}

/// `statement!(#value)`
///
/// Make an expression a statement. Currently, not very useful. If it is at a
/// return position, then it has the same effect as [`ret`]. Otherwise it is
/// possible that the expression will be optimized by the compiler.
#[proc_macro]
#[proc_macro_error]
pub fn statement(macro_input: TokenStream) -> TokenStream {
  let mut expr: Expr = parse_macro_input!(macro_input);
  MacroFill::new().visit_expr_mut(&mut expr);
  abort_if_dirty();
  quote!(cmtrs::Itfc::statement(&__cmt_gen, #expr)).into()
}

/// `step!(#statements;*)`
///
/// A `step` is a group of statements that will be executed together in a cycle
/// if activated. Though not very useful by itself, `step`s are the basic
/// execution units in high-level sequential control statements.
///
/// No sequential control statements is allowed within a `step!`.
///
/// Used only in rules or methods with a `fsm` or `pipeline` keyword.
///
/// # Example
/// ```
/// # use cmtrs::*;
/// # itfc_declare!(struct Top{out: output Type::UInt(4)});
/// # #[module] fn make_top() -> Top {
/// # let io = io!{};
/// # let ty = Type::UInt(4);
/// # let mut reg_a = instance!(stl::reg(&ty));
/// let work = method!(
///   fsm;
///   () {
///     seq!{
///       step!{reg_a %= 1.lit(&ty);};
///       step!{reg_a %= 2.lit(&ty);};
///       step!{reg_a %= 3.lit(&ty);};
///     };
///   }
/// );
/// # }
#[proc_macro]
#[proc_macro_error]
pub fn step(macro_input: TokenStream) -> TokenStream {
  let tokens: proc_macro2::TokenStream = macro_input.clone().into();
  let CtrlStmts { ops } = parse_macro_input!(macro_input);
  let span = extract_span(tokens.span());
  let mut expanded: Expr = parse_quote! { {
    cmtrs::Itfc::step(&__cmt_gen, Some(#span));
    #(#ops)*
    cmtrs::Itfc::statement(&__cmt_gen, cmtrs::Itfc::end(&__cmt_gen));
  } };
  MacroFill::new().visit_expr_mut(&mut expanded);
  abort_if_dirty();
  quote!(#expanded).into()
}

/// `seq!(#statements;*)`
///
/// A `seq` is a list of statements that will be executed in consecutive cycles
/// if activated. When one statement is finish, the next statment will start at
/// next cycle.
///
/// Used only in rules or methods with a `fsm` or `pipeline` keyword.
///
/// # Example
/// ```
/// # use cmtrs::*;
/// # itfc_declare!(struct Top{out: output Type::UInt(4)});
/// # #[module] fn make_top() -> Top {
/// # let io = io!{};
/// # let ty = Type::UInt(4);
/// # let mut reg_a = instance!(stl::reg(&ty));
/// let work = method!(
///   fsm;
///   () {
///   // Expect reg_a to be 1, 2 and 3 in three consecutive cylces once `work` is invoked.
///     seq!{
///       step!{reg_a %= 1.lit(&ty);};
///       step!{reg_a %= 2.lit(&ty);};
///       step!{reg_a %= 3.lit(&ty);};
///     };
///   }
/// );
/// # }
#[proc_macro]
#[proc_macro_error]
pub fn seq(macro_input: TokenStream) -> TokenStream {
  let tokens: proc_macro2::TokenStream = macro_input.clone().into();
  let CtrlStmts { ops } = parse_macro_input!(macro_input);
  let span = extract_span(tokens.span());
  let mut expanded: Expr = parse_quote! { {
    cmtrs::Itfc::seq(&__cmt_gen, Some(#span));
    #(#ops)*
    cmtrs::Itfc::statement(&__cmt_gen, cmtrs::Itfc::end(&__cmt_gen));
  } };
  MacroFill::new().visit_expr_mut(&mut expanded);
  abort_if_dirty();
  quote!(#expanded).into()
}

/// `par!(#statements;*)`
///
/// A `par` is a list of statements that will be executed in parallel. All
/// statements starts at the same cycle, then will be joined to wait for the
/// latest one to finish.
///
/// Used only in rules or methods with a `fsm` or `pipeline` keyword.
///
/// # Example
/// ```
/// # use cmtrs::*;
/// # itfc_declare!(struct Top{out: output Type::UInt(4)});
/// # #[module] fn make_top() -> Top {
/// # let io = io!{};
/// # let ty = Type::UInt(4);
/// # let mut reg_a = instance!(stl::reg(&ty));
/// # let mut reg_b = instance!(stl::reg(&ty));
/// let work = method!(
///   fsm;
///   () {
///     // Expect outcome
///     // a: 1, 2, 3
///     // b: 1, 1, 3
///     seq!{
///       par!{
///         seq!{
///           step!{reg_a %= 1.lit(&ty);};
///           step!{reg_a %= 2.lit(&ty);};
///         }
///         step!{reg_b %= 1.lit(&ty);};
///       };
///       step!{
///         reg_a %= 3.lit(&ty);
///         reg_b %= 3.lit(&ty);
///       }
///     }
///   }
/// );
/// # }
#[proc_macro]
#[proc_macro_error]
pub fn par(macro_input: TokenStream) -> TokenStream {
  let tokens: proc_macro2::TokenStream = macro_input.clone().into();
  let CtrlStmts { ops } = parse_macro_input!(macro_input);
  let span = extract_span(tokens.span());
  let mut expanded: Expr = parse_quote! { {
    cmtrs::Itfc::par(&__cmt_gen, Some(#span));
    #(#ops)*
    cmtrs::Itfc::statement(&__cmt_gen, cmtrs::Itfc::end(&__cmt_gen));
  } };
  MacroFill::new().visit_expr_mut(&mut expanded);
  abort_if_dirty();
  quote!(#expanded).into()
}

/// `branch` selectively executes statements according to a condition. In fact,
/// `branch` actually makes branchs on a FSM. Different from [`if_!`], it can
/// have sequential control statements within its body, but itself can not be
/// inside of a [`step!`].
///
/// There are two types:
/// + `branch!(loose; #cond {#then_body} [else {#else_body}])` A `branch` with
///   loose timing first takes a cycle to evaluate `#cond` condition. In the
///   next cycle, if `#cond` is true, then `#then_body` is executed. Otherwise
///   `#else_body` is executed, if existed. Loose `branch` is useful when the
///   critical path goes through the condition.
/// + `branch!(#cond {#then_body} [else {#else_body}])` A `branch` without
///   `loose` keyword means it has tight timing. As result, it saves the cycle
///   to evaluate `#cond`, so `#then_body` or `#else_body` starts execution at
///   the same cycle `#cond` is evaluated.
///
/// Used only in rules or methods with a `fsm` or `pipeline` keyword.
#[proc_macro]
#[proc_macro_error]
pub fn branch(macro_input: TokenStream) -> TokenStream {
  let tokens: proc_macro2::TokenStream = macro_input.into();
  let mut expanded = match expand_branch_macro(tokens) {
    Ok(e) => e,
    Err(e) => return e.to_compile_error().into(),
  };
  MacroFill::new().visit_expr_mut(&mut expanded);
  abort_if_dirty();
  quote!(#expanded).into()
}

/// `for_` mimics a C for-loop. There are two types:
///
/// + `for_!(loose;(#init; #cond; #update) {#for_body})`. A for-loop with loose
///   timing. First, `#init` takes one cycle to initialize. (skipped if empty)
///   Then the loop starts. In each loop iteration, first a cycle is taken to
///   evaluate the `#cond` condition, then it starts the execution of
///   `#for_body`. At the last cycle of `#for_body`, it will executes `#update`,
///   then jump back to `#cond`.
/// + `for_!((#init; #init_cond; #update; #updated_cond) {#for_body})`. A
///   for-loop with tight timing, which saved the cycle to evaluate the
///   condition. As result, the condition will be evaluated at the same cycle of
///   `#init` and `#update`. Due to the fact that anything can happen in these
///   two statements, tbe compiler cannot predict how the condition will
///   interact with them. The user need to provde two new conditions to evaluate
///   after `#init` and `#update` took place. E.g. if `#update` is `i += 1`,
///   `#cond` is `i<n`, then `#updated_cond` is `i+1<n`.
///
/// Used only in rules or methods with a `fsm` or `pipeline` keyword.
///
/// # Example
/// ```
/// # use cmtrs::*;
/// # itfc_declare! {
/// #   struct ForExample {
/// #     out: output Type::UInt(4)
/// #   }
/// # }
/// # #[module]
/// # fn make_for_example() -> ForExample {
/// #   let io = io! {};
/// #   let mut r = instance!(stl::reg(&Type::UInt(4)));
/// #  let mut sum = instance!(stl::reg(&Type::UInt(4)));
/// let start = method!(
///     fsm;
///     () {
///       seq!(
///         for_!(
///           ( r %= literal(0, &Type::UInt(4)); //init
///             true; //init_cond
///             r %= &r + literal(1, &Type::UInt(4)); //update
///             r.lt(literal(3, &Type::UInt(4))) //update_cond
///           ) { step!{ sum %= &sum + &r; }; }
///         );
///         step!{ sum %= 0.uint(4);};
///         for_!(
///           loose;
///           ( r%= 0.uint(4); //init
///             r.lt(literal(4, &Type::UInt(4))); //cond
///             r %= &r + literal(1, &Type::UInt(4)) //update
///           ) { step!{ sum %= &sum + &r; }; }
///         );
///       )
///     }
///   );
/// # }
/// ```
#[proc_macro]
#[proc_macro_error]
pub fn for_(macro_input: TokenStream) -> TokenStream {
  let tokens: proc_macro2::TokenStream = macro_input.into();
  let mut expanded = match expand_for_macro(tokens) {
    Ok(e) => e,
    Err(e) => return e.to_compile_error().into(),
  };
  MacroFill::new().visit_expr_mut(&mut expanded);
  abort_if_dirty();
  quote!(#expanded).into()
}

/// `move_!(|...| {...})`
///
/// A patch on `move || {}` closure. The generator context can not move to the
/// closure in common Rust way, so this `move_!` macro is required.
#[proc_macro]
#[proc_macro_error]
pub fn move_(macro_input: TokenStream) -> TokenStream {
  let mut closure: ExprClosure = parse_macro_input!(macro_input);
  if closure.capture.is_none() {
    closure.capture = Some(parse_quote!(move));
  }
  quote!(
    { let __cmt_gen = __cmt_gen.clone();
      #closure
    }
  )
  .into()
}

/// `#[gen_fn]` is an attribute on functions or methods. Attributed function
/// will act as if it is part of a module. Used together with [`generate!`]
/// macro.
///
/// # Example
///
/// This example shows the implementation of
/// [`stl::reg_pipe`](`cmtrs::stl::reg_pipe`), which makes a pipeline register
/// that is immediately used.
///
/// ```
/// use cmtrs::*;
/// #[gen_fn]
/// pub fn reg_pipe(name: String, t: &Type, input: Var) -> Var{
///   let r = named_instance!(name; stl::reg(t));
///   r.write(input);
///   r.read()
/// }
///
/// # itfc_declare!(struct Top{a: input Type::UInt(4), b: output Type::UInt(4)});
///
/// #[module]
/// fn top() -> Top {
///   let io = io!{};
///   let some_rule = always!{
///     (io.a) -> (io.b) {
///        ret!(generate!(reg_pipe("reg0".to_string(), &Type::UInt(4), io.a)))
///     }
///   };
/// }
/// ```
#[proc_macro_attribute]
#[proc_macro_error]
pub fn gen_fn(_attrs: TokenStream, macro_input: TokenStream) -> TokenStream {
  let mut gen_fn: ItemFn = parse_macro_input!(macro_input);

  gen_fn
    .sig
    .inputs
    .push(parse_quote!(__cmt_gen: std::rc::Rc<std::cell::RefCell<Option<impl cmtrs::Itfc>>>));

  quote!(#gen_fn).into()
}

/// `generate!(#invoke_gen_fn)`
///
/// Put the generated content of a [`#[gen_fn]`](`gen_fn`) attributed generate
/// function. Check [`#[gen_fn]`](`gen_fn`) for details.
#[proc_macro]
#[proc_macro_error]
pub fn generate(macro_input: TokenStream) -> TokenStream {
  let mut expr: Expr = parse_macro_input!(macro_input);

  match &mut expr {
    Expr::Call(call) => {
      call.args.push(parse_quote! {__cmt_gen.clone()});
    }
    Expr::MethodCall(call) => {
      call.args.push(parse_quote! {__cmt_gen.clone()});
    }
    _ => abort!(
      expr,
      "generate! macro only works on function or method call!"
    ),
  }

  quote!(#expr).into()
}

/// `sim_exit!();`
///
/// Declare the finish of simulation. Similar to `$finish` in Verilog
#[proc_macro]
#[proc_macro_error]
pub fn sim_exit(macro_input: TokenStream) -> TokenStream {
  let tokens: proc_macro2::TokenStream = macro_input.clone().into();
  let span = extract_span(tokens.span());
  quote!(cmtrs::Itfc::sim_exit(&__cmt_gen, Some(#span))).into()
}

/// `sim_print!(#args...)`
///
/// Print values during simulation.
#[proc_macro]
#[proc_macro_error]
pub fn sim_print(macro_input: TokenStream) -> TokenStream {
  let SimPrintMacro { exprs } = parse_macro_input!(macro_input);
  let span = extract_span(exprs.span());
  let mut values = Vec::new();
  for e in exprs {
    values.push(quote! {cmtrs::Itfc::print_item(&__cmt_gen, &#e);});
  }
  quote!({
    cmtrs::Itfc::print(&__cmt_gen, Some(#span));
    #(#values)*
    cmtrs::Itfc::statement(&__cmt_gen, cmtrs::Itfc::end(&__cmt_gen));
  })
  .into()
}
