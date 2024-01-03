use std::panic::Location;

use irony_cmt::{Environ, EventDef, EventPort, RegionId, TmpWhen};

use super::Cmtc;
use crate::hcl::Event;
use crate::preclude::{HasEntities, ToExpr, B};

pub trait CmtcEvent {
  fn add_event(&mut self, name: Option<String>) -> Event;
  fn specify_event_on_port<T: HasEntities>(&mut self, event: &Event, port: T);
  fn specify_event_eq_signal<T: ToExpr<B<1>>>(&mut self, event: &Event, expr: T);
  fn add_event_when(&mut self, event: &Event, region_id: RegionId) -> ();
}

impl CmtcEvent for Cmtc {
  #[track_caller]
  fn add_event(&mut self, name: Option<String>) -> Event {
    let raw_name = name.or(Some("event".to_string())).unwrap();
    let legal_name = self.symbol_table.get_legal_name_in_region(&self.ir, &raw_name);
    let entity_id = self.ir.add_entity(
      irony_cmt::IREvent::new(
        Some(irony_cmt::DataTypeEnum::Void),
        Some(legal_name.to_owned().into()),
        Some(self.config.debug.into()),
        Some(Location::caller().into()),
      )
      .into(),
    );
    self.ir.add_op(EventDef::new(Some(entity_id.to_owned())).into());
    Event { entity_id, name: legal_name }
  }

  #[track_caller]
  fn specify_event_on_port<T: HasEntities>(&mut self, event: &Event, port: T) {
    self.ir.add_op(
      EventPort::new(
        Some(event.entity_id.to_owned()),
        port.v_ir_entity_id()
      ).into()
    );    
  }

  #[track_caller]
  fn specify_event_eq_signal<T: ToExpr<B<1>>>(&mut self, event: &Event, expr: T) {
    let x = expr.expr().to(self);
    self.ir.add_op(
      irony_cmt::EventSignal::new(
        Some(event.entity_id.to_owned()),
        x.v_ir_entity_id[0],
      )
      .into(),
    );
  }

  #[track_caller]
  fn add_event_when(&mut self, event: &Event, region_id: RegionId) -> () {
    self.ir.add_op(TmpWhen::new(Some(event.entity_id), Some(region_id)).into());
  }

}
