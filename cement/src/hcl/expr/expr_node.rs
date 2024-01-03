use irony_cmt::*;

use crate::compiler::{Cmtc, CmtcBasics};
use crate::preclude::{FBFields, FBNames, IfcFields, DataValue};

#[derive(Debug, Clone)]
pub enum ExprNode {
  // Invalid(DataTypeEnum),
  Clone,

  Assign,

  Tuple,

  Variadic(Variadic),

  Binary(Binary),

  Unary(Unary),

  Mux,

  Select(bool),

  Cmpi(Cmpi),

  Cast(Cast),

  Constant(Constant),

  Extract(Extract),

  Concat(Concat),

  Reg,

  ArrayCreate,

  ArrayConcat,

  ArraySlice(usize),

  ArrayGet,

  StructCreate(StructType),

  StructExtract(String),

  StructInject(String),

  StructExplode,

  EventToSignal(EntityId),
}

impl ExprNode {
  pub fn to_str(&self) -> &str {
    match self {
      // ExprNode::Invalid(_) => "invalid",
      ExprNode::Clone => "clone",
      ExprNode::Assign => "assign",
      ExprNode::Tuple => "tuple",
      ExprNode::Variadic(variadic) => variadic.to_str(),
      ExprNode::Binary(binary) => binary.to_str(),
      ExprNode::Unary(unary) => unary.to_str(),
      ExprNode::Mux => "mux",
      ExprNode::Select(_) => "select",
      ExprNode::Cmpi(cmpi) => cmpi.to_str(),
      ExprNode::Cast(cast) => cast.to_str(),
      ExprNode::Constant(constant) => constant.to_str(),
      ExprNode::Extract(_) => "extract",
      ExprNode::Concat(_) => "concat",
      ExprNode::Reg => "reg",
      ExprNode::ArrayCreate => "array_create",
      ExprNode::ArrayConcat => "array_concat",
      ExprNode::ArraySlice(_) => "array_slice",
      ExprNode::ArrayGet => "array_get",
      ExprNode::StructCreate(_) => "struct_create",
      ExprNode::StructExtract(_) => "struct_extract",
      ExprNode::StructInject(_) => "struct_inject",
      ExprNode::StructExplode => "struct_explode",
      ExprNode::EventToSignal(_) => "event_to_signal",
    }
  }

  #[track_caller]
  pub fn eval(
    self, c: &mut Cmtc, operands: Vec<FBFields>, suggester: FBNames,
  ) -> FBFields {
    let (fwd, bwd): (Vec<(String, DataTypeEnum, Option<EntityId>)>, Vec<(String, DataTypeEnum, Option<EntityId>)>) = match self {
      // ExprNode::Invalid(dtyp) => {
      // },
      ExprNode::Clone => {
        assert!(operands.len() == 1);
        let mut v_rst = Vec::new();
        for (op, suggest_name) in
          operands[0].fwd.to_owned().into_iter().zip(suggester.into_iter())
        {
          let ((_name, data_type), _entity) = op;
          let name = suggest_name.or(Some(format!("clone")));
          let (rst, _) = c.add_wire(data_type.to_owned(), name.to_owned());
          v_rst.push((name.unwrap(), data_type, Some(rst)));
        }
        (v_rst, vec![])
      },
      ExprNode::Assign => {
        assert!(operands.len() == 1, "assign must have 1 operand");
        let mut v_rst = Vec::new();

        for (((name, data_type), entity), suggest_name) in
          operands[0].fwd.to_owned().into_iter().zip(suggester.into_iter())
        {
          let name = suggest_name.or(Some(format!("dup_{}", name)));
          let (rst, _) = c.add_wire(data_type.to_owned(), name.to_owned());
          c.add_op(Assign::new(Some(rst), entity).into());
          v_rst.push((name.unwrap(), data_type, Some(rst)));
        }
        (v_rst, vec![])
      },

      ExprNode::Tuple => {
        let mut v_fwd = Vec::new();
        let mut v_bwd = Vec::new();
        for operand in operands {
          let FBFields { fwd, bwd } = operand;
          let mut fwd = fwd.zip3();
          let mut bwd = bwd.zip3();

          v_fwd.append(&mut fwd);
          v_bwd.append(&mut bwd);
        }
        (v_fwd, v_bwd)
      },

      ExprNode::Variadic(variadic) => {
        assert!(operands.len() == 2);

        let mut v_rst = Vec::new();
        for ((op0, op1), suggest_name) in operands[0]
          .fwd
          .to_owned()
          .into_iter()
          .zip(operands[1].fwd.to_owned().into_iter())
          .zip(suggester.into_iter())
        {
          let ((_name0, data_type0), entity0) = op0;
          let ((_name1, data_type1), entity1) = op1;

          assert!(data_type0.width() == data_type1.width());
          let name = suggest_name.or(Some(format!(
            // "{}_{}_{}",
            "{}",
            variadic.to_str(),
            // name0,
            // name1
          )));

          let (rst, _) = c.add_wire(data_type0.to_owned(), name.to_owned());
          c.add_op(
            CombVariadic::new(
              Some(rst),
              vec![entity0, entity1],
              Some(variadic.to_predicate()),
            )
            .into(),
          );
          v_rst.push((name.unwrap(), data_type0, Some(rst)));
        }
        (v_rst, vec![])
      },

      ExprNode::Binary(binary) => {
        assert!(operands.len() == 2);

        let mut v_rst = Vec::new();
        for ((op0, op1), suggest_name) in operands[0]
          .fwd
          .to_owned()
          .into_iter()
          .zip(operands[1].fwd.to_owned().into_iter())
          .zip(suggester.into_iter())
        {
          let ((_name0, data_type0), entity0) = op0;
          let ((_name1, data_type1), entity1) = op1;

          assert!(data_type0.width() == data_type1.width());
          let name = suggest_name.or(Some(format!(
            // "{}_{}_{}",
            "{}",
            binary.to_str(),
            // name0,
            // name1
          )));

          let (rst, _) = c.add_wire(data_type0.to_owned(), name.to_owned());
          c.add_op(
            CombBinary::new(
              Some(rst),
              entity0,
              entity1,
              Some(binary.to_predicate()),
            )
            .into(),
          );
          v_rst.push((name.unwrap(), data_type0, Some(rst)));
        }
        (v_rst, vec![])
      },

      ExprNode::Unary(unary) => {
        assert!(operands.len() == 1);
        let mut v_rst = Vec::new();
        for (op, suggest_name) in
          operands[0].fwd.to_owned().into_iter().zip(suggester.into_iter())
        {
          let ((_name, data_type), entity) = op;
          let name =
            suggest_name.or(Some(format!("{}", unary.to_str() /* , name*/)));
          let (rst, _) = c.add_wire(data_type.to_owned(), name.to_owned());
          c.add_op(
            TmpUnary::new(Some(rst), entity, Some(unary.to_predicate())).into(),
          );
          v_rst.push((name.unwrap(), data_type, Some(rst)));
        }
        (v_rst, vec![])
      },

      ExprNode::Mux => {
        assert!(operands.len() == 3);
        let mut v_rst = Vec::new();
        let sel = operands[0].fwd.to_owned().into_iter().next().unwrap();
        for ((op0, op1), suggest_name) in operands[1]
          .fwd
          .to_owned()
          .into_iter()
          .zip(operands[2].fwd.to_owned().into_iter())
          .zip(suggester.into_iter())
        {
          let ((_name0, data_type0), entity0) = op0;
          let ((_name1, data_type1), entity1) = op1;

          let ((_name_sel, data_type_sel), entity_sel) = sel.to_owned();

          assert!(data_type0.width() == data_type1.width());

          assert!(data_type_sel.width() == 1);

          let name = suggest_name.or(Some(format!(
            "{}",
            self.to_str(),
            // name_sel,
            // name0,
            // name1
          )));

          let (rst, _) = c.add_wire(data_type0.to_owned(), name.to_owned());
          c.add_op(
            CombMux2::new(Some(rst), entity_sel, entity0, entity1)
              .into(),
          );
          v_rst.push((name.unwrap(), data_type0, Some(rst)));
        }
        (v_rst, vec![])
      },

      ExprNode::Select(select) => {
        let num_candidates = if operands.len() % 2 == 1 {
          (operands.len() - 1) / 2
        } else {
          operands.len() / 2
        };

        let entities_per_operand = operands[0].fwd.len();
        // println!("{:#?}", operands);
        // assert!(operands.iter().all(|x| x.fwd.len() == entities_per_operand));

        let mut v_rst = Vec::new();
        for i in 0..entities_per_operand {
          let data_type =
            operands[num_candidates].fwd.to_owned().v_data_type[i].to_owned();

          assert!(
            operands[..num_candidates]
              .iter()
              .all(|x| { x.fwd.v_data_type[i] == DataTypeEnum::UInt(UIntType(1)) }),
            "select's conds must have U1 type"
          );

          assert!(
            operands[num_candidates..]
              .iter()
              .all(|x| { x.fwd.v_data_type[i] == data_type }),
            "select's candidates must have same type"
          );

          let name = suggester[i].to_owned().or(Some(format!("select")));

          let (rst, _) = c.add_wire(data_type.to_owned(), name.to_owned());

          c.add_op(
            TmpSelect::new(
              Some(rst),
              if operands.len() % 2 == 1 {
                operands.last().map(|x| x.fwd.v_entity_id[i]).unwrap()
              } else {
                None
              },
              operands[..num_candidates].iter().map(|x| x.fwd.v_entity_id[i]).collect(),
              operands[num_candidates..].iter().map(|x| x.fwd.v_entity_id[i]).collect(),
              Some(select.into()),
            )
            .into(),
          );
          v_rst.push((name.unwrap(), data_type, Some(rst)));
        }
        (v_rst, vec![])
      },

      ExprNode::Cmpi(cmpi) => {
        assert!(operands.len() == 2);
        let mut v_rst = Vec::new();
        for ((op0, op1), suggest_name) in operands[0]
          .fwd
          .to_owned()
          .into_iter()
          .zip(operands[1].fwd.to_owned().into_iter())
          .zip(suggester.into_iter())
        {
          let ((_name0, data_type0), entity0) = op0;
          let ((_name1, data_type1), entity1) = op1;

          assert!(data_type0.width() == data_type1.width());
          let name = suggest_name.or(Some(format!(
            "{}",
            self.to_str(),
            // name0,
            // name1
          )));

          let (rst, _) = c.add_wire(DataTypeEnum::UInt(UIntType(1)), name.to_owned());
          c.add_op(
            CombICmp::new(
              Some(rst),
              entity0,
              entity1,
              Some(cmpi.to_predicate()),
            )
            .into(),
          );
          v_rst.push((name.unwrap(), DataTypeEnum::UInt(UIntType(1)), Some(rst)));
        }
        (v_rst, vec![])
      },

      ExprNode::Cast(cast) => {
        assert!(operands.len() == 1);

        let mut v_rst = Vec::new();
        for (((_name, data_type), entity), suggest_name) in
          operands[0].fwd.to_owned().into_iter().zip(suggester.into_iter())
        {
          let name = suggest_name.or(Some(format!("cast")));
          let (rst, _) = c.add_wire(cast.to_owned().target_data_type, name.to_owned());

          if cast.to_owned().target_data_type == data_type {
            c.add_op(Assign::new(Some(rst), entity).into());
          } else {
            assert!(cast.to_owned().target_data_type.width() == data_type.width());
            c.add_op(HwBitCast::new(Some(rst), entity).into());
          }
          v_rst.push((name.unwrap(), data_type, Some(rst)));
        }
        (v_rst, vec![])
      },

      ExprNode::Constant(constant) => {
        assert!(operands.len() == 0);
        assert!(suggester.len() == 1);
        // println!("{:#?}", constant);
        let Constant { ifc_fields, v_data } = constant;
        let name = suggester[0].to_owned().expect("constant must have name");
        let ifc_impl_fields = ifc_fields.to_with_constant(c, Some(name), &mut v_data.into_iter());
        
        let FBFields { fwd, bwd } = ifc_impl_fields.split();
        (fwd.zip3(), bwd.zip3())
      },

      ExprNode::Extract(Extract { target_data_type, low }) => {
        assert!(operands.len() == 1);
        let mut v_rst = Vec::new();
        for (((_name, _data_type), entity), suggest_name) in
          operands[0].fwd.to_owned().into_iter().zip(suggester.into_iter())
        {
          let name = suggest_name.or(Some(format!("extract")));
          let (rst, _) = c.add_wire(target_data_type.to_owned(), name.to_owned());

          c.add_op(CombExtract::new(Some(rst), entity, Some(low.into())).into());

          v_rst.push((name.unwrap(), target_data_type.to_owned(), Some(rst)));
        }
        (v_rst, vec![])
      },

      ExprNode::Concat(_) => {
        let entities_per_operand = operands[0].fwd.len();
        assert!(operands.iter().all(|x| x.fwd.len() == entities_per_operand));

        let mut v_rst = Vec::new();

        for i in 0..entities_per_operand {
          let data_type = operands[0].fwd.to_owned().v_data_type[i].to_owned();

          let len = operands
            .iter()
            .map(|op| {
              let DataTypeEnum::UInt(UIntType(len)) = op.fwd.v_data_type[i] else {
                panic!("operands must be uint for concat")
              };
              len
            })
            .reduce(|x, y| x + y)
            .unwrap();

          let name = suggester[i].to_owned().or(Some(format!("concat")));

          let (rst, _) = c.add_wire(DataTypeEnum::UInt(UIntType(len)), name.to_owned());

          c.add_op(
            CombConcat::new(
              Some(rst),
              operands.iter().map(|op| op.fwd.v_entity_id[i]).collect(),
            )
            .into(),
          );
          v_rst.push((name.unwrap(), data_type, Some(rst)));
        }
        (v_rst, vec![])
      },

      ExprNode::Reg => {
        assert!(operands.len() == 2);
        let mut v_rst = Vec::new();
        for ((op, clk), suggest_name) in operands[0]
          .fwd
          .to_owned()
          .into_iter()
          .zip(operands[1].fwd.to_owned().into_iter())
          .zip(suggester.into_iter())
        {
          let ((_name, data_type), entity) = op;
          let ((_name_clk, data_type_clk), entity_clk) = clk;

          assert!(data_type_clk.width() == 1);
          let name = suggest_name.or(Some(format!("reg")));
          let (rst, _) = c.add_wire(data_type.to_owned(), name.to_owned());
          c.add_op(
            SeqCompReg::new(Some(rst), entity, entity_clk, None, None).into(),
          );
          v_rst.push((name.unwrap(), data_type, Some(rst)));
        }
        (v_rst, vec![])
      },

      ExprNode::ArrayCreate => {
        assert!(operands.len() >= 1);
        let entities_per_operand = operands[0].fwd.len();
        assert!(operands.iter().all(|x| x.fwd.len() == entities_per_operand));

        let mut v_rst = Vec::new();
        for i in 0..entities_per_operand {
          let data_type = operands[0].fwd.to_owned().v_data_type[i].to_owned();

          let arr_data_type = DataTypeEnum::Array(ArrayType(
            Box::new(data_type.to_owned()),
            operands.len(),
          ));

          let name = suggester[i].to_owned().or(Some(format!("ac")));

          let (rst, _) = c.add_wire(arr_data_type, name.to_owned());

          c.add_op(
            HwArrayCreate::new(
              Some(rst),
              operands.iter().map(|x| x.fwd.v_entity_id[i]).collect(),
            )
            .into(),
          );
          v_rst.push((name.unwrap(), data_type, Some(rst)));
        }
        (v_rst, vec![])
      },

      ExprNode::ArrayConcat => {
        assert!(operands.len() >= 1);
        let entities_per_operand = operands[0].fwd.len();
        assert!(operands.iter().all(|x| x.fwd.len() == entities_per_operand));

        let mut v_rst = Vec::new();
        for i in 0..entities_per_operand {
          let DataTypeEnum::Array(ArrayType(sub_ty, _)) =
            operands[0].fwd.to_owned().v_data_type[i].to_owned()
          else {
            panic!("operands must be array for concat")
          };

          let len = operands
            .iter()
            .map(|op| {
              let DataTypeEnum::Array(ArrayType(_, len)) = op.fwd.v_data_type[i] else {
                panic!("operands must be array for concat")
              };
              len
            })
            .reduce(|x, y| x + y)
            .unwrap();

          let arr_data_type = DataTypeEnum::Array(ArrayType(Box::new(*sub_ty), len));

          let name = suggester[i].to_owned().or(Some(format!("concat")));

          let (rst, _) = c.add_wire(arr_data_type.to_owned(), name.to_owned());

          c.add_op(
            HwArrayConcat::new(
              Some(rst),
              operands.iter().map(|op| op.fwd.v_entity_id[i]).collect(),
            )
            .into(),
          );
          v_rst.push((name.unwrap(), arr_data_type, Some(rst)));
        }
        (v_rst, vec![])
      },

      ExprNode::ArraySlice(len) => {
        assert!(operands.len() == 2);

        let mut v_rst = Vec::new();

        for ((op0, op1), suggest_name) in operands[0]
          .fwd
          .to_owned()
          .into_iter()
          .zip(operands[1].fwd.to_owned().into_iter())
          .zip(suggester.into_iter())
        {
          let DataTypeEnum::Array(ArrayType(sub_typ, _)) = op0.0 .1 else {
            panic!("operands must be array for index")
          };

          let data_type = DataTypeEnum::Array(ArrayType(Box::new(*sub_typ), len));

          let name = suggest_name.or(Some(format!("slice")));

          let (rst, _) = c.add_wire(data_type.to_owned(), name.to_owned());
          c.add_op(HwArraySlice::new(Some(rst), op0.1, op1.1).into());
          v_rst.push((name.unwrap(), data_type, Some(rst)));
        }
        (v_rst, vec![])
      },

      ExprNode::ArrayGet => {
        assert!(operands.len() == 2);

        let mut v_rst = Vec::new();

        for ((op0, op1), suggest_name) in operands[0]
          .fwd
          .to_owned()
          .into_iter()
          .zip(operands[1].fwd.to_owned().into_iter())
          .zip(suggester.into_iter())
        {
          let DataTypeEnum::Array(ArrayType(sub_typ, _)) = op0.0 .1 else {
            panic!("operands must be array for index")
          };

          let data_type = *sub_typ;

          let name = suggest_name.or(Some(format!("get")));

          let (rst, _) = c.add_wire(data_type.to_owned(), name.to_owned());
          c.add_op(HwArrayGet::new(Some(rst), op0.1, op1.1).into());
          v_rst.push((name.unwrap(), data_type, Some(rst)));
        }
        (v_rst, vec![])
      },

      ExprNode::StructCreate(sig) => {
        // let node = ExprNode::StructCreate(sig.clone());
        assert!(operands.len() >= 1);
        let entities_per_operand = operands[0].fwd.len();
        assert!(operands.iter().all(|x| x.fwd.len() == entities_per_operand));

        let sig_bak = sig.clone();

        let mut v_rst = Vec::new();
        for i in 0..entities_per_operand {
          assert!(operands
            .iter()
            .enumerate()
            .all(|(j, op)| { op.fwd.v_data_type[i] == *sig_bak.0[j].1 }));

          let name = suggester[i].to_owned().or(Some(format!("struct_create")));

          let (rst, _) = c.add_wire(DataTypeEnum::Struct(sig.clone()), name.to_owned());

          c.add_op(
            HwStructCreate::new(
              Some(rst),
              operands.iter().map(|op| op.fwd.v_entity_id[i]).collect(),
            )
            .into(),
          );
          v_rst.push((name.unwrap(), DataTypeEnum::Struct(sig.clone()), Some(rst)));
        }
        (v_rst, vec![])
      },

      ExprNode::StructExtract(field) => {
        assert!(operands.len() == 1);
        let mut v_rst = Vec::new();

        for (op, suggest_name) in
          operands[0].fwd.to_owned().into_iter().zip(suggester.into_iter())
        {
          let ((name, data_type), entity) = op;

          let data_type = match data_type {
            DataTypeEnum::Struct(sig) => sig
              .0
              .iter()
              .find_map(|(name, ty)| if name == &field { Some(ty) } else { None })
              .unwrap()
              .to_owned(),
            _ => panic!("operands must be struct for extract"),
          };

          let name = suggest_name.or(Some(format!("{}.{}", name, field)));

          let (rst, _) = c.add_wire(*data_type.to_owned(), name.to_owned());
          c.add_op(
            HwStructExtract::new(Some(rst), entity, Some(field.to_owned().into()))
              .into(),
          );
          v_rst.push((name.unwrap(), *data_type, Some(rst)));
        }
        (v_rst, vec![])
      },

      ExprNode::StructInject(target_field) => {
        assert!(operands.len() == 2);
        let mut v_rst = Vec::new();
        for ((op0, op1), suggest_name) in operands[0]
          .fwd
          .to_owned()
          .into_iter()
          .zip(operands[1].fwd.to_owned().into_iter())
          .zip(suggester.into_iter())
        {
          let ((_name0, data_type0), entity0) = op0;
          let ((_name1, data_type1), entity1) = op1;

          let DataTypeEnum::Struct(sig) = data_type0 else {
            panic!("operands must be struct for inject")
          };

          let expected_data_type = sig
            .0
            .iter()
            .find_map(|(field, ty)| {
              if *field == target_field.to_owned() {
                Some((**ty).to_owned())
              } else {
                None
              }
            })
            .unwrap();

          assert!(expected_data_type == data_type1);

          let data_type = DataTypeEnum::Struct(sig.clone());

          let name = suggest_name.or(Some(format!("s_inject")));
          let (rst, _) = c.add_wire(data_type.to_owned(), name.to_owned());
          c.add_op(
            HwStructInject::new(
              Some(rst),
              entity0,
              entity1,
              Some(target_field.to_owned().into()),
            )
            .into(),
          );
          v_rst.push((name.unwrap(), data_type, Some(rst)));
        }
        (v_rst, vec![])
      },

      ExprNode::StructExplode => {
        assert!(operands.len() == 1);
        let mut v_rst = Vec::new();
        for (op, suggest_name) in
          operands[0].fwd.to_owned().into_iter().zip(suggester.into_iter())
        {
          let ((_name, data_type), entity) = op;

          let DataTypeEnum::Struct(sig) = data_type else {
            panic!("operands must be struct for explode")
          };

          let mut results = Vec::new();
          for (_field, sub_typ) in sig.0 {
            let name = suggest_name.to_owned().or(Some(format!("s_explode")));
            let (rst, _) = c.add_wire(*sub_typ.to_owned(), name.to_owned());
            v_rst.push((name.unwrap(), *sub_typ, Some(rst)));
            results.push(rst);
          }

          c.add_op(HwStructExplode::new(results.to_owned().into_iter().map(|x| Some(x)).collect(), entity).into());
        }
        (v_rst, vec![])
      },
      ExprNode::EventToSignal(event_id) => {
        assert!(operands.len() == 0);
        assert!(suggester.len() == 1);
        let name = suggester[0].to_owned().unwrap_or(format!("event"));
        let (signal_id, _) = c.add_wire(DataTypeEnum::UInt(UIntType(1)), Some(name.to_owned()));
        c.add_op(EventSignal::new(Some(event_id), Some(signal_id)).into());
        let v_rst = vec![(name, DataTypeEnum::UInt(UIntType(1)),Some(signal_id))];
        (v_rst, vec![])
      },
    };

    FBFields::new(fwd, bwd)
  }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Variadic {
  BitAnd,
  BitOr,
  BitXor,
  Add,
}

impl Variadic {
  pub fn to_str(&self) -> &str {
    match self {
      Variadic::BitAnd => "bitand",
      Variadic::BitOr => "bitor",
      Variadic::BitXor => "bitxor",
      Variadic::Add => "add",
    }
  }

  pub fn to_predicate(&self) -> CombVariadicPredicate {
    match self {
      Variadic::BitAnd => CombVariadicPredicate::And,
      Variadic::BitOr => CombVariadicPredicate::Or,
      Variadic::BitXor => CombVariadicPredicate::Xor,
      Variadic::Add => CombVariadicPredicate::Add,
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Binary {
  Shr,
  Shl,
  Sub,
}

impl Binary {
  pub fn to_str(&self) -> &str {
    match self {
      Binary::Shr => "shr",
      Binary::Shl => "shl",
      Binary::Sub => "sub",
    }
  }

  pub fn to_predicate(&self) -> CombBinaryPredicate {
    match self {
      Binary::Shr => CombBinaryPredicate::ShrU,
      Binary::Shl => CombBinaryPredicate::Shl,
      Binary::Sub => CombBinaryPredicate::Sub,
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Unary {
  Not,
  Neg,
}

impl Unary {
  pub fn to_str(&self) -> &str {
    match self {
      Unary::Not => "not",
      Unary::Neg => "neg",
    }
  }

  pub fn to_predicate(&self) -> CombUnaryPredicate {
    match self {
      Unary::Not => CombUnaryPredicate::Not,
      Unary::Neg => CombUnaryPredicate::Neg,
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cmpi {
  Eq,
  Ne,
  Lt,
  Le,
  Gt,
  Ge,
}

impl Cmpi {
  pub fn to_str(&self) -> &str {
    match self {
      Cmpi::Eq => "eq",
      Cmpi::Ne => "ne",
      Cmpi::Lt => "ult",
      Cmpi::Le => "ule",
      Cmpi::Gt => "ugt",
      Cmpi::Ge => "uge",
    }
  }

  pub fn to_predicate(&self) -> CombICmpPredicate {
    match self {
      Cmpi::Eq => irony_cmt::CombICmpPredicate::EQ,
      Cmpi::Ne => irony_cmt::CombICmpPredicate::NE,
      Cmpi::Lt => irony_cmt::CombICmpPredicate::ULT,
      Cmpi::Le => irony_cmt::CombICmpPredicate::ULE,
      Cmpi::Gt => irony_cmt::CombICmpPredicate::UGT,
      Cmpi::Ge => irony_cmt::CombICmpPredicate::UGE,
    }
  }
}

#[derive(Debug, Clone)]
pub struct Cast {
  // from: DataTypeEnum,
  pub target_data_type: DataTypeEnum,
}

impl Cast {
  pub fn to_str(&self) -> &str {
    // &format!("cast_{:?}_to_{:?}", self.from, self.to)[..]
    "cast"
  }
}

#[derive(Debug, Clone)]
pub struct Constant {
  pub ifc_fields: IfcFields,
  pub v_data: Vec<DataValue>,
}

impl Constant {
  pub fn to_str(&self) -> &str { "const" }
}

#[derive(Debug, Clone)]
pub struct Extract {
  pub target_data_type: DataTypeEnum,
  pub low: u32,
}

#[derive(Debug, Clone)]
pub struct Concat {}
