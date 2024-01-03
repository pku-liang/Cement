use core::panic;

use irony::{EntityId, Op};

use crate::BoolAttr;

use super::{AttributeEnum, DataTypeEnum};

pub type SameType = irony::SameTypeConstraint<DataTypeEnum, AttributeEnum>;
pub type SameTypeOperands = irony::SameTypeOperandConstraint<DataTypeEnum, AttributeEnum>;

irony::constraint_def! {
    [data_type = DataTypeEnum, attr = AttributeEnum]
    ConstraintEnum = {
        SameType(SameType),
        SameTypeOperands(SameTypeOperands),
        ModuleConstraint(ModuleConstraint,
            |env, attrs: Vec<(String, crate::AttributeEnum)>, _, _, regions: Vec<(String, Vec<Option<irony::RegionId>>)>|  {

            let region = regions[0].1[0];

            let Some(AttributeEnum::BoolAttr(BoolAttr(is_extern))) = irony::utils::extract_vec(&attrs, "is_extern") else {
              panic!("");
            };

            if is_extern {
              // region.is_none()
              true
            } else {
              let region = region.unwrap();
              irony::utils::extract_vec(&attrs, "arg_names") == super::cmt_utils::extract_input_names(env, region) &&
              irony::utils::extract_vec(&attrs, "arg_types") == super::cmt_utils::extract_input_types(env, region) &&
              irony::utils::extract_vec(&attrs, "output_types") == super::cmt_utils::extract_output_types(env, region)

            }
        }),
        InstanceConstraint(InstanceConstraint ,
            |env: &E, attrs, uses: Vec<(String, Vec<Option<EntityId>>)>, defs: Vec<(String, Vec<Option<EntityId>>)>, _| {
            let target_op_id = irony::utils::extract_vec(&attrs, "target_op_id");
            let Some(AttributeEnum::OpIdAttr(target_op_id)) = target_op_id else {
                panic!("target_id must be a OpIdAttr")
            };

            // let target_region= env.get_op(target_op_id.into()).get_regions()[0].1[0];
            let target_attrs = env.get_op(target_op_id.into()).get_attrs();

            irony::utils::extract_vec(&target_attrs, "arg_types") == super::cmt_utils::extract_types(env, uses[0].1.to_owned())
            &&
            irony::utils::extract_vec(&target_attrs, "output_types") == super::cmt_utils::extract_types(env, defs[0].1.to_owned())

        }),

        SameTypeConstant(SameTypeConstant,
            |_, _, _, _, _|  {
                true
        }),
        SameTypeAggregate(SameTypeAggregate,
            |_, _, _, _, _|  {
                true
        }),
        ArrayConcatConstraint(ArrayConcatConstraint ,
            |_, _, _, _, _|  {
                true
        }),
        ArrayCreateConstraint(ArrayCreateConstraint ,
            |_, _, _, _, _|  {
                true
        }),
        ArrayGetConstraint(ArrayGetConstraint ,
            |_, _, _, _, _|  {
                true
        }),
        ArraySliceConstraint(ArraySliceConstraint ,
            |_, _, _, _, _|  {
                true
        }),
        StructCreateConstraint(StructCreateConstraint ,
            |_, _, _, _, _|  {
                true
        }),
        StructExtractConstraint(StructExtractConstraint ,
            |_, _, _, _, _|  {
                true
        }),
        StructInjectConstraint(StructInjectConstraint ,
            |_, _, _, _, _|  {
                true
        }),
        StructExplodeConstraint(StructExplodeConstraint  ,
            |_, _, _, _, _|  {
                true
        }),
    }
}
