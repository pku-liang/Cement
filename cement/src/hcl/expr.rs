use super::{
  default_names_from_ifc_fields, FBFields, FBNames, IfcFields, Interface,
};
use crate::compiler::Cmtc;

mod expr_node;
pub use expr_node::*;

#[derive(Clone)]
pub struct Expr<Ifc: Interface> {
  pub ifc: Ifc,
  pub ast: ExprAst,
}

impl<Ifc: Interface> Expr<Ifc> {
  #[track_caller]
  pub fn to(self, c: &mut Cmtc) -> Ifc::ImplT {
    // println!();

    // println!("self.ast: {:?}", self.ast);

    let fb_fields = self.ast.eval(c);

    // println!("fb_fields: {:?}", fb_fields);
    // println!("ifc: {:?}", self.ifc);
    // println!();

    let ifc_impl_fields = self.ifc.traverse().with_fb_fields(fb_fields);

    self.ifc.impl_with(ifc_impl_fields)
  }

  pub fn with_name(self, fb_names: FBNames) -> Self {
    Self {
      ifc: self.ifc,
      ast: self.ast.with_name(fb_names),
    }
  }

  pub fn with_prefix(self, prefix: String) -> Self {
    let Expr { ifc, ast } = self;
    let names = ifc.traverse().names(Some(prefix));

    Expr {
      ifc: ifc.to_owned(),
      ast: ast.with_name(names),
    }
  }
}

#[derive(Debug, Clone)]
pub enum ExprAst {
  Leaf(FBFields, Option<FBNames>),
  Branch(ExprNode, Vec<ExprAst>, Option<FBNames>, IfcFields),
}

impl ExprAst {
  #[track_caller]
  pub fn eval(self, c: &mut Cmtc) -> FBFields {
    match self {
      ExprAst::Leaf(fb_fields, option_fb_names) => {
        let None = option_fb_names else { panic!("Leaf should not have new names") };
        // println!("leaf, fb_fields: {:?}", fb_fields);
        fb_fields
      },
      ExprAst::Branch(expr_node, v_expr_ast, option_fb_names, ifc_fields) => {
        let mut operands = Vec::new();
        for expr_ast in v_expr_ast {
          operands.push(expr_ast.eval(c));
        }
        let fb_fields = expr_node.eval(c, operands, match option_fb_names {
          Some(fb_names) => fb_names,
          None => default_names_from_ifc_fields(ifc_fields),
        });

        // println!("branch, fb_fields: {:?}", fb_fields);

        fb_fields
      },
    }
  }

  pub fn with_name(self, fb_names: FBNames) -> Self {
    match self {
      ExprAst::Leaf(fb_fields, _) => ExprAst::Leaf(fb_fields, Some(fb_names)),
      ExprAst::Branch(expr_node, v_expr_ast, _, ifc_fields) => {
        ExprAst::Branch(expr_node, v_expr_ast, Some(fb_names), ifc_fields)
      },
    }
  }
}

pub trait ToExpr<Ifc: Interface> {
  fn expr(&self) -> Expr<Ifc>;
}

impl<Ifc: Interface> ToExpr<Ifc> for Expr<Ifc> {
  fn expr(&self) -> Expr<Ifc> { self.to_owned() }
}

mod connection;
pub use connection::*;

mod seq;
pub use seq::*;

mod arith;
pub use arith::*;

mod cond;
pub use cond::*;

mod cast;
pub use cast::*;

mod array;
pub use array::*;