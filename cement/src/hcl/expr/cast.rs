use super::{Cast, Expr, ExprNode, ToExpr};
use crate::preclude::{DataTypeTrait, I};

pub trait CastOp<S: DataTypeTrait, T: DataTypeTrait>: ToExpr<S> + Sized {
  fn cast(self, target: T) -> Expr<T> {
    Expr {
      ifc: target,
      ast: super::ExprAst::Branch(
        ExprNode::Cast(Cast { target_data_type: target.ir_type() }),
        vec![self.expr().ast],
        None,
        target.traverse(),
      ),
    }
  }
}

impl<S: DataTypeTrait, T: DataTypeTrait> CastOp<S, T> for I<S> {}
impl<S: DataTypeTrait, T: DataTypeTrait> CastOp<S, T> for Expr<S> {}
