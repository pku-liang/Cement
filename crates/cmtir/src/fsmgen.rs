//! `cmtir`'s operation creation support for FSMGEN (generating FSMs from
//! procedural descriptions)primitives, including seq, par, branch, for, ...

use super::*;

impl ir::Op {
  /// Convert an `ir::Op` to a single step operation.
  pub fn as_step(self) -> ir::Op {
    ir::Op {
      inner: OpEnum::Step(Box::new(BodyOp { ops: vec![self] })),
      annotations: json::object::Object::new(),
    }
  }

  /// Create a step operation with multiple `ir::Op`s, which will be executed
  /// simultaneously.
  pub fn step(ops: Vec<ir::Op>) -> ir::Op {
    ir::Op {
      inner: OpEnum::Step(Box::new(BodyOp { ops })),
      annotations: json::object::Object::new(),
    }
  }

  /// Create a seq operation with multiple `ir::Op`s, which will be
  /// executed sequentially (one after another).
  pub fn seq(ops: Vec<ir::Op>) -> ir::Op {
    ir::Op {
      inner: OpEnum::Seq(Box::new(BodyOp { ops })),
      annotations: json::object::Object::new(),
    }
  }

  /// Create a par operation with multiple `ir::Op`s, which will be executed
  /// in parallel (fork-join).
  pub fn par(ops: Vec<ir::Op>) -> ir::Op {
    ir::Op {
      inner: OpEnum::Par(Box::new(ParOp { ops })),
      annotations: json::object::Object::new(),
    }
  }

  /// Create a branch operation with a condition `ir::Op` and two branches (then and else).
  pub fn branch(
    cond: ir::Op,
    then_body: ir::Op,
    else_body: Option<ir::Op>,
    loose: bool,
  ) -> ir::Op {
    ir::Op {
      inner: OpEnum::Branch(Box::new(BranchOp {
        cond,
        then_body,
        else_body,
        loose,
      })),
      annotations: json::object::Object::new(),
    }
  }

  /// Create a for-loose operation with a condition `ir::Op` and a body `ir::Op`.
  /// `loose` means the loop body's execution will have additional delay for
  /// each iteration to conduct `update` operation and decide whether to continue.
  pub fn for_loose(
    init: Option<ir::Op>,
    cond: ir::Op,
    update: ir::Op,
    body: ir::Op,
  ) -> ir::Op {
    ir::Op {
      inner: OpEnum::For(Box::new(ForOp {
        init,
        init_cond: None,
        cond,
        update,
        body,
        loose: true,
      })),
      annotations: json::object::Object::new(),
    }
  }

  /// Create a for-tight operation with a condition `ir::Op` and a body `ir::Op`.
  /// `tight` means the loop body's execution **will** not have additional delay for
  /// each iteration. Instead, the explicit `update_cond` will be used to decide whether to continue.
  pub fn for_tight(
    init: Option<ir::Op>,
    init_cond: ir::Op,
    update: ir::Op,
    update_cond: ir::Op,
    body: ir::Op,
  ) -> ir::Op {
    ir::Op {
      inner: OpEnum::For(Box::new(ForOp {
        init,
        init_cond: Some(init_cond),
        cond: update_cond,
        update,
        body,
        loose: false,
      })),
      annotations: json::object::Object::new(),
    }
  }
}
