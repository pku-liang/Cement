use super::*;
use crate::preclude::*;
pub trait ArrCreateOp<T: DataTypeTrait, const N: usize> {
  fn ac(self) -> Expr<Arr<N, T>>;
}

impl<T: DataTypeTrait, D: ToExpr<T> + Clone, const N: usize> ArrCreateOp<T, N>
  for [D; N]
{
  fn ac(self) -> Expr<Arr<N, T>> {
    let data_type = self.first().unwrap().to_owned().expr().ifc;
    Expr {
      ifc: Arr::<N, T>(data_type),
      ast: ExprAst::Branch(
        ExprNode::ArrayCreate,
        self.iter().map(|d| d.to_owned().expr().ast).collect(),
        None,
        Arr::<N, T>(data_type).traverse(),
      ),
    }
  }
}

pub trait ArrayCreateOp<T: DataTypeTrait>: Clone {
  fn ac(self) -> Expr<Array<T>>;
}

impl<T: DataTypeTrait, D: ToExpr<T> + Clone> ArrayCreateOp<T> for Vec<D> {
  fn ac(self) -> Expr<Array<T>> {
    let data_type = self.first().unwrap().to_owned().expr().ifc;
    Expr {
      ifc: Array::<T>(self.len(), data_type),
      ast: ExprAst::Branch(
        ExprNode::ArrayCreate,
        self.iter().map(|d| d.to_owned().expr().ast).collect(),
        None,
        Array::<T>(self.len(), data_type).traverse(),
      ),
    }
  }
}

pub trait ArrConcatOp<
  T: DataTypeTrait,
  const N: usize,
  const M: usize,
  R: ToExpr<Arr<N, T>> + 'static,
>: ToExpr<Arr<M, T>> + Sized + 'static
{
  fn concat(self, rhs: R) -> Expr<Arr<{ N + M }, T>> {
    let self_expr = self.expr();
    let data_type = self_expr.ifc.0.to_owned();
    Expr {
      ifc: Arr::<{ N + M }, T>(data_type),
      ast: ExprAst::Branch(
        ExprNode::ArrayConcat,
        vec![self_expr.ast, rhs.expr().ast],
        None,
        Arr::<{ N + M }, T>(data_type).traverse(),
      ),
    }
  }
}

impl<T: DataTypeTrait, const N: usize, const M: usize, R: ToExpr<Arr<N, T>> + 'static>
  ArrConcatOp<T, N, M, R> for Expr<Arr<M, T>>
{
}

impl<T: DataTypeTrait, const N: usize, const M: usize, R: ToExpr<Arr<N, T>> + 'static>
  ArrConcatOp<T, N, M, R> for I<Arr<M, T>>
{
}

pub trait ArrayConcatOp<T: DataTypeTrait, R: ToExpr<Array<T>>>:
  ToExpr<Array<T>> + Sized
{
  fn concat(self, rhs: R) -> Expr<Array<T>> {
    let self_expr = self.expr();
    let data_type = self_expr.ifc.1.to_owned();
    Expr {
      ifc: Array::<T>(self_expr.ifc.0, data_type),
      ast: ExprAst::Branch(
        ExprNode::ArrayConcat,
        vec![self_expr.ast, rhs.expr().ast],
        None,
        Array::<T>(self_expr.ifc.0, data_type).traverse(),
      ),
    }
  }
}

impl<T: DataTypeTrait, R: ToExpr<Array<T>>> ArrayConcatOp<T, R> for Expr<Array<T>> {}

impl<T: DataTypeTrait, R: ToExpr<Array<T>>> ArrayConcatOp<T, R> for I<Array<T>> {}

pub trait ArrSliceOp<
  T: SignalTrait,
  START: ToExpr<T>,
  const N: usize,
  const M: usize,
  ST: DataTypeTrait,
>: ToExpr<Arr<N, ST>> + Sized
{
  fn slice(self, start: START, len: usize) -> Expr<Arr<M, ST>> {
    assert!(M <= N);
    assert!(M == len);

    let self_expr = self.expr();
    let data_type = self_expr.ifc.0;
    Expr {
      ifc: Arr::<M, ST>(data_type),
      ast: ExprAst::Branch(
        ExprNode::ArraySlice(M),
        vec![self_expr.ast, start.expr().ast],
        None,
        Arr::<M, ST>(data_type).traverse(),
      ),
    }
  }
}

impl<
    T: SignalTrait,
    START: ToExpr<T>,
    const N: usize,
    const M: usize,
    ST: DataTypeTrait,
  > ArrSliceOp<T, START, N, M, ST> for Expr<Arr<N, ST>>
{
}

impl<
    T: SignalTrait,
    START: ToExpr<T>,
    const N: usize,
    const M: usize,
    ST: DataTypeTrait,
  > ArrSliceOp<T, START, N, M, ST> for I<Arr<N, ST>>
{
}

pub trait ArraySliceOp<T: SignalTrait, START: ToExpr<T>, ST: DataTypeTrait>:
  ToExpr<Array<ST>> + Sized
{
  fn slice(self, start: START, len: usize) -> Expr<Array<ST>> {
    let self_expr = self.expr();
    let data_type = self_expr.ifc.1;
    Expr {
      ifc: Array(len, data_type),
      ast: ExprAst::Branch(
        ExprNode::ArraySlice(len),
        vec![self_expr.ast, start.expr().ast],
        None,
        Array(len, data_type).traverse(),
      ),
    }
  }
}

impl<T: SignalTrait, START: ToExpr<T>, ST: DataTypeTrait> ArraySliceOp<T, START, ST>
  for Expr<Array<ST>>
{
}

impl<T: SignalTrait, START: ToExpr<T>, ST: DataTypeTrait> ArraySliceOp<T, START, ST>
  for I<Array<ST>>
{
}

pub trait ArrGetOp<T: SignalTrait, IDX: ToExpr<T>, const N: usize, ST: DataTypeTrait>:
  ToExpr<Arr<N, ST>> + Sized
{
  fn get(self, idx: IDX) -> Expr<ST> {
    let self_expr = self.expr();
    let data_type = self_expr.ifc.0;
    Expr {
      ifc: data_type,
      ast: ExprAst::Branch(
        ExprNode::ArrayGet,
        vec![self_expr.ast, idx.expr().ast],
        None,
        data_type.traverse(),
      ),
    }
  }
}

impl<T: SignalTrait, START: ToExpr<T>, const N: usize, ST: DataTypeTrait>
  ArrGetOp<T, START, N, ST> for Expr<Arr<N, ST>>
{
}
impl<T: SignalTrait, START: ToExpr<T>, const N: usize, ST: DataTypeTrait>
  ArrGetOp<T, START, N, ST> for I<Arr<N, ST>>
{
}
pub trait ArrayGetOp<T: SignalTrait, IDX: ToExpr<T>, ST: DataTypeTrait>:
  ToExpr<Array<ST>> + Sized
{
  fn get(self, idx: IDX) -> Expr<ST> {
    let self_expr = self.expr();
    let data_type = self_expr.ifc.1;
    Expr {
      ifc: data_type,
      ast: ExprAst::Branch(
        ExprNode::ArrayGet,
        vec![self_expr.ast, idx.expr().ast],
        None,
        data_type.traverse(),
      ),
    }
  }
}

impl<T: SignalTrait, START: ToExpr<T>, ST: DataTypeTrait> ArrayGetOp<T, START, ST>
  for Expr<Array<ST>>
{
}
impl<T: SignalTrait, START: ToExpr<T>, ST: DataTypeTrait> ArrayGetOp<T, START, ST>
  for I<Array<ST>>
{
}
