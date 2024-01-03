
use super::*;
use crate::preclude::{
  Connect, InterfaceImpl, I, O, DataTypeTrait, Flip,
};

impl<T: InterfaceImpl> ToExpr<T::IfcT> for T {
  fn expr(&self) -> Expr<T::IfcT> {
    // println!("ImplFields: {:?}", self.traverse());
    let fb_fields = self.traverse().split();
    Expr {
      ifc: self.ifc(),
      ast: ExprAst::Leaf(fb_fields, None),
    }
  }
}

pub trait ConnectExpr<T: Interface>: Connect<T::ImplT> {
  #[track_caller]
  fn connect_expr(self, target_expr: Expr<T>, c: &mut Cmtc)
  where Self: Sized {
    self.connect(target_expr.to(c), c)
  }
}

impl<T: DataTypeTrait+Interface<FlipT = Flip<T>, ImplT = I<T>>> ConnectExpr<T> for O<T>  {}
impl<T: DataTypeTrait+Interface<FlipT = Flip<T>, ImplT = I<T>>> ConnectExpr<Flip<T>> for I<T> {}
