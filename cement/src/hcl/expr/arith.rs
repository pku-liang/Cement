use super::{Concat, Expr, ExprAst, ExprNode, Extract, ToExpr};
use crate::preclude::{paste, Bits, Interface, B, I, DataTypeTrait};

pub trait SignlessInteger: DataTypeTrait {}

impl<const N: usize> SignlessInteger for B<N> {}
impl SignlessInteger for Bits {}

pub trait SignlessIntegerOp<S: SignlessInteger, T: SignlessInteger>:
  ToExpr<S> + Sized + Clone
{
  fn extract(&self, low: u32, target_data_type: T) -> Expr<T> {
    let self_expr = self.to_owned().expr();
    Expr {
      ifc: target_data_type.to_owned(),
      ast: ExprAst::Branch(
        ExprNode::Extract(Extract {
          low,
          target_data_type: target_data_type.v_ir_type()[0].to_owned(),
        }),
        vec![self_expr.ast],
        None,
        target_data_type.traverse(),
      ),
    }
  }
}

impl<S: SignlessInteger, T: SignlessInteger> SignlessIntegerOp<S, T> for Expr<S> {}
impl<S: SignlessInteger, T: SignlessInteger> SignlessIntegerOp<S, T> for I<S> {}

macro_rules! impl_biop {
  ($kind_variant:ident, $trait:ident, $fn:ident) => {
    impl<Signal: SignlessInteger> std::ops::$trait<I<Signal>> for I<Signal> {
      type Output = Expr<Signal>;

      fn $fn(self, rhs: I<Signal>) -> Self::Output { self.expr().$fn(rhs.expr()) }
    }

    impl<Signal: SignlessInteger> std::ops::$trait<I<Signal>> for Expr<Signal> {
      type Output = Expr<Signal>;

      fn $fn(self, rhs: I<Signal>) -> Self::Output { self.$fn(rhs.expr()) }
    }
    impl<Signal: SignlessInteger> std::ops::$trait<Expr<Signal>> for I<Signal> {
      type Output = Expr<Signal>;

      fn $fn(self, rhs: Expr<Signal>) -> Self::Output { self.expr().$fn(rhs) }
    }

    impl<Signal: SignlessInteger> std::ops::$trait<Expr<Signal>> for Expr<Signal> {
      type Output = Expr<Signal>;

      fn $fn(self, rhs: Expr<Signal>) -> Self::Output {
        Expr {
          ifc: self.ifc.to_owned(),
          ast: super::ExprAst::Branch(
            super::ExprNode::$kind_variant(super::$kind_variant::$trait),
            vec![self.ast, rhs.ast],
            None,
            self.ifc.traverse(),
          ),
        }
      }
    }
  };
}

impl_biop!(Variadic, BitAnd, bitand);
impl_biop!(Variadic, BitOr, bitor);
impl_biop!(Variadic, BitXor, bitxor);
impl_biop!(Variadic, Add, add);

impl_biop!(Binary, Shr, shr);
impl_biop!(Binary, Shl, shl);
impl_biop!(Binary, Sub, sub);

macro_rules! impl_unaop {
  ($kind_variant:ident, $trait:ident, $fn:ident) => {
    impl<Signal: SignlessInteger> std::ops::$trait for I<Signal> {
      type Output = Expr<Signal>;

      fn $fn(self) -> Self::Output { self.expr().$fn() }
    }

    impl<Signal: SignlessInteger> std::ops::$trait for Expr<Signal> {
      type Output = Expr<Signal>;

      fn $fn(self) -> Self::Output {
        Expr {
          ifc: self.ifc.to_owned(),
          ast: super::ExprAst::Branch(
            super::ExprNode::$kind_variant(super::$kind_variant::$trait),
            vec![self.ast],
            None,
            self.ifc.traverse(),
          ),
        }
      }
    }
  };
}

impl_unaop!(Unary, Not, not);
impl_unaop!(Unary, Neg, neg);

macro_rules! impl_tuple_concat {
    ($n:expr => $($ty:ident : $id:tt),*) => {
        paste! {
            pub trait [<ConcatTuple $n>]<$($ty : SignlessInteger,)*> {
                fn concat(self) -> Expr<Bits>;
            }
            impl<$($ty : SignlessInteger,)* $([<E $id>]: ToExpr<$ty>,)*> [<ConcatTuple $n>]<$($ty,)*> for ($([<E $id>],)*) {
                fn concat(self) -> Expr<Bits> {
                    let mut ast = vec![];
                    let mut len = 0;

                    $(
                        let expr = self.$id.expr();
                        ast.push(expr.ast);
                        len += expr.ifc.total_width();
                    )*

                    Expr {
                        ifc: Bits(len),
                        ast: ExprAst::Branch(
                            ExprNode::Concat(Concat{}),
                            ast,
                            None,
                            Bits(len).traverse(),
                        ),
                    }
                }
            }
        }
    }
}

impl_tuple_concat!(2 => T0:0, T1:1);
impl_tuple_concat!(3 => T0:0, T1:1, T2:2);
impl_tuple_concat!(4 => T0:0, T1:1, T2:2, T3:3);
impl_tuple_concat!(5 => T0:0, T1:1, T2:2, T3:3, T4:4);
impl_tuple_concat!(6 => T0:0, T1:1, T2:2, T3:3, T4:4, T5:5);
impl_tuple_concat!(7 => T0:0, T1:1, T2:2, T3:3, T4:4, T5:5, T6:6);
impl_tuple_concat!(8 => T0:0, T1:1, T2:2, T3:3, T4:4, T5:5, T6:6, T7:7);
