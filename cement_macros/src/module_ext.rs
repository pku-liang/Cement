use proc_macro2::TokenStream;
use syn::parse2;
use syn::visit_mut::VisitMut;

use crate::module::Module;
use crate::visitor::HwVisitor;

pub(crate) fn module_ext_decl(input: TokenStream) -> TokenStream {
  let module: Result<Module, _> = parse2(input);
  let Module {
    ifc_generics,
    ifc_type,
    where_clause,
    ifc_impl,
    c,
    module_name,
    args,
    tcl,
    ext_sv,
    mut body,
  } = match module {
    Ok(module) => module,
    Err(e) => return e.to_compile_error().into(),
  };

  let mut hw_visitor = HwVisitor::new(c.to_owned());
  hw_visitor.visit_block_mut(&mut body);

  let (impl_generics, ..) = ifc_generics.split_for_impl();

  match (tcl, ext_sv) {
    (Some(tcl), None) => {
      quote! {
          impl #impl_generics #ifc_type #where_clause {
              pub fn #module_name(self, #c: &mut Cmtc, #args) -> <Self as Interface>::ImplT {
                  let #ifc_impl = #c.begin_module(self, stringify!(#module_name).to_string(), true);

                  let __op_id = #c.get_current_module_ip().expect("current_module is Some");
                  #c.add_tcl(__op_id, (#tcl).into());
                  
                  #body

                  #c.end_module::<Self>(true)
              }
          }
      }
    },
    (None, Some(_ext_sv)) => {
      unimplemented!("TODO: support externel verilog")
    },
    _ => {
      return syn::Error::new(
        proc_macro2::Span::call_site(),
        "Expected module declaration with either [tcl=...] or [ext_sv=...]",
      )
      .to_compile_error()
      .into();
    },
  }
}
