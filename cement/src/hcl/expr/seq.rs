use super::{Expr, ExprAst, ExprNode, ToExpr};
use crate::preclude::{Clk, SignalTrait, I, DataTypeTrait};

pub trait CompRegTrait<T: SignalTrait, CLK: ToExpr<Clk>>: ToExpr<T> + Sized {
  fn reg(self, clk: CLK) -> Expr<T> {
    let self_expr = self.expr();
    Expr {
      ifc: self_expr.ifc.to_owned(),
      ast: ExprAst::Branch(
        ExprNode::Reg,
        vec![self_expr.ast, clk.expr().ast],
        None,
        self_expr.ifc.traverse(),
      ),
    }
  }
}

impl<T: SignalTrait, CLK: ToExpr<Clk>> CompRegTrait<T, CLK> for Expr<T> {}
impl<T: DataTypeTrait, CLK: ToExpr<Clk>> CompRegTrait<T, CLK> for I<T> {}
