use std::any::Any;

use irony_cmt::OpId;

use crate::preclude::InterfaceImpl;

#[derive(Debug, Default)]
pub struct ModuleStack {
  ifc_impl_stack: Vec<Box<dyn Any>>,
  op_id_stack: Vec<OpId>,
}

impl ModuleStack {
  pub fn current_module(&self) -> Option<OpId> {
    self.op_id_stack.last().copied()
  }

  pub fn push<T: InterfaceImpl + 'static>(&mut self, ii: T, op_id: OpId) {
    self.ifc_impl_stack.push(Box::new(ii));
    self.op_id_stack.push(op_id);
  }

  pub fn pop<T: InterfaceImpl + 'static>(&mut self) -> T {
    self.op_id_stack.pop();
    *self.ifc_impl_stack.pop().unwrap().downcast::<T>().unwrap()
  }
}
