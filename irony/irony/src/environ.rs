use std::cell::RefMut;

use super::constraint::ConstraintTrait;
use super::entity::{Entity, EntityId};
use super::operation::{Op, OpId};
use crate::{Id, OpPrinterTrait, ReducerTrait, Region, RegionId, AsBool};

pub trait Environ: Sized {
  type DataTypeT;
  type AttributeT: Clone + PartialEq + std::fmt::Display + AsBool;

  type OpT: Op<DataTypeT = Self::DataTypeT, AttributeT = Self::AttributeT>;
  type EntityT: Entity<DataTypeT = Self::DataTypeT, AttributeT = Self::AttributeT>;
  type ConstraintT: ConstraintTrait<
    AttributeT = Self::AttributeT,
    DataTypeT = Self::DataTypeT,
  >;

  fn get_defs(&self, id: EntityId) -> Vec<OpId>;
  fn get_uses(&self, id: EntityId) -> Vec<OpId>;
  fn get_entity(&self, id: EntityId) -> &Self::EntityT;
  fn get_entities(&self, ids: &[EntityId]) -> Vec<&Self::EntityT>;
  fn get_entities_with_parent(&self, id: Option<RegionId>) -> Vec<EntityId>;
  fn get_entity_entry(
    &mut self, entity_id: EntityId,
  ) -> indexmap::map::Entry<usize, Self::EntityT>;

  fn update_entity_attr<F>(&mut self, entity_id: EntityId, field_name: &str, f: F) -> ()
  where F: Fn(Self::AttributeT) -> Self::AttributeT {
    self
      .get_entity_entry(entity_id)
      .and_modify(|entity| entity.update_attrs(field_name, f));
  }

  fn get_op(&self, id: OpId) -> &Self::OpT;
  fn get_op_entry(&mut self, op_id: OpId) -> indexmap::map::Entry<usize, Self::OpT>;

  fn get_ops(&self, ids: &[OpId]) -> Vec<&Self::OpT>;
  fn get_ops_with_parent(&self, id: Option<RegionId>) -> Vec<OpId>;
  fn add_entity(&mut self, entity: Self::EntityT) -> EntityId;
  fn get_region(&self, id: RegionId) -> &Region;
  fn get_region_entry(
    &mut self, region_id: RegionId,
  ) -> indexmap::map::Entry<usize, Region>;
  fn add_region(&mut self, region: Region) -> RegionId;
  fn add_op(&mut self, op: Self::OpT) -> OpId;
  fn set_entity_parent(&mut self, id: EntityId);
  fn set_op_parent(&mut self, id: OpId);
  fn get_region_use(&self, region: RegionId) -> Option<OpId>;
  fn begin_region(&mut self, region: Option<RegionId>);
  fn end_region(&mut self) -> Option<Option<RegionId>>;

  fn with_region<F: for<'a> Fn(&mut Self) -> ()>(
    &mut self, parent: Option<RegionId>, f: F,
  );

  fn verify_op(&self, op: OpId) -> bool {
    let op = self.get_op(op);
    let constraints = op.get_constraints();
    let attributes = op.get_attrs();
    let uses = op.get_uses();
    let defs = op.get_defs();
    let regions = op.get_regions();
    let all_true = constraints.into_iter().all(|constraint| {
      constraint.verify(
        self,
        attributes.to_owned(),
        defs.to_owned(),
        uses.to_owned(),
        regions.to_owned(),
      )
    });
    all_true
  }

  fn print_op(&self, op_id: OpId) -> String {
    self.verify_op(op_id);

    let op = self.get_op(op_id);
    let printer = op.get_printer();
    let attributes = op.get_attrs();
    let uses = op.get_uses();
    let defs = op.get_defs();
    let regions = op.get_regions();

    // println!("op: {:?}", op_id);
    let mut str = printer.print(self, attributes, uses, defs.to_owned(), regions);

    for (_def_name, defv) in defs.iter() {
      for def in defv {
        if let Some(entity_id) = def {
          let entity = self.get_entity(*entity_id);
          let debug = entity.get_attr("debug");
          let location = entity.get_attr("location");
          match (debug, location) {
            (Some(debug), Some(location)) => {
              if debug.as_bool() {
                str = format!(
                  "{}\n\t// {}: {}",
                  str,
                  self.print_entity(*entity_id),
                  location
                );
              }
            },
            _ => {},
          }
        } else {
        }
      }
    }

    str
  }

  fn print_entity(&self, entity: EntityId) -> String {
    // TODO: Add better printing for entities
    let entity = self.get_entity(entity);
    let attrs = entity.get_attrs();
    if let Some(name) = crate::utils::extract_vec(&attrs, "name") {
      format!("%{}", name)
    } else {
      format!("%{}", entity.id())
    }
  }

  fn print_region(&self, region: RegionId) -> String {
    let region = self.get_region(region);
    let mut ops = vec![];

    for op in region.op_children.iter() {
      ops.push(format!("{}", self.print_op(*op)));
    }
    format!("{}", crate::utils::print::tab(ops.join("\n")))
  }

  fn delete_entity(&mut self, entity_id: EntityId);

  fn delete_op(&mut self, op_id: OpId) -> ();
  fn delete_op_and_all(&mut self, op_id: OpId) -> ();

  fn delete_region(&mut self, region_id: Option<RegionId>) -> () {
    match region_id {
      Some(region_id) => {
        for op in self.get_region(region_id).get_op_children() {
          self.delete_op_and_all(op);
        }
        for entity in self.get_region(region_id).get_entity_children() {
          self.delete_entity(entity);
        }
      },
      None => {},
    }
  }

  fn dump(&self) -> String;

  fn run_passes(&mut self) -> Result<(), ()>; // -> ???

  #[track_caller]
  fn get_hasher(&self) -> RefMut<crate::FxHasher>;

  fn hash_region(&self, region: RegionId, reducer: &mut impl ReducerTrait) {
    let region = self.get_region(region);

    for op in region.get_op_children() {
      self.get_op(op).hash_with_reducer(self, reducer);
    }
  }
}

#[macro_export]
macro_rules! environ_def {
    (
        [data_type = $data_ty:ty, attr = $attr_ty:ty, entity = $entity_ty:ty, op = $op_ty:ty, constraint = $constraint_ty:ty, pm = $pm_ty:ty]
        struct $name:ident;
    ) => {
        irony::environ_def! {
            @inner
            DATATYPE: $data_ty;
            ENTITY: $entity_ty;
            OP: $op_ty;
            ATTR: $attr_ty;
            CONSTRAINT: $constraint_ty;
            PASSMANAGER: $pm_ty;
            NAME: $name;
            FIELDS: ;
        }
    };

    (
        [data_type = $data_ty:ty, attr = $attr_ty:ty, entity = $entity_ty:ty, op = $op_ty:ty, constraint = $constraint_ty:ty, pm = $pm_ty:ty]
        struct $name:ident {
            $(
                $field_vis:vis $field_name:ident : $field_ty:ty
            ),*
        }
    ) => {
        irony::environ_def! {
            @inner
            DATATYPE: $data_ty;
            ENTITY: $entity_ty;
            OP: $op_ty;
            ATTR: $attr_ty;
            CONSTRAINT: $constraint_ty;
            PASSMANAGER: $pm_ty;
            NAME: $name;
            FIELDS: $($field_vis $field_name : $field_ty)*;
        }
    };

    (@inner
        DATATYPE: $data_ty:ty;
        ENTITY: $entity_ty:ty;
        OP: $op_ty:ty;
        ATTR: $attr_ty:ty;
        CONSTRAINT: $constraint_ty:ty;
        PASSMANAGER: $pm_ty:ty;
        NAME: $name:ident ;
        FIELDS: $($field_vis:vis $field_name:ident : $field_ty:ty)* ;
    ) => {

        #[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
        pub struct OpHashT(Option<RegionId>, u64);

        use std::rc::Rc;

        #[StructFields(pub)]
        #[derive(Default)]
        pub struct $name {
            op_table: irony::FxMapWithUniqueId<$op_ty>,
            entity_table: irony::FxMapWithUniqueId<$entity_ty>,
            region_table: irony::FxMapWithUniqueId<irony::Region>,
            parent_stack: Vec<Option<irony::RegionId>>,
            pass_manager: $pm_ty,

            hasher: Rc<RefCell<irony::FxHasher>>,
            op_hash_table: FxHashMap<OpHashT, irony::OpId>,

            $($field_vis $field_name: $field_ty,)*
        }

        use std::hash::BuildHasher;
        use std::hash::Hasher;

        impl irony::Environ for $name {
            type DataTypeT = $data_ty;

            type OpT = $op_ty;

            type EntityT = $entity_ty;

            type ConstraintT = $constraint_ty;

            type AttributeT = $attr_ty;

            fn get_defs(&self, entity: irony::EntityId) -> Vec<irony::OpId> {
                self.op_table
                .iter()
                .filter(|tuple| tuple.1.defs(entity))
                .map(|tuple| irony::OpId::from(*tuple.0))
                .collect()
            }

            fn get_uses(&self, entity: irony::EntityId) -> Vec<irony::OpId> {
                let mut v = Vec::new();
                for (id, op) in self.op_table.get_map() {
                    if op.uses(irony::EntityId::from(entity.to_owned())) {
                        v.push(irony::OpId::from(*id));
                    }
                }
                v
            }

            fn get_entity(&self, id: irony::EntityId) -> &Self::EntityT {
                match self.entity_table.get(&id.id()) {
                    Some(entity) => entity,
                    None => panic!(
                        "get entity not in the table by id \ntable: {:#?}\nentity: {:#?}",
                        self.entity_table.get_map(),
                        id.id()
                    ),
                }
            }

            fn get_entities(&self, ids: &[irony::EntityId]) -> Vec<&Self::EntityT> {
                ids.iter()
                .map(|id| self.get_entity(id.to_owned()))
                .collect()
            }

            fn get_entities_with_parent(&self, parent: Option<RegionId>) -> Vec<EntityId> {
                self.entity_table.iter().filter_map(|(id, entity)| {
                    if entity.get_parent() == parent {
                        Some(EntityId(*id))
                    } else {
                        None
                    }
                }).collect()
            }

            fn get_entity_entry(&mut self , entity_id: irony::EntityId) -> indexmap::map::Entry<usize, Self::EntityT> {
                self.entity_table.entry(entity_id.id())
            }

            fn get_op(&self, id: irony::OpId) -> &Self::OpT {
                match self.op_table.get(&id.id()) {
                    Some(op) =>op,
                    None => panic!(
                        "get op not in the table by id \ntable: {:#?}\nop: {:#?}",
                        self.op_table.get_map(),
                        id.id()
                    ),
                }
            }
            fn get_ops_with_parent(&self, parent: Option<RegionId>) -> Vec<OpId> {
                self.op_table.iter().filter_map(|(id, op)| {
                    if op.get_parent() == parent {
                        Some(OpId(*id))
                    } else {
                        None
                    }
                }).collect()
            }


            fn get_op_entry(&mut self, op_id: irony::OpId) -> indexmap::map::Entry<usize, Self::OpT> {
                self.op_table.entry(op_id.id())
            }

            fn get_ops(&self, ids: &[irony::OpId]) -> Vec<&Self::OpT> {
                ids.iter()
                .map(|id| self.get_op(id.to_owned()))
                .collect()
            }

            fn add_entity(&mut self, entity: Self::EntityT) -> irony::EntityId {
                let (id, _) = self.entity_table.insert_with_id(entity);
                self.set_entity_parent(irony::EntityId::from(id));
                irony::EntityId(id)
            }

            fn get_region(&self, id: irony::RegionId) -> &irony::Region {
                match self.region_table.get(&id.id()) {
                    Some(region) => region,
                    None => panic!(
                        "get region not in the table by id \ntable: {:#?}\nregion: {:#?}",
                        self.region_table.get_map(),
                        id.id()
                    ),
                }
            }

            fn get_region_entry(&mut self, region_id: irony::RegionId) -> indexmap::map::Entry<usize, irony::Region> {
                self.region_table.entry(region_id.id())
            }

            fn add_region(&mut self, region: irony::Region) -> irony::RegionId {
                let (id, _) = self.region_table.insert_with_id(region);
                irony::RegionId(id)
            }

            fn add_op(&mut self, op: Self::OpT) -> irony::OpId {
                let (id, op) = self.op_table.insert_with_id(op);
                self.set_op_parent(irony::OpId::from(id));
                irony::OpId(id)
            }

            fn set_entity_parent(&mut self, entity: irony::EntityId) {
                if let Some(parent) = self.parent_stack.last() {
                    self.entity_table
                        .entry(entity.id())
                        .and_modify(|entity| entity.set_parent(parent.to_owned()));
                    if let Some(parent) = parent {
                        self.region_table.entry(parent.id()).and_modify(|region|
                            region.add_entity_child(irony::EntityId(entity.id()))
                        );
                    }
                }
            }

            fn set_op_parent(&mut self, op: irony::OpId) {
                if let Some(parent) = self.parent_stack.last() {
                    self.op_table
                        .entry(op.id())
                        .and_modify(|op| op.set_parent(parent.to_owned()));
                    if let Some(parent) = parent {
                        self.region_table.entry(parent.id()).and_modify(|region|
                            region.add_op_child(irony::OpId(op.id()))
                        );
                    }
                }
            }

            fn with_region<F: Fn(&mut Self) -> ()>(&mut self, parent: Option<irony::RegionId>, f: F) {
                self.begin_region(parent);
                f(self);
                self.end_region();
            }

            fn get_region_use(&self, region: irony::RegionId) -> Option<irony::OpId> {
                self.op_table.iter().find(|tuple| tuple.1.use_region(region))
                .map(|tuple| irony::OpId::from(*tuple.0))
            }

            fn begin_region(&mut self, region: Option<irony::RegionId>) {
                self.parent_stack.push(region);
            }
            fn end_region(&mut self) -> Option<Option<RegionId>> {
                self.parent_stack.pop()
            }

            fn dump(&self) -> String {
                format!("entity table: {:#?}\nregion table: {:#?}\nop table: {:#?}", self.entity_table.get_map(), self.region_table.get_map(), self.op_table.get_map())
            }

            fn run_passes(&mut self) -> Result<(), ()>{
                let pass_manager = self.pass_manager.clone();
                pass_manager.run_passes(self)?;
                Ok(())
            }

            fn get_hasher(&self) -> RefMut<irony::FxHasher> {
                self.hasher.borrow_mut()
            }

            fn delete_entity(&mut self, entity_id: EntityId) {
                // if !self.get_entity(entity_id).get_uses(self).is_empty() {
                //     panic!("The op to be deleted defines an entity that has been used.\n\t{} has been used by\n\t{}\n", self.print_entity(entity_id), self.get_entity(entity_id).get_uses(self).iter().map(|op| self.print_op(*op)).collect::<Vec<String>>().join("\n\t"));
                // } else {
                //     self.entity_table.remove(&entity_id.id());
                // }

                // TODO: turn the "delete entity being used" panic into some checking

                self.entity_table.remove(&entity_id.id());
            }

            fn delete_op(&mut self, op_id: OpId) -> () {
              let parent = self.get_op(op_id).get_parent().to_owned();
              match parent {
                Some(parent) => {
                  self.region_table.entry(parent.id()).and_modify(|region|
                    region.delete_op_child(op_id)
                  );
                },
                None => {}
              }
              self.op_table.remove(&op_id.id());
            }

            fn delete_op_and_all(&mut self, op_id: OpId) -> () {
                for (_, def_field) in self.get_op(op_id).get_defs() {
                    for def in def_field {
                        if let Some(entity_id) = def {
                            {
                                self.delete_entity(entity_id);
                            }
                        }
                    }
                }

                for (_, region_field) in self.get_op(op_id).get_regions() {
                    for region in region_field {
                        self.delete_region(region);
                    }
                }

                let parent = self.get_op(op_id).get_parent().to_owned();
                match parent {
                  Some(parent) => {
                    self.region_table.entry(parent.id()).and_modify(|region|
                      region.delete_op_child(op_id)
                    );
                  },
                  None => {}
                }
                self.op_table.remove(&op_id.id());

            }
        }
    };
}
