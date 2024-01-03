use irony_cmt::DataTypeEnum;

use super::*;
use crate::preclude::{
  DataTypeTrait, Event, ImplFields, InterfaceImpl, SignalTrait, B, I,
};

pub trait Cmp {}

impl<Signal: SignalTrait> Cmp for Signal {}

pub fn cmpi<T: SignalTrait + Cmp>(
  variant: Cmpi, op0: Expr<T>, op1: Expr<T>,
) -> Expr<B<1>> {
  Expr {
    ifc: B::<1>,
    ast: ExprAst::Branch(
      ExprNode::Cmpi(variant),
      vec![op0.ast, op1.ast],
      None,
      B::<1>.traverse(),
    ),
  }
}

pub trait CmpiOp<T: SignalTrait + Cmp, R: ToExpr<T>>: Sized + ToExpr<T> {
  fn eq(self, rhs: R) -> Expr<B<1>> { cmpi(Cmpi::Eq, self.expr(), rhs.expr()) }

  fn ne(self, rhs: R) -> Expr<B<1>> { cmpi(Cmpi::Ne, self.expr(), rhs.expr()) }

  fn lt(self, rhs: R) -> Expr<B<1>> { cmpi(Cmpi::Lt, self.expr(), rhs.expr()) }

  fn le(self, rhs: R) -> Expr<B<1>> { cmpi(Cmpi::Le, self.expr(), rhs.expr()) }

  fn gt(self, rhs: R) -> Expr<B<1>> { cmpi(Cmpi::Gt, self.expr(), rhs.expr()) }

  fn ge(self, rhs: R) -> Expr<B<1>> { cmpi(Cmpi::Ge, self.expr(), rhs.expr()) }
}

impl<T: DataTypeTrait + Cmp> CmpiOp<T, I<T>> for I<T> {}
impl<T: DataTypeTrait + Cmp> CmpiOp<T, Expr<T>> for I<T> {}
impl<T: DataTypeTrait + Cmp> CmpiOp<T, I<T>> for Expr<T> {}
impl<T: SignalTrait + Cmp> CmpiOp<T, Expr<T>> for Expr<T> {}

pub trait MuxOp<T: Interface, R0: ToExpr<T>, R1: ToExpr<T>>:
  Sized + ToExpr<B<1>>
{
  fn mux(self, t: R0, e: R1) -> Expr<T> {
    let (t, e) = (t.expr(), e.expr());
    let self_expr = self.expr();
    // println!("self.expr() is {:?}", self_expr.ast);
    Expr {
      ifc: t.ifc.to_owned(),
      ast: ExprAst::Branch(
        ExprNode::Mux,
        vec![self_expr.ast, t.ast, e.ast],
        None,
        t.ifc.traverse(),
      ),
    }
  }
}

impl<T: InterfaceImpl> MuxOp<T::IfcT, T, T> for Event {}
impl<T: InterfaceImpl> MuxOp<T::IfcT, T, Expr<T::IfcT>> for Event {}
impl<T: InterfaceImpl> MuxOp<T::IfcT, Expr<T::IfcT>, T> for Event {}
impl<T: Interface> MuxOp<T, Expr<T>, Expr<T>> for Event {}

impl<T: InterfaceImpl> MuxOp<T::IfcT, T, T> for I<B<1>> {}
impl<T: InterfaceImpl> MuxOp<T::IfcT, T, Expr<T::IfcT>> for I<B<1>> {}
impl<T: InterfaceImpl> MuxOp<T::IfcT, Expr<T::IfcT>, T> for I<B<1>> {}
impl<T: Interface> MuxOp<T, Expr<T>, Expr<T>> for I<B<1>> {}

impl<T: InterfaceImpl> MuxOp<T::IfcT, T, T> for Expr<B<1>> {}
impl<T: InterfaceImpl> MuxOp<T::IfcT, T, Expr<T::IfcT>> for Expr<B<1>> {}
impl<T: InterfaceImpl> MuxOp<T::IfcT, Expr<T::IfcT>, T> for Expr<B<1>> {}
impl<T: Interface> MuxOp<T, Expr<T>, Expr<T>> for Expr<B<1>> {}

pub fn select<T: SignalTrait>(
  onehot: bool, conds: Vec<Option<Expr<B<1>>>>, values: Vec<Expr<T>>,
  default: Option<Expr<T>>,
) -> Expr<T> {
  assert_eq!(conds.len(), values.len());
  let ifc =
    values.to_owned().into_iter().next().expect("at least one value for select").ifc;
  let ifc_fields = ifc.traverse();
  Expr {
    ifc,
    ast: ExprAst::Branch(
      ExprNode::Select(onehot),
      conds
        .into_iter()
        .map(|c| match c {
          Some(c) => c.ast,
          None => ExprAst::Leaf(
            FBFields {
              fwd: ImplFields {
                v_name: vec![format!("TBD")],
                v_data_type: vec![DataTypeEnum::UInt(1.into())],
                v_entity_id: vec![None],
              },
              bwd: ImplFields::new(vec![], vec![], vec![]),
            },
            None,
          ),
        })
        .chain(values.into_iter().map(|e| e.ast))
        .chain(default.into_iter().map(|e| e.ast))
        .collect(),
      None,
      ifc_fields,
    ),
  }
}

pub trait CaseOp<D: SignalTrait, T: SignalTrait, R: ToExpr<D>, V: ToExpr<T>>:
  CmpiOp<D, R> + Sized + Clone
{
  fn case(self, onehot: bool, cases: Vec<(R, V)>, default: Option<V>) -> Expr<T> {
    let mut conds = vec![];
    let mut values = vec![];
    for (cond, value) in cases {
      conds.push(Some(self.to_owned().eq(cond)));
      values.push(value.expr());
    }
    select(onehot, conds, values, default.map(|x| x.expr()))
  }
}

impl<D: SignalTrait, T: SignalTrait, R: ToExpr<D>, V: ToExpr<T>, SELF> CaseOp<D, T, R, V>
  for SELF
where SELF: CmpiOp<D, R> + Sized + Clone
{
}
