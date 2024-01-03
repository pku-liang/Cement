use proc_macro2::{Ident, TokenStream};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::token::PathSep;
use syn::visit::Visit;
use syn::visit_mut::{self, VisitMut};
use syn::*;

use crate::event::{event_transform, EventStmts};

#[derive(Default)]
struct NameVisitor {
  names: Vec<Option<Ident>>,
}
impl<'visit> Visit<'visit> for NameVisitor {
  fn visit_pat_wild(&mut self, _i: &PatWild) { self.names.push(None) }

  fn visit_ident(&mut self, i: &Ident) { self.names.push(Some(i.to_owned())) }
}
pub(crate) struct HwVisitor {
  pub(crate) c: proc_macro2::TokenStream,
}

impl HwVisitor {
  pub(crate) fn new(c: Ident) -> Self { Self { c: quote! {#c} } }
}

fn into_ident(
  leading_colon: Option<PathSep>, segments: Punctuated<PathSegment, PathSep>,
) -> Option<Ident> {
  if let None = leading_colon {
    let mut iter = segments.into_iter();
    if let Some(PathSegment { ident, arguments }) = iter.next() {
      if let (PathArguments::None, None) = (arguments, iter.next()) {
        return Some(ident);
      } else {
        None
      }
    } else {
      None
    }
  } else {
    None
  }
}

pub(crate) struct PunctratedExprWrapper {
  pub(crate) punctuated: Punctuated<Expr, Token![,]>,
}

impl Parse for PunctratedExprWrapper {
  fn parse(input: ParseStream) -> Result<Self> {
    Ok(PunctratedExprWrapper {
      punctuated: Punctuated::parse_terminated(input)?,
    })
  }
}

impl VisitMut for HwVisitor {
  fn visit_expr_mut(&mut self, expr: &mut Expr) {
    visit_mut::visit_expr_mut(self, expr);
    match expr {
      Expr::Binary(ExprBinary {
        left,
        op: BinOp::RemAssign(rem_assign),
        right,
        ..
      }) => {
        let left = *(*left).to_owned();
        let right = *(*right).to_owned();
        let c = self.c.to_owned();
        let span = rem_assign.spans[0];

        let new_expr: Expr = parse_quote_spanned! { span =>
            #left.connect_expr((#right).expr(), #c)
        };
        *expr = new_expr;
      },
      _ => {},
    }
  }

  fn visit_stmt_macro_mut(&mut self, i: &mut StmtMacro) {
    let StmtMacro {
      mac:
        Macro {
          path: Path { leading_colon, segments },
          tokens,
          ..
        },
      ..
    } = i.to_owned();
    if let Some(ident) = into_ident(leading_colon, segments) {
      match ident.to_string().as_str() {
        "event" => {
          let event_stmts: EventStmts = parse2(tokens).unwrap();
          let c = self.c.to_owned();
          let transformed = event_transform(EventStmts {
            c: Some(parse2(c).unwrap()),
            ..event_stmts
          });
          let new_macro: Macro = parse2(quote! {
              echo! {
                  #transformed
              }
          })
          .unwrap();
          i.mac = new_macro;
        },
        _ => {},
      }
    }
  }

  fn visit_local_mut(&mut self, local: &mut syn::Local) {
    visit_mut::visit_local_mut(self, local);
    if let Some(LocalInit { expr, diverge: None, .. }) = &mut local.init {
      let mut name_visitor = NameVisitor::default();
      name_visitor.visit_pat(&local.pat);

      let names_token_stream =
        name_visitor.names.to_owned().into_iter().map(|x| match x {
          Some(ident) => quote! { Some(stringify!(#ident).to_string())},
          None => quote! { None },
        });
      let c = self.c.to_owned();

      if let Expr::Macro(ExprMacro {
        mac:
          Macro {
            path: Path { leading_colon, segments },
            tokens,
            ..
          },
        ..
      }) = (**expr).to_owned()
      {
        if let Some(ident) = into_ident(leading_colon, segments) {
          match ident.to_string().as_str() {
            "wire" => {
              let names = quote! { vec![#(#names_token_stream,)*]};
              let new_expr: Expr = parse2(quote! {
                  (#tokens).expr().with_name(#names).to(#c)
              })
              .unwrap();
              *expr = Box::new(new_expr);
            },
            "mut_wire" => {
              let name_prefix = if name_visitor.names.len() == 1 {
                if let Some(Some(ident)) =
                  name_visitor.names.to_owned().into_iter().next()
                {
                  quote! {#ident}
                } else {
                  ident.span().unwrap().error("instance should have one name").emit();
                  TokenStream::default()
                }
              } else {
                ident.span().unwrap().error("instance should have one name").emit();
                TokenStream::default()
              };

              let data_type: Expr = match parse2(tokens) {
                Ok(expr) => expr,
                Err(err) => {
                  err.span().unwrap().error("wire should have one argument").emit();
                  panic!()
                },
              };

              let new_expr: Expr = parse_quote! {
                {
                  let __i_ifc = #data_type;
                  let __i_ifc_impl_fields = __i_ifc.traverse().to(#c, Some(format!("{}", stringify!(#name_prefix))));
                  Wire {
                    i: #data_type,
                    o: #data_type.flip(),
                  }.flip().impl_with(IfcImplFields::Branch(vec![(format!("i"), __i_ifc_impl_fields.to_owned().flip()), (format!("o"), __i_ifc_impl_fields.to_owned())]))
                }
              };
              *expr = Box::new(new_expr);
            },
            "named" => {
              let names = quote! { vec![#(#names_token_stream,)*]};
              let new_expr: Expr = parse2(quote! {
                  (#tokens).expr().with_name(#names)
              })
              .unwrap();
              *expr = Box::new(new_expr);
            },
            // reg! is used for register instantiation
            "reg" => {
              let name_prefix = if name_visitor.names.len() == 1 {
                if let Some(Some(ident)) =
                  name_visitor.names.to_owned().into_iter().next()
                {
                  quote! {#ident}
                } else {
                  ident.span().unwrap().error("instance should have one name").emit();
                  TokenStream::default()
                }
              } else {
                ident.span().unwrap().error("instance should have one name").emit();
                TokenStream::default()
              };

              let mut token = match parse2(tokens) {
                Ok(PunctratedExprWrapper { punctuated }) => punctuated,
                Err(err) => {
                  err.span().unwrap().error("reg should have one argument").emit();
                  Punctuated::new()
                },
              }
              .into_iter();
              let data_type = token.next().expect("provide first argument as data_type");
              let clk = token.next().expect("provide second argument as clk");

              let new_expr: Expr = parse_quote! {
                {
                  let __reg_ifc = Reg {
                    w_port: #data_type,
                    r_port: #data_type.flip(), 
                  };

                  let __reg_ifc_impl_fields = __reg_ifc.traverse().to(#c, Some(format!("{}", stringify!(#name_prefix))));
                  let __reg = __reg_ifc.to_owned().impl_with(__reg_ifc_impl_fields.to_owned());

                  __reg.r_port.connect_expr(__reg.w_port.expr().reg(#clk), #c);

                  __reg_ifc.flip().impl_with(__reg_ifc_impl_fields.flip()).hold(#c)

                }
              };
              *expr = Box::new(new_expr);
            },
            "instance" => {
              let inst_name = if name_visitor.names.len() == 1 {
                if let Some(Some(ident)) =
                  name_visitor.names.to_owned().into_iter().next()
                {
                  quote! {#ident}
                } else {
                  ident.span().unwrap().error("instance should have one name").emit();
                  TokenStream::default()
                }
              } else {
                ident.span().unwrap().error("instance should have one name").emit();
                TokenStream::default()
              };
              let instance_token_stream =
                crate::instance::instance_decl(tokens, inst_name, c.to_owned());
              let new_expr: Expr = parse2(instance_token_stream).unwrap();
              *expr = Box::new(new_expr);
            },

            "event" => {
              let suggest_name = if name_visitor.names.len() == 1 {
                if let Some(Some(ident)) =
                  name_visitor.names.to_owned().into_iter().next()
                {
                  quote! {stringify!(#ident)}
                } else {
                  ident.span().unwrap().error("event should have one name").emit();
                  TokenStream::default()
                }
              } else {
                ident.span().unwrap().error("event should have one name").emit();
                TokenStream::default()
              };

              let event_stmts: EventStmts = parse2(tokens).unwrap();
              let event_name = match event_stmts.event_name.to_owned() {
                Some(event_name) => event_name,
                None => parse2(suggest_name).unwrap(),
              };

              let new_expr: Expr = parse2(event_transform(EventStmts {
                event_name: Some(event_name),
                c: Some(parse2(c).unwrap()),
                ..event_stmts
              }))
              .unwrap();
              *expr = Box::new(new_expr);
            },
            _ => {},
          }
        }
      }
    }
  }
}
