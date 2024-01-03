use super::Event;
use crate::preclude::{Cmtc, CmtcBasics, CmtcStmt};

mod protocol;
use irony_cmt::{EntityId, StmtFor, StmtIf, StmtSeq, StmtStep, StmtWhile};
pub use protocol::*;

pub struct Stmt {
  pub name: Option<String>,
  pub ast: StmtAst,
}

impl Stmt {
  #[track_caller]
  pub fn to(self, c: &mut Cmtc) -> EntityId {
    match self.ast {
      StmtAst::Step(StepStmt { events, wait_at_exit }) => {
        let entity_id = c.add_stmt(self.name);
        c.add_op(
          StmtStep::new(
            Some(entity_id.to_owned()),
            events.into_iter().map(|x| Some(x.entity_id)).collect(),
            wait_at_exit.into_iter().map(|x| Some(x.entity_id)).collect(),
          )
          .into(),
        );
        entity_id
      },
      StmtAst::Seq(SeqStmt { stmts }) => {
        let entity_id = c.add_stmt(self.name);
        let mut stmts_entity_ids = Vec::new();
        for stmt in stmts {
          stmts_entity_ids.push(stmt.to(c));
        }
        c.add_op(StmtSeq::new(Some(entity_id.to_owned()), stmts_entity_ids.into_iter().map(|x| Some(x)).collect()).into());
        entity_id
      },
      StmtAst::If(IfStmt { cond, then_stmt, else_stmt }) => {
        let entity_id = c.add_stmt(self.name);
        let cond_entity_id = cond.entity_id;
        let then_entity_id = then_stmt.to(c);
        let else_entity_id = else_stmt.map(|x| x.to(c));
        c.add_op(
          StmtIf::new(
            Some(entity_id.to_owned()),
            Some(cond_entity_id),
            Some(then_entity_id),
            else_entity_id,
          )
          .into(),
        );
        entity_id
      },
      StmtAst::For(ForStmt {
        indvar_rd,
        indvar_wr,
        start,
        end,
        incr,
        step,
        do_stmt,
      }) => {
        let entity_id = c.add_stmt(self.name);
        let (start, const_start) = match start {
          Bound::Const(x) => (None, Some(x.into())),
          Bound::Var(x) => (Some(x), None),
        };
        let (end, const_end) = match end {
          Bound::Const(x) => (None, Some(x.into())),
          Bound::Var(x) => (Some(x), None),
        };
        let do_entity_id = do_stmt.to(c);
        c.add_op(
          StmtFor::new(
            Some(entity_id.to_owned()),
            Some(indvar_rd),
            Some(indvar_wr),
            Some(do_entity_id),
            start,
            end,
            Some(incr.into()),
            const_start,
            const_end,
            Some(step.into()),
          )
          .into(),
        );
        entity_id
      },
      StmtAst::While(WhileStmt { cond, do_stmt }) => {
        let entity_id = c.add_stmt(self.name);
        let cond_entity_id = cond.entity_id;
        let do_entity_id = do_stmt.to(c);
        c.add_op(
          StmtWhile::new(
            Some(entity_id.to_owned()),
            Some(cond_entity_id),
            Some(do_entity_id),
          )
          .into(),
        );
        entity_id
      },

      StmtAst::Par(ParStmt { stmts }) => {
        let entity_id = c.add_stmt(self.name);
        let mut stmts_entity_ids = Vec::new();
        for stmt in stmts {
          stmts_entity_ids.push(stmt.to(c));
        }
        c.add_op(StmtSeq::new(Some(entity_id.to_owned()), stmts_entity_ids.into_iter().map(|x| Some(x)).collect()).into());
        entity_id
      },
    }
  }
}

pub enum StmtAst {
  Step(StepStmt),
  Seq(SeqStmt),
  If(IfStmt),
  For(ForStmt),
  While(WhileStmt),
  Par(ParStmt),
}

pub struct StepStmt {
  pub events: Vec<Event>,
  pub wait_at_exit: Vec<Event>,
}

pub struct SeqStmt {
  pub stmts: Vec<Stmt>,
}
pub struct ParStmt {
  pub stmts: Vec<Stmt>,
}

pub struct IfStmt {
  pub cond: Event,
  pub then_stmt: Box<Stmt>,
  pub else_stmt: Option<Box<Stmt>>,
}

pub enum Bound {
  Const(usize),
  Var(EntityId),
}

pub struct ForStmt {
  pub indvar_rd: EntityId,
  pub indvar_wr: EntityId,
  pub start: Bound,
  pub end: Bound,
  pub incr: bool,
  pub step: usize,
  pub do_stmt: Box<Stmt>,
}

pub struct WhileStmt {
  pub cond: Event,
  pub do_stmt: Box<Stmt>,
}

#[macro_export]
macro_rules! stmt {

    // Match for Seq statement
    (seq { $({$($stmt:tt)*})+ }) => {
        Stmt {
            name: Some("seq".to_string()),
            ast: StmtAst::Seq(SeqStmt {
                stmts: vec![$(stmt!($($stmt)*)),*],
            })
        }
    };
    // Match for Par statement
    (par { $({$($stmt:tt)*})+}) => {
        Stmt {
            name: Some("par".to_string()),
            ast: StmtAst::Par(ParStmt {
                stmts: vec![$(stmt!($($stmt)*)),*],
            })
        }
    };
    // Match for If statement
    (if $cond:expr => $then_stmt:tt $(else $else_stmt:tt)* ) => {
        Stmt {
            name: Some("if".to_string()),
            ast: StmtAst::If(IfStmt {
                cond: $cond,
                then_stmt: Box::new(stmt!($then_stmt)),
                else_stmt: $(Some(Box::new(stmt!($else_stmt))))?,
            })
        }
    };
    // Match for For statement
    (for $indvar_rd:expr, $indvar_wr:expr, $start:expr, $end:expr, $incr:expr, $step:expr => $($do_stmt:tt)* ) => {
        Stmt {
            name: Some("for".to_string()),
            ast: StmtAst::For(ForStmt {
                indvar_rd: $indvar_rd,
                indvar_wr: $indvar_wr,
                start: $start,
                end: $end,
                incr: $incr,
                step: $step,
                do_stmt: Box::new(stmt!($($do_stmt)*)),
            })
        }
    };
    // Match for While statement
    (while $cond:expr => $($do_stmt:tt)*) => {
        Stmt {
            name: Some("while".to_string()),
            ast: StmtAst::While(WhileStmt {
                cond: $cond,
                do_stmt: Box::new(stmt!($($do_stmt)*)),
            })
        }
    };
    // Terminal case for an individual statement
    ($($stmt:expr),* $(; [$($exit:tt)*])?) => {
        Stmt {
            name: Some("step".to_string()),
            ast: StmtAst::Step(StepStmt {
                events: vec![$($stmt),*],
                wait_at_exit: vec![$($($exit)*)?],
            })
        }
    };
}
