use irony_cmt::{
  ArrayAttr, ArrayType, AttributeEnum, ConstantAttr, DataTypeEnum, HwAggregateConstant,
  HwConstant, IRWire,
};

use super::*;
use crate::preclude::{DataValue, FBFields, ImplFields};

pub trait CmtcBasics {
  fn get_module_name(&self, module_op_id: OpId) -> String;
  fn begin_module<T: Interface>(&mut self, ifc: T, module_name: String, is_extern: bool) -> T::ImplT;
  fn end_module<T: Interface>(&mut self, is_extern: bool) -> T::ImplT;
  fn begin_region_for_module(&mut self);
  fn end_region_for_module(&mut self);
  fn begin_region(&mut self, region_id: RegionId);
  fn end_region(&mut self) -> Option<RegionId>;
  fn add_wire(
    &mut self, data_type: irony_cmt::DataTypeEnum, suggest_name: Option<String>,
  ) -> (EntityId, String);
  fn add_wires(
    &mut self, data_types: Vec<irony_cmt::DataTypeEnum>, suggest_name: Vec<String>,
  ) -> Vec<(EntityId, String)>;
  fn add_constant(
    &mut self, wire: EntityId, data_type: irony_cmt::DataTypeEnum, value: DataValue,
  ) -> ();
  fn assign(&mut self, lhs: Vec<Option<EntityId>>, rhs: Vec<Option<EntityId>>) -> ();
  fn add_instance(
    &mut self, target_module_op_id: OpId, instance_name: String, inputs: Vec<Option<EntityId>>,
    outputs: Vec<Option<EntityId>>,
  ) -> ();
  fn add_op(&mut self, op: OpEnum) -> OpId;
  fn add_tcl(&mut self, op_id: OpId, tcl: TclIP);
}

impl CmtcBasics for Cmtc {
  fn get_module_name(&self, module_op_id: OpId) -> String {
    match self.ir.get_op(module_op_id) {
      OpEnum::HwModule(hw_module) => hw_module.name.to_owned().unwrap().0,
      _ => {
        panic!("module_op_id must be HwModule")
      },
    }
  }

  #[track_caller]
  fn begin_module<T: Interface>(&mut self, ifc: T, module_name: String, is_extern: bool) -> T::ImplT {
    self.begin_region_for_module();
    let module_body = self.ir.add_region(Region::new(true));
    let module_name =
      self.symbol_table.get_legal_name_in_region(&self.ir, module_name.as_str());

    let module_def_op = self.ir.add_op(
      HwModule::new(
        Some(module_name.into()),
        Some(is_extern.into()),
        Some(true.into()),
        None,
        None,
        None,
        None,
        Some(module_body),
      )
      .into(),
    );

    self.begin_region(module_body);

    let ifc_fields = ifc.traverse();

    let ifc_impl_fields = ifc_fields.to(self, None);

    let FBFields {
      fwd:
        ImplFields {
          v_name: arg_names,
          v_data_type: arg_types,
          v_entity_id: arg_entities,
        },
      bwd:
        ImplFields {
          v_name: output_names,
          v_data_type: output_types,
          v_entity_id: output_entities,
        },
    } = ifc_impl_fields.to_owned().split();

    self.ir.get_op_entry(module_def_op).and_modify(|hw_module| {
      *hw_module = match hw_module {
        OpEnum::HwModule(hw_module) => HwModule::new(
          hw_module.name.to_owned(),
          Some(is_extern.into()),
          Some(true.into()),
          Some(vec_string_to_array_attr(arg_names)),
          Some(vec_data_type_to_array_attr(arg_types)),
          Some(vec_string_to_array_attr(output_names)),
          Some(vec_data_type_to_array_attr(output_types)),
          hw_module.body,
        )
        .into(),
        _ => {
          panic!("module_def_op must be HwModule")
        },
      }
    });

    if !is_extern {
      self.ir.add_op(HwInput::new(arg_entities).into());
      self.ir.add_op(HwOutput::new(output_entities).into());
    }

    let ifc_impl = ifc.to_owned().impl_with(ifc_impl_fields.to_owned());
    self.module_stack.push(ifc_impl, module_def_op);
    ifc.clone().impl_with(ifc_impl_fields)
  }

  fn end_module<T: Interface>(&mut self, _is_extern: bool) -> T::ImplT {
    self.end_region();
    self.end_region_for_module();

    self.module_stack.pop()
  }

  fn begin_region_for_module(&mut self) { self.ir.begin_region(None); }

  fn end_region_for_module(&mut self) {
    assert!(
      matches!(self.ir.end_region(), Some(None)),
      "end_region_for_module must end a None region"
    );
  }

  fn begin_region(&mut self, region_id: RegionId) {
    self.ir.begin_region(Some(region_id))
  }

  fn end_region(&mut self) -> Option<RegionId> {
    let region = self.ir.end_region();
    assert!(
      matches!(region, Some(Some(_))),
      "end_region must end a Some(RegionId) region"
    );
    region.expect("exist one region to end")
  }

  #[track_caller]
  fn add_wire(
    &mut self, data_type: irony_cmt::DataTypeEnum, suggest_name: Option<String>,
  ) -> (EntityId, String) {
    let raw_name = suggest_name.expect("wire must have name when it's added to IR");
    let legal_name = self.symbol_table.get_legal_name_in_region(&self.ir, &raw_name);
    (
      self.ir.add_entity(
        IRWire::new(
          Some(data_type),
          Some(legal_name.to_owned().into()),
          Some(self.config.debug.into()),
          Some(Location::caller().into()),
        )
        .into(),
      ),
      legal_name,
    )
  }

  #[track_caller]
  fn add_wires(
    &mut self, data_types: Vec<irony_cmt::DataTypeEnum>, suggest_name: Vec<String>,
  ) -> Vec<(EntityId, String)> {
    let mut v = Vec::new();
    for (data_type, suggest_name) in data_types.into_iter().zip(suggest_name.into_iter())
    {
      v.push(self.add_wire(data_type, Some(suggest_name)));
    }
    v
  }

  fn add_constant(
    &mut self, wire: EntityId, data_type: irony_cmt::DataTypeEnum, value: DataValue,
  ) -> () {
    match (data_type, value) {
      (DataTypeEnum::UInt(_), DataValue::Bits(bits)) => {
        self.ir.add_op(HwConstant::new(Some(wire), Some(ConstantAttr(bits.data))).into());
      },
      (data_type @ DataTypeEnum::Array(_), value @ _) => {
        let AttributeEnum::ArrayAttr(array_attr) =
          value_to_array_attr(&data_type, &value)
        else {
          panic!()
        };
        self.ir.add_op(HwAggregateConstant::new(Some(wire), Some(array_attr)).into());
      },

      (data_type, value) => {
        panic!("not supported/matched constant: {:?} to {:?}", value, data_type)
      },
    }
  }

  fn assign(&mut self, lhs: Vec<Option<EntityId>>, rhs: Vec<Option<EntityId>>) -> () {
    assert!(lhs.len() == rhs.len(), "assign lhs and rhs must have same length");
    for (lhs, rhs) in lhs.into_iter().zip(rhs.into_iter()) {
      self.ir.add_op(Assign::new(lhs, rhs).into());
    }
  }

  fn add_instance(
    &mut self, target_module_op_id: OpId, instance_name: String, inputs: Vec<Option<EntityId>>,
    outputs: Vec<Option<EntityId>>,
  ) -> () {
    let target_module_op_id = if self.config.deduplicate {
      self
        .ir
        .hash_op(target_module_op_id)
        .expect("cannot be None for module deduplication")
    } else {
      target_module_op_id
    };

    self.ir.get_op_entry(target_module_op_id).and_modify(|hw_module| match hw_module {
      OpEnum::HwModule(hw_module) => hw_module.top = Some(false.into()),
      _ => {
        panic!("target_module_op_id must be HwModule")
      },
    });

    // Assuming the instance_name has been legalized in the region
    self.ir.add_op(
      HwInstance::new(
        outputs,
        inputs,
        Some(target_module_op_id.into()),
        Some(instance_name.into()),
      )
      .into(),
    );
  }
  
  fn add_op(&mut self, op: OpEnum) -> OpId { self.ir.add_op(op) }

  fn add_tcl(&mut self, op_id: OpId, tcl: TclIP) {
    self.ip_tcls.add(op_id, tcl);
  }
}

fn vec_string_to_array_attr(v_string: Vec<String>) -> irony_cmt::ArrayAttr {
  irony_cmt::ArrayAttr(
    v_string.into_iter().map(|s| irony_cmt::StringAttr(s).into()).collect(),
  )
}

fn vec_data_type_to_array_attr(
  v_data_type: Vec<irony_cmt::DataTypeEnum>,
) -> irony_cmt::ArrayAttr {
  irony_cmt::ArrayAttr(
    v_data_type.into_iter().map(|s| irony_cmt::TypeAttr(s).into()).collect(),
  )
}

fn value_to_array_attr(data_type: &DataTypeEnum, value: &DataValue) -> AttributeEnum {
  match data_type {
    DataTypeEnum::UInt(_) => {
      if let DataValue::Bits(bits) = value {
        AttributeEnum::ConstantAttr(ConstantAttr(bits.data.to_owned()))
      } else {
        panic!("value {:?} doesn't match type {:?}", value, data_type)
      }
    },
    DataTypeEnum::Array(ArrayType(sub_type, _)) => {
      if let DataValue::Nested(nested) = value {
        let mut vattr = Vec::new();
        for item in nested.data.iter() {
          vattr.push(value_to_array_attr(&sub_type, item));
        }
        AttributeEnum::ArrayAttr(ArrayAttr(vattr))
      } else {
        panic!("value {:?} doesn't match type {:?}", value, data_type)
      }
    },
    DataTypeEnum::Struct(_) => todo!(),
    _ => panic!("type {:?} unsupported yet", data_type),
  }
}
