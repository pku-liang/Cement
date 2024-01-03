mod hw_test {
  use std::panic::Location;
  use std::vec;

  use irony::{Environ, Region};

  use crate::*;

  pub fn create() -> (CmtIR, OpId) {
    let mut cmt = CmtIR::default();

    let module_pass_body = cmt.add_region(Region::new(true));
    let module_pass_def = cmt.add_op(
      HwModule::new(
        Some(StringAttr("pass".into())),
        Some(BoolAttr(false)),
        Some(BoolAttr(false)),
        Some(vec![StringAttr("a".into())].into()),
        Some(vec![TypeAttr(DataTypeEnum::UInt(8.into()))].into()),
        Some(vec![StringAttr("b".into())].into()),
        Some(vec![TypeAttr(DataTypeEnum::UInt(8.into()))].into()),
        Some(module_pass_body),
      )
      .into(),
    );

    cmt.with_region(Some(module_pass_body), |cmt| {
      let a = cmt.add_entity(
        IRWire::new(
          Some(DataTypeEnum::UInt(8.into())),
          Some("a".into()),
          Some(true.into()),
          Some(LocationAttr(Location::caller().to_owned())),
        )
        .into(),
      );
      // let c = circt.add_entity(IRWire::new("c", DataTypeEnum::UInt(8.into())).into());
      cmt.add_op(HwInput::new(vec![Some(a)]).into());
      cmt.add_op(HwOutput::new(vec![Some(a)]).into());
    });

    assert!(cmt.verify_op(module_pass_def));

    let module_body = cmt.add_region(Region::new(true));
    let module_def = cmt.add_op(
      HwModule::new(
        Some(StringAttr("top".into())),
        Some(false.into()),
        Some(true.into()),
        Some(vec![StringAttr("a".into())].into()),
        Some(vec![TypeAttr(DataTypeEnum::UInt(8.into()))].into()),
        Some(vec![StringAttr("b".into())].into()),
        Some(vec![TypeAttr(DataTypeEnum::UInt(8.into()))].into()),
        Some(module_body),
      )
      .into(),
    );

    cmt.with_region(Some(module_body), |cmt| {
      let clk = cmt.add_entity(
        IRWire::new(
          Some(DataTypeEnum::UInt(1.into())),
          Some("clk".into()),
          Some(true.into()),
          Some(LocationAttr(Location::caller().to_owned())),
        )
        .into(),
      );
      let a = cmt.add_entity(
        IRWire::new(
          Some(DataTypeEnum::UInt(8.into())),
          Some("a".into()),
          Some(true.into()),
          Some(LocationAttr(Location::caller().to_owned())),
        )
        .into(),
      );
      let b = cmt.add_entity(
        IRWire::new(
          Some(DataTypeEnum::UInt(8.into())),
          Some("b".into()),
          Some(true.into()),
          Some(LocationAttr(Location::caller().to_owned())),
        )
        .into(),
      );
      let c = cmt.add_entity(
        IRWire::new(
          Some(DataTypeEnum::UInt(8.into())),
          Some("c".into()),
          Some(true.into()),
          Some(LocationAttr(Location::caller().to_owned())),
        )
        .into(),
      );
      let d = cmt.add_entity(
        IRWire::new(
          Some(DataTypeEnum::UInt(8.into())),
          Some("d".into()),
          Some(true.into()),
          Some(LocationAttr(Location::caller().to_owned())),
        )
        .into(),
      );
      let e = cmt.add_entity(
        IRWire::new(
          Some(DataTypeEnum::UInt(8.into())),
          Some("e".into()),
          Some(true.into()),
          Some(LocationAttr(Location::caller().to_owned())),
        )
        .into(),
      );
      let cond = cmt.add_entity(
        IRWire::new(
          Some(DataTypeEnum::UInt(1.into())),
          Some("cond".into()),
          Some(true.into()),
          Some(LocationAttr(Location::caller().to_owned())),
        )
        .into(),
      );
      let h = cmt.add_entity(
        IRWire::new(
          Some(DataTypeEnum::UInt(8.into())),
          Some("h".into()),
          Some(true.into()),
          Some(LocationAttr(Location::caller().to_owned())),
        )
        .into(),
      );
      let h_reg = cmt.add_entity(
        IRWire::new(
          Some(DataTypeEnum::UInt(8.into())),
          Some("h_reg".into()),
          Some(true.into()),
          Some(LocationAttr(Location::caller().to_owned())),
        )
        .into(),
      );

      cmt.add_op(HwInput::new(vec![Some(a), Some(clk)]).into());

      let instance = cmt.add_op(
        HwInstance::new(
          vec![Some(b)],
          vec![Some(a)],
          Some(module_pass_def.into()),
          Some(StringAttr("pass_inst".into())),
        )
        .into(),
      );

      assert!(cmt.verify_op(instance));

      cmt.add_op(HwConstant::new(Some(c), Some([1, 0, 0, 0].into())).into());
      cmt.add_op(
        CombVariadic::new(Some(d), vec![Some(b), Some(c)].into(), Some(CombVariadicPredicate::Add))
          .into(),
      );
      cmt.add_op(TmpUnary::new(Some(e), Some(d), Some(CombUnaryPredicate::Not)).into());

      cmt.add_op(
        CombICmp::new(Some(cond), Some(e), Some(d), Some(CombICmpPredicate::EQ)).into(),
      );

      cmt.add_op(CombMux2::new(Some(h), Some(cond), Some(d), Some(e)).into());

      cmt.add_op(SeqCompReg::new(Some(h_reg), Some(h), Some(clk), None, None).into());

      cmt.add_op(HwOutput::new(vec![Some(h_reg)]).into());
    });
    (cmt, module_def)
  }

  #[test]
  pub fn print_test() -> Result<(), ()> {
    let (cmt, ..) = create();

    let no_parent = cmt
      .op_table
      .iter()
      .filter(|(_, op)| op.get_parent().is_none())
      .map(|(id, _)| OpId(*id))
      .collect::<Vec<_>>();

    for op in no_parent.iter() {
      println!("{}", cmt.print_op(*op));
    }

    for op in no_parent.iter() {
      println!("{}", cmt.print_op(*op));
    }
    Ok(())
  }

  #[test]
  pub fn module_constraint_test() {
    let mut circt = CmtIR::default();

    let module_body = circt.add_region(Region::new(true));
    let module_def = circt.add_op(
      HwModule::new(
        Some(StringAttr("top".into())),
        Some(false.into()),
        Some(true.into()),
        Some(vec![StringAttr("a".into()), StringAttr("b".into())].into()),
        Some(
          vec![
            TypeAttr(DataTypeEnum::UInt(8.into())),
            TypeAttr(DataTypeEnum::UInt(8.into())),
          ]
          .into(),
        ),
        Some(vec![StringAttr("c".into())].into()),
        Some(vec![TypeAttr(DataTypeEnum::UInt(8.into()))].into()),
        Some(module_body),
      )
      .into(),
    );

    circt.with_region(Some(module_body), |circt| {
      let a = circt.add_entity(
        IRWire::new(
          Some(DataTypeEnum::UInt(8.into())),
          Some("a".into()),
          Some(true.into()),
          Some(LocationAttr(Location::caller().to_owned())),
        )
        .into(),
      );
      let b = circt.add_entity(
        IRWire::new(
          Some(DataTypeEnum::UInt(8.into())),
          Some("b".into()),
          Some(true.into()),
          Some(LocationAttr(Location::caller().to_owned())),
        )
        .into(),
      );
      // let c = circt.add_entity(IRWire::new("c", DataTypeEnum::UInt(8.into())).into());
      circt.add_op(HwInput::new(vec![Some(a), Some(b)]).into());
      circt.add_op(HwOutput::new(vec![Some(a)]).into());
    });

    assert!(circt.verify_op(module_def))
  }

  #[test]
  pub fn instance_constraint_test() {
    let mut circt = CmtIR::default();

    let module_pass_body = circt.add_region(Region::new(true));
    let module_pass_def = circt.add_op(
      HwModule::new(
        Some(StringAttr("pass".into())),
        Some(false.into()),
        Some(false.into()),
        Some(vec![StringAttr("a".into())].into()),
        Some(vec![TypeAttr(DataTypeEnum::UInt(8.into()))].into()),
        Some(vec![StringAttr("b".into())].into()),
        Some(vec![TypeAttr(DataTypeEnum::UInt(8.into()))].into()),
        Some(module_pass_body),
      )
      .into(),
    );

    circt.with_region(Some(module_pass_body), |circt| {
      let a = circt.add_entity(
        IRWire::new(
          Some(DataTypeEnum::UInt(8.into())),
          Some("a".into()),
          Some(true.into()),
          Some(LocationAttr(Location::caller().to_owned())),
        )
        .into(),
      );
      // let c = circt.add_entity(IRWire::new("c", DataTypeEnum::UInt(8.into())).into());
      circt.add_op(HwInput::new(vec![Some(a)]).into());
      circt.add_op(HwOutput::new(vec![Some(a)]).into());
    });

    assert!(circt.verify_op(module_pass_def));

    let module_body = circt.add_region(Region::new(true));
    circt.add_op(
      HwModule::new(
        Some(StringAttr("top".into())),
        Some(false.into()),
        Some(true.into()),
        Some(vec![StringAttr("a".into())].into()),
        Some(vec![TypeAttr(DataTypeEnum::UInt(8.into()))].into()),
        Some(vec![StringAttr("b".into())].into()),
        Some(vec![TypeAttr(DataTypeEnum::UInt(8.into()))].into()),
        Some(module_body),
      )
      .into(),
    );

    circt.with_region(Some(module_body), |circt| {
      let a = circt.add_entity(
        IRWire::new(
          Some(DataTypeEnum::UInt(8.into())),
          Some("a".into()),
          Some(true.into()),
          Some(LocationAttr(Location::caller().to_owned())),
        )
        .into(),
      );
      let b = circt.add_entity(
        IRWire::new(
          Some(DataTypeEnum::UInt(8.into())),
          Some("b".into()),
          Some(true.into()),
          Some(LocationAttr(Location::caller().to_owned())),
        )
        .into(),
      );

      circt.add_op(HwInput::new(vec![Some(a)]).into());
      circt.add_op(HwOutput::new(vec![Some(b)]).into());

      let instance = circt.add_op(
        HwInstance::new(
          vec![Some(b)],
          vec![Some(a)],
          Some(OpIdAttr(module_pass_def)),
          Some(StringAttr("pass_inst".into())),
        )
        .into(),
      );

      assert!(circt.verify_op(instance))
    });
  }
}
