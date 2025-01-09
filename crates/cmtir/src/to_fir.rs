use super::*;

/// Convert `cmtir` type to `firrtl` type. It supports `UInt`, `SInt`, `Bundle`, and `Vector`.
pub fn cmtir_type_to_firrtl_type(typ: ir::Type) -> fir::Type {
  match typ {
    ir::Type::UInt(width) => fir::Type::UIntType(width as usize),
    ir::Type::SInt(width) => fir::Type::SIntType(width as usize),
    ir::Type::Bundle(fields) => {
      let fields = fields
        .iter()
        .map(|(name, ty, flip)| {
          (cmtir_type_to_firrtl_type(ty.clone()), name.clone(), *flip)
        })
        .collect::<Vec<_>>();
      fir::Type::BundleType { elements: fields }
    }
    ir::Type::Vector(ty, width) => fir::Type::VectorType {
      elem_ty: Box::new(cmtir_type_to_firrtl_type(*ty.clone())),
      size: width as usize,
    },
    _ => panic!("Unsupported type: {}", typ.to_string()),
  }
}
