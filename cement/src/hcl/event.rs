use irony_cmt::EntityId;

use super::{Expr, ExprAst, ExprNode, Interface, InterfaceImpl, ToExpr, B, B1};
use crate::preclude::{Cmtc, CmtcEvent};

#[derive(Clone)]
pub struct Event {
  pub entity_id: EntityId,
  pub name: String,
}

impl Event {
  pub fn attach<T: ToExpr<B<1>>>(&self, t: T, c: &mut Cmtc) {
    c.specify_event_eq_signal(self, t);
  }

  pub fn expr(&self) -> Expr<B<1>> {
    Expr {
      ifc: B1,
      ast: ExprAst::Branch(
        ExprNode::EventToSignal(self.entity_id.to_owned()),
        Vec::new(),
        None,
        B1.traverse(),
      ),
    }
    .with_name(vec![Some(self.name.to_owned())])
  }
}

impl ToExpr<B<1>> for Event {
  fn expr(&self) -> Expr<B<1>> {
    Expr {
      ifc: B1,
      ast: ExprAst::Branch(
        ExprNode::EventToSignal(self.entity_id.to_owned()),
        Vec::new(),
        None,
        B1.traverse(),
      ),
    }
  }
}

pub struct Guarded<T: InterfaceImpl> {
  pub wire: T,
  pub guard: Event,
}
