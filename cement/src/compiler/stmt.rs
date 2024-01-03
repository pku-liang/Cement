use std::panic::Location;

use irony_cmt::{ArrayAttr, EntityId, Environ, StmtSynth, StringAttr};

use super::Cmtc;
use crate::preclude::{Stmt, StmtProtocol};

pub trait CmtcStmt {
  fn synthesize<P: StmtProtocol>(&mut self, stmt: Stmt, protocol: P);
  fn add_stmt(&mut self, name: Option<String>) -> EntityId;
}

impl CmtcStmt for Cmtc {
  fn synthesize<P: StmtProtocol>(&mut self, stmt: Stmt, protocol: P) {
    let stmt_entity_id = stmt.to(self);

    let clk = protocol.clk();
    let protocol_entity_id = protocol.v_event();
    let protocol_names = protocol.v_name();

    self.ir.add_op(
      StmtSynth::new(
        Some(stmt_entity_id),
        clk,
        protocol_entity_id.into_iter().map(|x| Some(x.entity_id)).collect(),
        Some(ArrayAttr(
          protocol_names.into_iter().map(|x| StringAttr(x).into()).collect(),
        )),
      )
      .into(),
    );
  }

  #[track_caller]
  fn add_stmt(&mut self, name: Option<String>) -> EntityId {
    let raw_name = name.or(Some("stmt".to_string())).unwrap();
    let legal_name = self.symbol_table.get_legal_name_in_region(&self.ir, &raw_name);
    let entity_id = self.ir.add_entity(
      irony_cmt::IRStmt::new(
        Some(irony_cmt::DataTypeEnum::Void),
        Some(legal_name.into()),
        Some(self.config.debug.into()),
        Some(Location::caller().into()),
      )
      .into(),
    );
    entity_id
  }
}
