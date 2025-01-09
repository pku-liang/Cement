use proc_macro2::Span;
use proc_macro_error::*;
use quote::format_ident;
use syn::{Expr, Ident, Local, Pat, Path, Stmt, Token, Type, TypeParen};

pub(crate) fn type_param_name(ident: &Ident) -> Ident {
  // format_ident!("{}", snake_case(&ident.to_string()))
  ident.clone()
}

// pub(crate) fn type_last_name(ty: &Type) -> String {
//   if let Type::Paren(ty) = ty {
//     type_last_name(&ty.elem)
//   } else if let Type::Path(ty) = ty {
//     let t = ty.path.segments.last().unwrap();
//     if !t.arguments.is_none() {
//       abort!(ty, "type with argumuents is not supported yet");
//     }
//     t.ident.to_string()
//   } else {
//     abort!(ty, "Can not infer the name of the module from the type");
//   }
// }

pub(crate) fn type_last_name_append(ty: &Type, s: &str, span: Span) -> Type {
  if let Type::Paren(ty) = ty {
    Type::Paren(TypeParen {
      elem: Box::new(type_last_name_append(ty.elem.as_ref(), s, span)),
      ..ty.clone()
    })
  } else if let Type::Path(ty) = ty {
    let mut ty = ty.clone();
    let new_ident = format_ident!(
      "{}{s}",
      ty.path.segments.last().unwrap().ident,
      span = span
    );
    ty.path.segments.last_mut().unwrap().ident = new_ident;
    Type::Path(ty)
  } else {
    abort!(ty, "Can not append the name to the module from the type");
  }
}

pub(crate) fn macro_with_result(stmt: &Stmt, target: &str) -> bool {
  if let Stmt::Local(local) = &stmt {
    if let Some(init) = &local.init {
      if let Expr::Macro(macro_) = init.expr.as_ref() {
        if macro_.mac.path.is_ident(target) {
          return true;
        }
      }
    }
  }
  false
}

pub(crate) fn macro_without_result(stmt: &Stmt, target: &str) -> bool {
  if let Stmt::Macro(macro_) = stmt {
    if macro_.mac.path.is_ident(target) {
      return true;
    }
  } else if let Stmt::Expr(Expr::Macro(macro_), _) = stmt {
    if macro_.mac.path.is_ident(target) {
      return true;
    }
  }
  false
}

// pub(crate) struct StmtLocalHolder {
//   pub local: Local,
//   pub eq_token: Token![=],
//   pub diverge: Option<(Else, Box<Expr>)>,
// }
// impl StmtLocalHolder {
//   pub fn reconstruct(self, expr: Expr) -> Stmt {
//     Stmt::Local(Local {
//       init: Some(LocalInit {
//         eq_token: self.eq_token,
//         expr: Box::new(expr),
//         diverge: self.diverge,
//       }),
//       ..self.local
//     })
//   }
// }

// pub(crate) fn get_macro_with_result(
//   stmt: Stmt,
// ) -> (StmtLocalHolder, proc_macro2::TokenStream) {
//   if let Stmt::Local(mut local) = stmt {
//     let init = local.init.take().unwrap();
//     if let Expr::Macro(macro_) = *init.expr {
//       (
//         StmtLocalHolder {
//           local,
//           eq_token: init.eq_token,
//           diverge: init.diverge,
//         },
//         macro_.mac.tokens,
//       )
//     } else {
//       panic!()
//     }
//   } else {
//     panic!()
//   }
// }

// pub(crate) fn get_macro_without_result(
//   stmt: Stmt,
// ) -> (proc_macro2::TokenStream, Option<Token![;]>) {
//   if let Stmt::Macro(macro_) = stmt {
//     return (macro_.mac.tokens, macro_.semi_token);
//   } else if let Stmt::Expr(Expr::Macro(macro_), semi_token) = stmt {
//     return (macro_.mac.tokens, semi_token);
//   }
//   panic!()
// }

pub(crate) fn clone_macro_without_result(
  stmt: &mut Stmt,
) -> (proc_macro2::TokenStream, Path, Option<Token![;]>) {
  if let Stmt::Macro(macro_) = stmt {
    return (
      macro_.mac.tokens.clone(),
      macro_.mac.path.clone(),
      macro_.semi_token.clone(),
    );
  } else if let Stmt::Expr(Expr::Macro(macro_), semi_token) = stmt {
    return (
      macro_.mac.tokens.clone(),
      macro_.mac.path.clone(),
      semi_token.clone(),
    );
  }
  panic!()
}

pub(crate) fn get_name_from_local(local: &Local) -> Option<Ident> {
  if let Pat::Ident(pat) = &local.pat {
    return Some(pat.ident.clone());
  } else {
    if let Pat::Type(pat) = &local.pat {
      if let Pat::Ident(pat) = pat.pat.as_ref() {
        return Some(pat.ident.clone());
      }
    }
  }
  None
}

use quote::quote;

pub(crate) fn extract_span(
  span: proc_macro2::Span,
) -> proc_macro2::TokenStream {
  let file = span.source_file().path().to_string_lossy().to_string();
  let start_line = span.start().line;
  let start_column = span.start().column;
  let end_line = span.end().line;
  let end_column = span.end().column;
  let start_byte = span.byte_range().start;
  let end_byte = span.byte_range().end;
  quote! {
    cmtrs::MySpan {
      file: #file.to_string(),
      start_line: #start_line,
      start_column: #start_column,
      end_line: #end_line,
      end_column: #end_column,
      start_byte: #start_byte,
      end_byte: #end_byte,
    }
  }
}
