#![feature(macro_metavar_expr)]
#[allow(unused_variables)]
pub use irony::{self, preclude::*};

/// define types and attributes
mod common;
mod constraints;
mod passes;

pub use common::*;
pub use constraints::*;
pub use indexmap;
pub use passes::*;

mod cmt_utils;

// pub use interpret::*;

irony::entity_def! {
    [data_type = DataTypeEnum, attr = AttributeEnum]

    EntityEnum = {
        NONE: [],
        IRStmt: [name: StringAttr(StringAttr), debug: BoolAttr(BoolAttr), location: LocationAttr(LocationAttr)],
        IREvent: [name: StringAttr(StringAttr), debug: BoolAttr(BoolAttr), location: LocationAttr(LocationAttr)],
        IRWire: [name: StringAttr(StringAttr), debug: BoolAttr(BoolAttr), location: LocationAttr(LocationAttr)],
    }
}

irony::op_def! {
    [data_type = DataTypeEnum, attr = AttributeEnum, constraint = ConstraintEnum]

    OpEnum = {

        // ------ BEGIN: define the operations in `stmt` dialect -------

        StmtSynth: {
            defs: [],
            uses: [stmt, clk; protocol_events],
            attrs: [protocol_event_names: ArrayAttr(ArrayAttr)(*)],
            print: (
                |env:&E, attrs: Vec<(String, AttributeEnum)>, uses: Vec<(String, Vec<Option<EntityId>>)>, _, _| {

                    let stmt = env.print_entity(uses[0].1[0].unwrap());
                    let protocol_events = uses[2].1.to_owned().into_iter().map(|id| {
                        format!("{}", env.print_entity(id.unwrap()))
                    });
                    let AttributeEnum::ArrayAttr(protocol_event_names) = irony::utils::extract_vec(&attrs, "protocol_event_names").unwrap() else { panic!("")};
                    let mut protocol = protocol_event_names.0.iter().zip(protocol_events).map(|(name, event)| {
                        format!("{}: {}", name, event)
                    }).collect::<Vec<_>>().join(", ");

                    if let Some(clk) = uses[1].1[0].to_owned() {
                        protocol += format!(", clk: {}", env.print_entity(clk.to_owned())).as_ref();
                    }

                    format!("stmt.synth {} into protocol {{{}}}", stmt, protocol)
                }
            )
        },

        StmtStep: {
            defs: [lhs],
            uses: [; events, wait_at_exist],
            print: (
                |env:&E, _, uses: Vec<(String, Vec<Option<EntityId>>)>, defs: Vec<(String, Vec<Option<EntityId>>)>, _| {
                    let lhs = env.print_entity(defs[0].1[0].unwrap());

                    let events = uses[0].1.iter().map(|id| {
                        format!("{}", env.print_entity((*id).unwrap()))
                    }).collect::<Vec<_>>().join(", ");

                    let wait_at_exit = uses[1].1.iter().map(|id| {
                        format!("{}", env.print_entity((*id).unwrap()))
                    }).collect::<Vec<_>>().join(", ");

                    let wait_at_exit = if wait_at_exit.is_empty() {
                        format!("")
                    } else {
                        format!("wait-at-exit: {}", wait_at_exit)
                    };

                    format!("{} = stmt.step {} {{{}}}", lhs, events, wait_at_exit)

                }
            )
        },

        StmtSeq: {
            defs: [lhs],
            uses: [; sub_stmts],
            print: (
                |env:&E, _, uses: Vec<(String, Vec<Option<EntityId>>)>, defs: Vec<(String, Vec<Option<EntityId>>)>, _| {
                    let lhs = env.print_entity(defs[0].1[0].unwrap());
                    let sub_stmts = uses[0].1.iter().map(|id| {
                        format!("{}", env.print_entity((*id).unwrap()))
                    }).collect::<Vec<_>>().join(", ");
                    format!("{} = stmt.seq {}", lhs, sub_stmts)
                }
            )
        },

        StmtIf: {
            defs: [lhs],
            uses: [cond, then_stmt, else_stmt],
            print: (
                |env:&E, _, uses: Vec<(String, Vec<Option<EntityId>>)>, defs: Vec<(String, Vec<Option<EntityId>>)>, _| {
                    let lhs = env.print_entity(defs[0].1[0].unwrap());
                    let cond = env.print_entity(uses[0].1[0].unwrap());
                    let then_stmt = format!("then {{{}}}", env.print_entity(uses[1].1[0].unwrap()));
                    let else_stmt = match uses[2].1[0] {
                        Some(id) => format!("else {{{}}}", env.print_entity(id)),
                        None => format!(""),
                    };
                    format!("{} = stmt.if {} {} {}", lhs, cond, then_stmt, else_stmt)
                }
            )
        },

        StmtFor: {
            defs: [lhs],
            uses: [indvar_rd, indvar_wr, do_stmt, start, end],
            attrs: [incr: BoolAttr(BoolAttr)(*), const_start: UIntAttr(UIntAttr)(*), const_end: UIntAttr(UIntAttr)(*), const_step: UIntAttr(UIntAttr)(*)],
            print: (
                |env: &E, attrs: Vec<(String, AttributeEnum)>, uses: Vec<(String, Vec<Option<EntityId>>)>, defs:Vec<(String, Vec<Option<EntityId>>)>, _| {
                    let lhs = env.print_entity(defs[0].1[0].unwrap());
                    let indvar_rd = env.print_entity(uses[0].1[0].unwrap());
                    let indvar_wr = env.print_entity(uses[1].1[0].unwrap());
                    let do_stmt = env.print_entity(uses[2].1[0].unwrap());
                    let start = match irony::utils::extract_vec(&attrs, "const_start") {
                        Some(x) => format!("{}", x),
                        None => env.print_entity(uses[3].1[0].unwrap()),
                    };
                    let end = match irony::utils::extract_vec(&attrs, "const_end") {
                        Some(x) => format!("{}", x),
                        None => env.print_entity(uses[4].1[0].unwrap()),
                    };
                    let incr = if let AttributeEnum::BoolAttr(BoolAttr(x)) = irony::utils::extract_vec(&attrs, "incr").unwrap() {x} else { panic!("")};
                    let incr = if incr { "to" } else { "downto" };
                    let step = match irony::utils::extract_vec(&attrs, "const_step") {
                        Some(x) => format!("{}", x),
                        None => format!("1"),
                    };
                    format!("{} = stmt.for ({}, {}) = {} {} {} step {} do {}", lhs, indvar_rd, indvar_wr, start, incr, end, step, do_stmt)
                }
            )
        },

        StmtWhile: {
            defs: [lhs],
            uses: [cond, do_stmt],
            print: (
                |env: &E, _, uses: Vec<(String, Vec<Option<EntityId>>)>, defs:Vec<(String, Vec<Option<EntityId>>)>, _| {
                    let lhs = env.print_entity(defs[0].1[0].unwrap());
                    let cond = env.print_entity(uses[0].1[0].unwrap());
                    let do_stmt = env.print_entity(uses[1].1[0].unwrap());
                    format!("{} = stmt.while {} do {}", lhs, cond, do_stmt)
                }
            )
        },

        StmtPar: {
            defs: [lhs],
            uses: [;stmts],
            print: (
                |env: &E, _, uses: Vec<(String, Vec<Option<EntityId>>)>, defs:Vec<(String, Vec<Option<EntityId>>)>, _| {
                    let lhs = env.print_entity(defs[0].1[0].unwrap());
                    let stmts = uses[0].1.iter().map(|id| {
                        format!("{}", env.print_entity((*id).unwrap()))
                    }).collect::<Vec<_>>().join(", ");
                    format!("{} = stmt.par {}", lhs, stmts)
                }
            )
        },
        // ------ END: define the operations in `stmt` dialect -------


        // ------ BEGIN: define the operations in `event` dialect -------


        EventSignal: {
          defs: [],
          uses: [event, signal],
          print: (
              |env: &E, _, uses: Vec<(String, Vec<Option<EntityId>>)>, _, _| {
                  let event = env.print_entity(uses[0].1[0].unwrap());
                  let signal = env.print_entity(uses[1].1[0].unwrap());
                  format!("event.signal {} === {}", event, signal)
              }
          )
        },

        EventPort: {
            defs: [],
            uses: [event; wires],
            print: (
                |env: &E, _, uses: Vec<(String, Vec<Option<EntityId>>)>, _, _| {
                    let event = env.print_entity(uses[0].1[0].unwrap());
                    let wires = uses[1].1.iter().map(|id| {
                        format!("{}", env.print_entity((*id).unwrap()))
                    }).collect::<Vec<_>>().join(", ");
                    format!("event.port {} === [{}]", event, wires)
                }
            )
        },

        EventDef: {
            defs: [event],
            uses: [],
            print: (
                |env: &E, _, _, defs: Vec<(String, Vec<Option<EntityId>>)>, _| {
                    let event = env.print_entity(defs[0].1[0].unwrap());
                    format!("{} = event.def", event)
                }
            )
        },


        // ------ END: define the operations in `event` dialect -------


        TmpWhen: {
          defs: [],
          uses: [cond],
          regions: [body],
          print: (
              |env: &E, _, uses: Vec<(String, Vec<Option<EntityId>>)>, _, regions: Vec<(String, Vec<Option<RegionId>>)>| {
                  let cond = env.print_entity(uses[0].1[0].unwrap());
                  let body = env.print_region(regions[0].1[0].expect("must have region"));
                  format!("ILLEGAL.when {} {{\n{}\n}}", cond, body)
              }
          )
        },

        TmpSelect: {
            defs: [lhs],
            uses: [default; conds, values],
            attrs: [ onehot: BoolAttr(BoolAttr)(*)],
            constraints: [/* TODO */],
            print: (
                |env: &E, attrs: Vec<(String, AttributeEnum)>, uses: Vec<(String, Vec<Option<EntityId>>)>, defs: Vec<(String, Vec<Option<EntityId>>)>, _| {
                    let lhs = env.print_entity(defs[0].1[0].unwrap());

                    let mode = if let AttributeEnum::BoolAttr(BoolAttr(x)) = irony::utils::extract_vec(&attrs, "onehot").unwrap() {
                        if x {
                            "onehot"
                        } else {
                            "priority"
                        }
                    } else {
                        "priority"
                    };

                    let candidates = uses[2].1.iter().zip(uses[1].1.iter()).map(|(value, cond)| {
                        format!("\t{} : {}",
                          match cond {
                            Some(cond) => env.print_entity(*cond),
                            None => format!("[TBD]"),
                          },
                          env.print_entity(value.unwrap()))
                    }).collect::<Vec<_>>().join(", \n");


                    let default =  if let Some(default) = uses[0].1[0] {
                        format!("\tdefault : {}\n", env.print_entity(default))
                    } else  { String::default() } ;

                    let typ = env.get_entity(defs[0].1[0].unwrap()).get_dtype().unwrap();
                    format!("{} = ILLEGAL.select {} {{\n{}\n{}}} : {}", lhs, mode, candidates, default, typ)
                }
            )
        },

        TmpUnary: {
            defs: [lhs],
            uses: [op],
            attrs: [predicate: CombUnaryPredicate(CombUnaryPredicate)(*)],
            constraints: [SameType::new().into()],
            print: (
                |env: &E, attrs: Vec<(String, AttributeEnum)>, uses: Vec<(String, Vec<Option<EntityId>>)>, defs: Vec<(String, Vec<Option<EntityId>>)>, _| {
                    let def = env.print_entity(defs[0].1[0].unwrap());
                    let uses = vec![env.print_entity(uses[0].1[0].unwrap())].join(", ");
                    let AttributeEnum::CombUnaryPredicate(predicate) = irony::utils::extract_vec(&attrs, "predicate").unwrap() else { panic!("")};
                    let typ = env.get_entity(defs[0].1[0].unwrap()).get_dtype().unwrap();
                    format!("{} = ILLEGAL.{} {} : {}", def, predicate, uses, typ)
                }
            )
        },

        // ------ END: define the operations in `temporary` dialect -------

        // ------ BEGIN: define the operations in `sv` dialect -------

        SvConstantX: {
          defs: [lhs],
          uses: [],
          print:(
              |env: &E, _, _, defs: Vec<(String, Vec<Option<EntityId>>)>, _| {
                  let lhs = env.print_entity(defs[0].1[0].unwrap());
                  let typ = env.get_entity(defs[0].1[0].unwrap()).get_dtype().unwrap();
                  format!("{} = sv.constantX : {}", lhs, typ)
              }
          )
        },


        // ______ END: define the operations in `sv` dialect ______

        // ------ BEGIN: define the operations in `hw` dialect -------
        Assign: {
            defs: [lhs],
            uses: [rhs],
            constraints: [SameType::new().into()],
            print: (
                |env: &E, _, uses: Vec<(String, Vec<Option<EntityId>>)>,  defs:Vec<(String, Vec<Option<EntityId>>)>, _ | {
                    // format!("{} = {}", env.print_entity(defs[0].1[0].unwrap()), env.print_entity(uses[0].1[0].unwrap()))
                    let lhs = env.print_entity(defs[0].1[0].unwrap());
                    let rhs = env.print_entity(uses[0].1[0].unwrap());
                    let typ = env.get_entity(defs[0].1[0].unwrap()).get_dtype().unwrap();
                    format!("{} = hw.wire {} : {}", lhs, rhs, typ)
                }
            )
        },

        HwModule: {
            defs: [],
            uses: [],
            attrs: [name: StringAttr(StringAttr), is_extern: BoolAttr(BoolAttr)(*), top: BoolAttr(BoolAttr), arg_names: ArrayAttr(ArrayAttr), arg_types: ArrayAttr(ArrayAttr)(*), output_names: ArrayAttr(ArrayAttr), output_types: ArrayAttr(ArrayAttr)(*)],
            regions: [body],
            constraints: [ModuleConstraint::default().into()],
            print: (
                |env: &E, attrs: Vec<(String, AttributeEnum)>, _ , _, regions: Vec<(String, Vec<Option<RegionId>>)>| {

                    let AttributeEnum::BoolAttr(BoolAttr(is_extern)) = irony::utils::extract_vec(&attrs, "is_extern").unwrap() else {
                      panic!("")
                    };

                    let AttributeEnum::ArrayAttr(arg_names) = irony::utils::extract_vec(&attrs, "arg_names").unwrap() else { panic!("")};
                    let AttributeEnum::ArrayAttr(arg_types) = irony::utils::extract_vec(&attrs, "arg_types").unwrap() else { panic!("")};

                    let AttributeEnum::ArrayAttr(output_names) = irony::utils::extract_vec(&attrs, "output_names").unwrap() else { panic!("")};
                    let AttributeEnum::ArrayAttr(output_types) = irony::utils::extract_vec(&attrs, "output_types").unwrap() else { panic!("")};
                    let name = irony::utils::extract_vec(&attrs, "name").unwrap();

                    let args = arg_names.0.iter().zip(arg_types.0.iter()).map(|(name, ty)| {
                        format!("%{}: {}", name, ty)
                    }).collect::<Vec<_>>().join(", ");

                    let outputs = output_names.0.iter().zip(output_types.0.iter()).map(|(name, ty)| {
                        format!("{}: {}", name, ty)
                    }).collect::<Vec<_>>().join(", ");

                    if is_extern {
                      format!("hw.module.extern @{}({}) -> ({})", name, args, outputs)
                    } else {
                      format!("hw.module @{}({}) -> ({}) {{\n{}\n}}", name, args, outputs, env.print_region(regions[0].1[0].expect("must have region")))
                    }
                }
            )
        },

        // TODO: Support EXT_W_PARAMS ?
        HwInstance: {
            defs: [; outputs],
            uses: [; inputs],
            attrs: [target_op_id: OpIdAttr(OpIdAttr)(*), name: StringAttr(StringAttr)],
            constraints: [InstanceConstraint::default().into()],
            print: (
                |env: &E, attrs: Vec<(String, AttributeEnum)>, uses: Vec<(String, Vec<Option<EntityId>>)>, defs: Vec<(String, Vec<Option<EntityId>>)>, _| {
                    let AttributeEnum::OpIdAttr(target_op_id) = irony::utils::extract_vec(&attrs, "target_op_id").unwrap() else { panic!("")};
                    let module_attrs = env.get_op(target_op_id.into()).get_attrs();
                    let AttributeEnum::StringAttr(instance_name) = irony::utils::extract_vec(&attrs, "name").unwrap() else { panic!("")};

                    let AttributeEnum::ArrayAttr(arg_names) = irony::utils::extract_vec(&module_attrs, "arg_names").unwrap() else { panic!("")};
                    let AttributeEnum::ArrayAttr(arg_types) = irony::utils::extract_vec(&module_attrs, "arg_types").unwrap() else { panic!("")};

                    let AttributeEnum::ArrayAttr(output_names) = irony::utils::extract_vec(&module_attrs, "output_names").unwrap() else { panic!("")};
                    let AttributeEnum::ArrayAttr(output_types) = irony::utils::extract_vec(&module_attrs, "output_types").unwrap() else { panic!("")};
                    let AttributeEnum::StringAttr(mod_name) = irony::utils::extract_vec(&module_attrs, "name").unwrap() else { panic!("")};

                    let outputs = defs[0].1.iter().map(|id| {
                        env.print_entity((*id).unwrap())
                    }).collect::<Vec<_>>().join(", ");

                    let output_types = output_names.0.iter().zip(output_types.0.iter()).map(|(name, ty)| {
                        format!("{}: {}", name, ty)
                    }).collect::<Vec<_>>().join(", ");

                    let args = arg_names.0.iter().zip(uses[0].1.iter()).zip(arg_types.0.iter()).map(|((name, id), ty)| {
                        format!("{} : {} : {}", name, env.print_entity((*id).unwrap()), ty)
                    }).collect::<Vec<_>>().join(", ");

                    format!("{} = hw.instance \"{}\" @{}({}) -> ({})", outputs, instance_name, mod_name, args, output_types)
                }
            )
        },

        HwInput: {
            defs: [; inputs],
            uses: [],
            print: (
                |env: &E, _, _, defs: Vec<(String, Vec<Option<EntityId>>)>,  _| {
                    let inputs = defs[0].1.iter().map(|id| {
                        format!("{}", env.print_entity((*id).unwrap()))
                    }).collect::<Vec<_>>().join(", ");
                    let input_types = defs[0].1.iter().map(|id| {
                        format!("{}", env.get_entity((*id).unwrap()).get_dtype().unwrap())
                    }).collect::<Vec<_>>().join(", ");
                    format!("// hw.input {} : {}", inputs, input_types)
                }
            )
        },

        HwOutput: {
            defs: [],
            uses: [; outputs],
            print: (
                |env: &E, _, uses: Vec<(String, Vec<Option<EntityId>>)>, _, _| {
                    let outputs = uses[0].1.iter().map(|id| {
                        format!("{}", env.print_entity((*id).unwrap()))
                    }).collect::<Vec<_>>().join(", ");
                    let output_types = uses[0].1.iter().map(|id| {
                        format!("{}", env.get_entity((*id).unwrap()).get_dtype().unwrap())
                    }).collect::<Vec<_>>().join(", ");
                    format!("hw.output {} : {}", outputs, output_types)
                }
            )
        },

        HwBitCast: {
            defs: [lhs],
            uses: [rhs],
            print: (
                |env: &E, _, uses: Vec<(String, Vec<Option<EntityId>>)>, defs: Vec<(String, Vec<Option<EntityId>>)>, _| {
                    let lhs = env.print_entity(defs[0].1[0].unwrap());
                    let rhs = env.print_entity(uses[0].1[0].unwrap());
                    let rhs_typ = env.get_entity(uses[0].1[0].unwrap()).get_dtype().unwrap();
                    let lhs_typ = env.get_entity(defs[0].1[0].unwrap()).get_dtype().unwrap();

                    format!("{} = hw.bitcast {}: ({}) -> {}", lhs, rhs, rhs_typ, lhs_typ)
                }
            )
        },

        // TODO: support super large constant and boolean constant
        HwConstant: {
            defs: [lhs],
            uses: [],
            attrs: [value: ConstantAttr(ConstantAttr)(*)],
            constraints: [SameTypeConstant::default().into()],
            print: (
                |env: &E, attrs: Vec<(String, AttributeEnum)>, _, defs: Vec<(String, Vec<Option<EntityId>>)>, _| {
                    let AttributeEnum::ConstantAttr(value) = irony::utils::extract_vec(&attrs, "value").unwrap() else { panic!("")};
                    let value = irony::utils::arith::from_bits_to_u32(value.0);
                    let names = defs[0].1.iter().map(|id| {
                        format!("{}", env.print_entity((*id).unwrap()))
                    }).collect::<Vec<_>>().join(", ");
                    let types = defs[0].1.iter().map(|id| {
                        format!("{}", env.get_entity((*id).unwrap()).get_dtype().unwrap())
                    }).collect::<Vec<_>>().join(", ");
                    format!("{} = hw.constant {}: {}", names, value, types)
                }
            )
        },

        HwAggregateConstant: {
            defs: [lhs],
            uses: [],
            attrs: [attrs: ArrayAttr(ArrayAttr)(*)],
            constraints: [SameTypeAggregate::default().into()],
            print: (
                |env: &E, attrs: Vec<(String, AttributeEnum)>, _, defs: Vec<(String, Vec<Option<EntityId>>)>, _| {
                    let attrs = irony::utils::extract_vec(&attrs, "attrs").unwrap();
                    let name = format!("{}", env.print_entity(defs[0].1[0].unwrap()));
                    let types = format!("{}", env.get_entity(defs[0].1[0].unwrap()).get_dtype().unwrap());
                    let values = attrs.print_for_aggregate_constant(env.get_entity(defs[0].1[0].unwrap()).get_dtype().unwrap());
                    format!("{} = hw.aggregate_constant {} : {}", name, values, types)
                }
            )
        },

        HwArrayConcat: {
            defs: [lhs],
            uses: [; operands],
            constraints: [ArrayConcatConstraint::default().into()],
            print: (
                |env: &E, _, uses: Vec<(String, Vec<Option<EntityId>>)>, defs: Vec<(String, Vec<Option<EntityId>>)>, _| {
                    let rst = env.print_entity(defs[0].1[0].unwrap());
                    let operands = uses[0].1.iter().map(|id| {
                        format!("{}", env.print_entity((*id).unwrap()))
                    }).collect::<Vec<_>>().join(", ");
                    let sub_typs = uses[0].1.iter().map(|id| {
                        format!("{}", env.get_entity((*id).unwrap()).get_dtype().unwrap())
                    }).collect::<Vec<_>>().join(", ");
                    format!("{} = hw.array_concat {} : {}", rst, operands, sub_typs)
                }
            )
        },

        HwArrayCreate: {
            defs: [lhs],
            uses: [; operands],
            constraints: [ArrayCreateConstraint::default().into(), SameTypeOperands::new().into()],
            print: (
                |env: &E, _, uses: Vec<(String, Vec<Option<EntityId>>)>, defs: Vec<(String, Vec<Option<EntityId>>)>, _| {
                    let rst = env.print_entity(defs[0].1[0].unwrap());
                    let operands = uses[0].1.iter().map(|id| {
                        format!("{}", env.print_entity((*id).unwrap()))
                    }).collect::<Vec<_>>().join(", ");
                    let sub_typ = env.get_entity(uses[0].1[0].unwrap()).get_dtype().unwrap();
                    format!("{} = hw.array_create {} : {}", rst, operands, sub_typ)
                }
            )
        },

        HwArrayGet: {
            defs: [lhs],
            uses: [array, index],
            constraints: [ArrayGetConstraint::default().into()],
            print: (
                |env: &E, _, uses: Vec<(String, Vec<Option<EntityId>>)>, defs: Vec<(String, Vec<Option<EntityId>>)>, _| {
                    let rst = env.print_entity(defs[0].1[0].unwrap());
                    let array = env.print_entity(uses[0].1[0].unwrap());
                    let index = env.print_entity(uses[1].1[0].unwrap());
                    let array_typ = env.get_entity(uses[0].1[0].unwrap()).get_dtype().unwrap();
                    let index_typ = env.get_entity(uses[1].1[0].unwrap()).get_dtype().unwrap();
                    format!("{} = hw.array_get {}[{}] : {}, {}", rst, array, index, array_typ, index_typ)
                }
            )
        },

        HwArraySlice: {
            defs: [lhs],
            uses: [array, index],
            constraints: [ArraySliceConstraint::default().into()],
            print: (
                |env: &E, _, uses: Vec<(String, Vec<Option<EntityId>>)>, defs: Vec<(String, Vec<Option<EntityId>>)>, _| {
                    let rst = env.print_entity(defs[0].1[0].unwrap());
                    let array = env.print_entity(uses[0].1[0].unwrap());
                    let index = env.print_entity(uses[1].1[0].unwrap());
                    let old_typ = env.get_entity(uses[0].1[0].unwrap()).get_dtype().unwrap();
                    let new_typ = env.get_entity(defs[0].1[0].unwrap()).get_dtype().unwrap();
                    format!("{} = hw.array_slice {}[{}] : ({}) -> {}", rst, array, index, old_typ, new_typ)
                }
            )
        },

        HwStructCreate: {
            defs: [lhs],
            uses: [; operands],
            constraints: [StructCreateConstraint::default().into()],
            print: (
                |env: &E, _, uses: Vec<(String, Vec<Option<EntityId>>)>, defs: Vec<(String, Vec<Option<EntityId>>)>, _| {

                    let lhs = env.print_entity(defs[0].1[0].unwrap());
                    let operands = uses[0].1.iter().map(|id| {
                        format!("{}", env.print_entity((*id).unwrap()))
                    }).collect::<Vec<_>>().join(", ");
                    let lhs_ty = env.get_entity(defs[0].1[0].unwrap()).get_dtype().unwrap();
                    format!("{} = hw.struct_create ({}) : {}", lhs, operands, lhs_ty)
                }
            )
        },

        HwStructExtract: {
            defs: [lhs],
            uses: [struct_input],
            attrs: [field: StringAttr(StringAttr)(*)],
            constraints: [StructExtractConstraint::default().into()],
            print: (
                |env: &E, attrs: Vec<(String, AttributeEnum)>, uses: Vec<(String, Vec<Option<EntityId>>)>, defs: Vec<(String, Vec<Option<EntityId>>)>, _| {
                    let lhs = env.print_entity(defs[0].1[0].unwrap());
                    let struct_input = env.print_entity(uses[0].1[0].unwrap());
                    let field = irony::utils::extract_vec(&attrs, "field").unwrap();
                    let struct_ty = env.get_entity(uses[0].1[0].unwrap()).get_dtype().unwrap();
                    format!("{} = hw.struct_extract {}[\"{}\"] : {}", lhs, struct_input, field, struct_ty)
                }
            )
        },

        HwStructInject: {
            defs: [lhs],
            uses: [struct_input, new_value],
            attrs: [field: StringAttr(StringAttr)(*)],
            constraints: [StructInjectConstraint::default().into()],
            print: (
                |env: &E, attrs: Vec<(String, AttributeEnum)>, uses: Vec<(String, Vec<Option<EntityId>>)>, defs: Vec<(String, Vec<Option<EntityId>>)>, _| {
                    let lhs = env.print_entity(defs[0].1[0].unwrap());
                    let struct_input = env.print_entity(uses[0].1[0].unwrap());
                    let new_value = env.print_entity(uses[1].1[0].unwrap());
                    let field = irony::utils::extract_vec(&attrs, "field").unwrap();
                    let struct_ty = env.get_entity(uses[0].1[0].unwrap()).get_dtype().unwrap();
                    format!("{} = hw.struct_inject {}[\"{}\"], {} : {}", lhs, struct_input, field, new_value, struct_ty)
                }
            )
        },

        HwStructExplode: {
            defs: [; outputs],
            uses: [struct_input],
            constraints: [StructExplodeConstraint::default().into()],
            print: (
                |env: &E, _, uses: Vec<(String, Vec<Option<EntityId>>)>, defs: Vec<(String, Vec<Option<EntityId>>)>, _| {
                    let outputs = defs[0].1.iter().map(|id| {
                        format!("{}", env.print_entity((*id).unwrap()))
                    }).collect::<Vec<_>>().join(", ");
                    let struct_input = env.print_entity(uses[0].1[0].unwrap());
                    let struct_ty = env.get_entity(uses[0].1[0].unwrap()).get_dtype().unwrap();

                    format!("{} = hw.struct_explode {} : {}", outputs, struct_input, struct_ty)
                }
            )
        },

        // ------ END: define the operations in `hw` dialect -------

        // ------ BEGIN: define the operations in `comb` dialect -------
        // TODO: Add more constraints for safer usage
        CombVariadic: {
            defs: [lhs],
            uses: [; operands],
            attrs: [predicate: CombVariadicPredicate(CombVariadicPredicate)(*)],
            constraints: [SameType::new().into()],
            print: (
                |env: &E, attrs: Vec<(String, AttributeEnum)>, uses: Vec<(String, Vec<Option<EntityId>>)>,  defs: Vec<(String, Vec<Option<EntityId>>)>, _| {
                    let def = env.print_entity(defs[0].1[0].unwrap());
                    let uses = uses[0].1.iter().map(|id| {
                        format!("{}", env.print_entity((*id).unwrap()))
                    }).collect::<Vec<_>>().join(", ");
                    let AttributeEnum::CombVariadicPredicate(predicate) = irony::utils::extract_vec(&attrs, "predicate").unwrap() else { panic!("")};
                    let typ = env.get_entity(defs[0].1[0].unwrap()).get_dtype().unwrap();
                    format!("{} = comb.{} {} : {}", def, predicate, uses, typ)
                }
            )
        },
        CombBinary: {
            defs: [lhs],
            uses: [op0, op1],
            attrs: [predicate: CombBinaryPredicate(CombBinaryPredicate)(*)],
            constraints: [SameType::new().into()],
            print: (
                |env: &E, attrs: Vec<(String, AttributeEnum)>, uses: Vec<(String, Vec<Option<EntityId>>)>, defs: Vec<(String, Vec<Option<EntityId>>)>, _| {
                    let def = env.print_entity(defs[0].1[0].unwrap());
                    let uses = vec![env.print_entity(uses[0].1[0].unwrap()), env.print_entity(uses[1].1[0].unwrap())].join(", ");
                    let AttributeEnum::CombBinaryPredicate(predicate) = irony::utils::extract_vec(&attrs, "predicate").unwrap() else { panic!("")};
                    let typ = env.get_entity(defs[0].1[0].unwrap()).get_dtype().unwrap();
                    format!("{} = comb.{} {} : {}", def, predicate, uses, typ)
                }
            )
        },

        CombICmp: {
            defs: [lhs],
            uses: [op0, op1],
            attrs: [predicate: CombICmpPredicate(CombICmpPredicate)(*)],
            constraints: [SameTypeOperands::new().into()],
            print: (
                |env: &E, attrs: Vec<(String, AttributeEnum)>, uses: Vec<(String, Vec<Option<EntityId>>)>, defs: Vec<(String,Vec<Option<EntityId>>)>, _| {
                    let def = env.print_entity(defs[0].1[0].unwrap());
                    let inputs = vec![env.print_entity(uses[0].1[0].unwrap()), env.print_entity(uses[1].1[0].unwrap())].join(", ");
                    let AttributeEnum::CombICmpPredicate(predicate) = irony::utils::extract_vec(&attrs, "predicate").unwrap() else { panic!("")};
                    let typ = env.get_entity(uses[0].1[0].unwrap()).get_dtype().unwrap();
                    format!("{} = comb.icmp {} {} : {}", def, predicate, inputs, typ)
                }
            )
        },
        // CombParity: {
        //     defs: [lhs],
        //     uses: [rhs],
        //     constraints: [/* TODO: fill this */],
        //     print: (
        //         |_, _, _, _, _| {
        //             unimplemented!()
        //         }
        //     )
        // },
        CombExtract: {
            defs: [lhs],
            uses: [input],
            attrs: [low: UIntAttr(UIntAttr)],
            print: (
                |env: &E, attrs: Vec<(String, AttributeEnum)>, uses: Vec<(String, Vec<Option<EntityId>>)>, defs: Vec<(String,Vec<Option<EntityId>>)>, _| {
                    let lhs = env.print_entity(defs[0].1[0].unwrap());
                    let input = env.print_entity(uses[0].1[0].unwrap());
                    let AttributeEnum::UIntAttr(low) = irony::utils::extract_vec(&attrs, "low").unwrap() else {panic!("")};
                    let input_type = env.get_entity(uses[0].1[0].unwrap()).get_dtype().unwrap();
                    let lhs_type = env.get_entity(defs[0].1[0].unwrap()).get_dtype().unwrap();

                    format!("{} = comb.extract {} from {} : ({}) -> {}", lhs, input, low, input_type, lhs_type)
                }
            )
        },
        CombConcat: {
            defs: [lhs],
            uses: [; operands],
            print: (
                |env: &E, _, uses: Vec<(String, Vec<Option<EntityId>>)>, defs: Vec<(String, Vec<Option<EntityId>>)>, _| {
                    let lhs = env.print_entity(defs[0].1[0].unwrap());

                    let operands = uses[0].1.iter().map(|id| {
                        format!("{}", env.print_entity((*id).unwrap()))
                    }).collect::<Vec<_>>().join(", ");

                    let op_types = uses[0].1.iter().map(|id| {
                        format!("{}", env.get_entity((*id).unwrap()).get_dtype().unwrap())
                    }).collect::<Vec<_>>().join(", ");

                    format!("{} = comb.concat {} : {}", lhs, operands, op_types)
                }
            )
        },
        // CombReplicate: {
        //     defs: [lhs],
        //     uses: [rhs],
        //     constraints: [/* TODO: fill this */],
        //     print: (
        //         |_, _, _, _, _| {
        //             unimplemented!()
        //         }
        //     )
        // },
        CombMux2: {
            defs: [lhs],
            uses: [cond, op0, op1],
            constraints: [/* TODO: fill this */],
            print: (
                |env: &E, _, uses: Vec<(String, Vec<Option<EntityId>>)>, defs: Vec<(String, Vec<Option<EntityId>>)>, _| {
                    let def = env.print_entity(defs[0].1[0].unwrap());
                    let uses = uses.iter().map(|(_, ids)| {
                        format!("{}", env.print_entity(ids[0].unwrap()))
                    }).collect::<Vec<_>>().join(", ");
                    let typ = env.get_entity(defs[0].1[0].unwrap()).get_dtype().unwrap();
                    format!("{} = comb.mux {} : {}", def, uses, typ)
                }
            )
        },
        // ------ END: define the operations in `comb` dialect -------

        // ------ BEGIN: define the operations in `seq` dialect -------
        SeqCompReg: {
            defs: [output],
            uses: [input, clk,reset,reset_val],
            constraints: [/* TODO: fill this */],
            print: (
                |env: &E, _, uses: Vec<(String, Vec<Option<EntityId>>)>, defs: Vec<(String, Vec<Option<EntityId>>)>, _| {
                    let output_name = env.print_entity(defs[0].1[0].unwrap());
                    let input_name = match uses.iter().find(|(name, _)| name == "input").and_then(|(_, ids)| Some(env.print_entity(ids[0].unwrap()))) {
                        Some(name) => name,
                        None => panic!("must provide op for seq.compreg"),
                    };
                    let clk = match uses.iter().find(|(name, _)| name == "clk").and_then(|(_, ids)| Some(env.print_entity(ids[0].unwrap()))) {
                        Some(name) => format!(",{}", name),
                        None => panic!("must provide clk for seq.compreg"),
                    };
                    let reset = match uses.iter().find(|(name, _)| name == "reset").and_then(|(_, ids)| {
                        if let Some(id) = ids[0] {
                            Some(env.print_entity(id))
                        } else {
                            None
                        }}) {
                        Some(name) => format!(",{}", name),
                        None => format!(""),
                    };
                    let reset_val = match uses.iter().find(|(name, _)| name == "reset_val").and_then(|(_, ids)| {
                        if let Some(id) = ids[0] {
                            Some(env.print_entity(id))
                        } else {
                            None
                        }}) {
                        Some(name) => format!(",{}", name),
                        None => format!(""),
                    };

                    let typ = env.get_entity(defs[0].1[0].unwrap()).get_dtype().unwrap();

                    format!("{} = seq.compreg {} {} {} {} : {}", output_name, input_name, clk, reset, reset_val, typ)
                }
            )
        },

        // SeqHlmem: {
        //     defs: [handle],
        //     uses: [clk, reset],
        //     constraints: [/* TODO: fill this */],
        //     print: (
        //         |_, _, _, _, _| {
        //             format!("")
        //         }
        //     )
        // },

        // SeqRead: {
        //     defs: [rdata],
        //     uses: [mem, renable; address],
        //     attrs: [latency: IdAttr(IdAttr)],
        //     print: (
        //         |_, _, _, _, _| {
        //             format!("")
        //         }
        //     )
        // },

        // SeqWrite: {
        //     defs: [],
        //     uses: [mem, wenable, wdata; address],
        //     attrs: [latency: IdAttr(IdAttr)],
        //     print: (
        //         |_, _, _, _, _| {
        //             format!("")
        //         }
        //     )
        // },

        // ------ END: define the operations in `seq` dialect -------


        // ------ BEGIN: define the operations in `interpret` dialect -------
        ItprtCondCheck: {
            defs: [],
            uses: [; conds],
            attrs: [has_default: BoolAttr(BoolAttr)(*), onehot: BoolAttr(BoolAttr)(*)],
            print: (
                |env: &E, attrs: Vec<(String, AttributeEnum)>, uses: Vec<(String, Vec<Option<EntityId>>)>, _, _| {

                    let conds = uses[0].1.iter().map(|id| {
                        format!("{}", env.print_entity((*id).unwrap()))
                    }).collect::<Vec<_>>().join(", ");

                    let has_default = irony::utils::extract_vec(&attrs, "has_default").unwrap();

                    let onehot = irony::utils::extract_vec(&attrs, "onehot").unwrap();
                    format!("itprt.cond_check {} {{has_default = {}, onehot = {}}}", conds, has_default, onehot)
                }
            )
        },

        // ------ END: define the operations in `interpret` dialect -------


    }
}

irony::environ_def! {
    [data_type = DataTypeEnum, attr = AttributeEnum, entity = EntityEnum, op = OpEnum, constraint = ConstraintEnum, pm = PassManager]
    struct CmtIR;
}

pub(crate) const NONE: NONE = NONE::const_new(None);

#[derive(Default, Debug)]
pub struct IdReducer {
  entity_set: FxHashMap<EntityId, usize>,
  op_set: FxHashMap<OpId, usize>,
}

impl ReducerTrait for IdReducer {
  fn reduce_entity(&mut self, id: EntityId) -> usize {
    let len = self.entity_set.len();
    match self.entity_set.entry(id) {
      std::collections::hash_map::Entry::Occupied(entry) => *entry.get(),
      std::collections::hash_map::Entry::Vacant(entry) => {
        let new_id = len;
        entry.insert(new_id);
        new_id
      },
    }
  }

  fn reduce_op(&mut self, id: OpId) -> usize {
    let len = self.op_set.len();
    match self.op_set.entry(id) {
      std::collections::hash_map::Entry::Occupied(entry) => *entry.get(),
      std::collections::hash_map::Entry::Vacant(entry) => {
        let new_id = len;
        entry.insert(new_id);
        new_id
      },
    }
  }
}

impl CmtIR {
  pub fn new() -> Self {
    let mut this = Self::default();
    this.add_entity(NONE.into());

    this.begin_region(None);
    this
  }

  pub fn hash_op(&mut self, op: OpId) -> Option<OpId> {
    self.hasher.replace(irony::FxHasherBuilder::default().build_hasher());
    let mut id_reducer = IdReducer::default();

    self.get_op(op).hash_with_reducer(self, &mut id_reducer);

    let hash_value = self.hasher.borrow_mut().finish();

    let parent = self.get_op(op).get_parent();

    // println!("hash_op op: {:#?}, parent: {:#?}, hash_value: {:#?}", op, parent, hash_value);

    let (deletion, final_op_id) =
      match self.op_hash_table.entry(OpHashT(parent, hash_value)) {
        std::collections::hash_map::Entry::Occupied(entry) => {
          (true, Some(OpId::from(*entry.get())))
        },
        std::collections::hash_map::Entry::Vacant(entry) => {
          entry.insert(op);
          (false, Some(op))
        },
      };

    if deletion {
      self.delete_op_and_all(op);
    }

    final_op_id
  }
}

impl Drop for CmtIR {
  fn drop(&mut self) { self.end_region(); }
}

#[cfg(test)]
mod tests;
