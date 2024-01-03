pub use tuple::*;
pub use vec::*;

mod tuple {
  use crate::preclude::*;

  macro_rules! impl_tuple {
        ($($ty:ident : $id:tt),*) => {
            paste! {

              impl<$($ty: SignalTrait,)* $([<S $ty>]: IntoValue<$ty>,)*> IntoValue<($($ty,)*)> for ($([<S $ty>],)*) {
                
                fn into_value(self, signal: ($($ty,)*)) -> SignalValue {
                  let ($([<t $id>],)*) = self;
                  let ($([<s $id>],)*) = signal;
                  let mut v_data = vec![];
                  $(
                      v_data.extend([<t $id>].into_value([<s $id>]).v_data);
                  )*
                  SignalValue {
                    v_data,
                    name: format!("tuple"),
                  }
                }
              }

              impl<$($ty: SignalTrait,)*> SignalTrait for ($($ty,)*) {
                fn total_width(&self) -> usize {
                    let ($([<t $id>],)*) = self;
                    let mut width = 0;
                    $(
                        width += [<t $id>].total_width();
                    )*
                    width
                }
                fn v_ir_type(&self) -> Vec<irony_cmt::DataTypeEnum> {
                    let ($([<t $id>],)*) = self;
                    let mut v = vec![];
                    $(
                        v.extend([<t $id>].v_ir_type());
                    )*
                    v
                }
              }
                impl<$($ty: Interface,)*> Interface for ($($ty,)*) {
                    type FlipT = ($($ty::FlipT,)*);

                    type ImplT = ($($ty::ImplT,)*);

                    fn name() -> String {
                        let mut name = format!("tuple");
                        $(
                            name.push_str("_");
                            name.push_str(&$ty::name());
                        )*
                        name
                    }

                    fn flip(self) -> Self::FlipT {
                        ($(self.$id.flip(),)*)
                    }

                    fn traverse(&self) -> IfcFields {
                        IfcFields::Branch(vec![
                            $(
                                (format!("{}", $id), self.$id.traverse()),
                            )*
                        ])
                    }

                    fn impl_with(self, fields: IfcImplFields) -> Self::ImplT {
                        match fields {
                            IfcImplFields::Branch(v) => {
                                let mut v = v.into_iter();
                                $(
                                    let (_name, [<fields $id>]) = v.next().unwrap();
                                )*
                                assert!(v.next().is_none());
                                ($(self.$id.impl_with([<fields $id>]),)*)
                            },
                            _ => {
                                panic!("impl_with only be applied to Branch")
                            },
                        }
                    }
                }

                impl<$($ty: InterfaceImpl,)*> InterfaceImpl for ($($ty,)*) {
                    type FlipT = ($($ty::FlipT,)*);
                    type IfcT = ($($ty::IfcT,)*);

                    fn flip(self) -> Self::FlipT { ($(self.$id.flip(),)*) }

                    fn traverse(&self) -> IfcImplFields {
                        IfcImplFields::Branch(vec![
                            $(
                                (format!("{}", $id), self.$id.traverse()),
                            )*
                        ])
                    }

                    fn replace_with_fields(self, fields: IfcImplFields) -> Self {
                        match fields {
                            IfcImplFields::Branch(v) => {
                                let mut v = v.into_iter();
                                $(
                                    let (_name, [<fields $id>]) = v.next().unwrap();
                                )*
                                assert!(v.next().is_none());
                                (
                                    $(
                                        self.$id.replace_with_fields([<fields $id>]),
                                    )*
                                )
                            },
                            _ => {
                                panic!("replace_with_fields only be applied to Branch")
                            },
                        }
                    }

                    fn ifc(&self) -> Self::IfcT { ($(self.$id.ifc(),)*) }
                }

                impl<$($ty: InterfaceImpl + Connect<<$ty as InterfaceImpl>::FlipT>,)*> Connect<<($($ty,)*) as InterfaceImpl>::FlipT> for ($($ty,)*) {
                    fn connect(self, rhs: <($($ty,)*) as InterfaceImpl>::FlipT, c: &mut Cmtc) {
                        let ($([<t $id>],)*) = self;
                        let ($([<r $id>],)*) = rhs;
                        $(
                            [<t $id>].connect([<r $id>], c);
                        )*
                    }
                }
                impl<$($ty: InterfaceImpl + Connect<<$ty as InterfaceImpl>::FlipT>,)*> ConnectExpr<<<($($ty,)*) as InterfaceImpl>::FlipT as InterfaceImpl>::IfcT> for ($($ty,)*) {}
            }

        };
    }

  impl_tuple!(T0:0);
  impl_tuple!(T0:0, T1:1);
  impl_tuple!(T0:0, T1:1, T2:2);
  impl_tuple!(T0:0, T1:1, T2:2, T3:3);
  impl_tuple!(T0:0, T1:1, T2:2, T3:3, T4:4);
  impl_tuple!(T0:0, T1:1, T2:2, T3:3, T4:4, T5:5);
  impl_tuple!(T0:0, T1:1, T2:2, T3:3, T4:4, T5:5, T6:6);
  impl_tuple!(T0:0, T1:1, T2:2, T3:3, T4:4, T5:5, T6:6, T7:7);

  macro_rules! impl_tuple_to_expr {
        ($($ty:ident : $id:tt),*) => {
            impl<$($ty: Interface,)* > ToExpr<($($ty,)*)> for ($(Expr<$ty>,)*) {
                fn expr(&self) -> Expr<($($ty,)*)> {

                    Expr {
                        ifc: ($(self.$id.ifc.to_owned(),)*),
                        ast: ExprAst::Branch(
                            ExprNode::Tuple,
                            vec![$(self.$id.ast.to_owned(),)*],
                            None,
                            ($(self.$id.ifc.to_owned(),)*).traverse(),
                        ),
                    }
                }
            }
        };
    }
  impl_tuple_to_expr!(T0:0);
  impl_tuple_to_expr!(T0:0, T1:1);
  impl_tuple_to_expr!(T0:0, T1:1, T2:2);
  impl_tuple_to_expr!(T0:0, T1:1, T2:2, T3:3);
  impl_tuple_to_expr!(T0:0, T1:1, T2:2, T3:3, T4:4);
  impl_tuple_to_expr!(T0:0, T1:1, T2:2, T3:3, T4:4, T5:5);
  impl_tuple_to_expr!(T0:0, T1:1, T2:2, T3:3, T4:4, T5:5, T6:6);
  impl_tuple_to_expr!(T0:0, T1:1, T2:2, T3:3, T4:4, T5:5, T6:6, T7:7);
}

mod vec {
  use crate::preclude::*;

  impl<T: SignalTrait, S: IntoValue<T>> IntoValue<Vec<T>> for Vec<S> {
    fn into_value(self, signal: Vec<T>) -> SignalValue {
      let mut v_data = vec![];
      self.into_iter().zip(signal.into_iter()).for_each(|(x, y)| {
        v_data.extend(x.into_value(y).v_data);
      });
      SignalValue {
        v_data,
        name: format!("vec"),
      }
    }
  }

  impl<T: SignalTrait> SignalTrait for Vec<T> {
    fn total_width(&self) -> usize {
        self.iter().map(|x| x.total_width()).sum()
    }
    fn v_ir_type(&self) -> Vec<irony_cmt::DataTypeEnum> {
        self.iter().map(|x| x.v_ir_type()).flatten().collect()
    }
  }

  impl<T: Interface> Interface for Vec<T> {
    type FlipT = Vec<T::FlipT>;
    type ImplT = Vec<T::ImplT>;

    fn name() -> String { format!("vec_{}", T::name()) }

    fn flip(self) -> Self::FlipT { self.into_iter().map(|x| x.flip()).collect() }

    fn traverse(&self) -> IfcFields {
      IfcFields::Branch(
        self.iter().enumerate().map(|(i, v)| (format!("{}", i), v.traverse())).collect(),
      )
    }

    fn impl_with(self, fields: IfcImplFields) -> Self::ImplT {
      match fields {
        IfcImplFields::Branch(v) => {
          let mut v = v.into_iter();
          self
            .into_iter()
            .map(|x| {
              let (_name, fields) = v.next().unwrap();
              x.impl_with(fields)
            })
            .collect()
        },
        _ => {
          panic!("impl_with only be applied to Branch")
        },
      }
    }
  }

  impl<T: InterfaceImpl> InterfaceImpl for Vec<T> {
    type FlipT = Vec<T::FlipT>;
    type IfcT = Vec<T::IfcT>;

    fn flip(self) -> Self::FlipT { self.into_iter().map(|x| x.flip()).collect() }

    fn traverse(&self) -> IfcImplFields {
      IfcImplFields::Branch(
        self.iter().enumerate().map(|(i, v)| (format!("{}", i), v.traverse())).collect(),
      )
    }

    fn replace_with_fields(self, fields: IfcImplFields) -> Self {
      match fields {
        IfcImplFields::Branch(v) => {
          let mut v = v.into_iter();
          self
            .into_iter()
            .map(|x| {
              let (_name, fields) = v.next().unwrap();
              x.replace_with_fields(fields)
            })
            .collect()
        },
        _ => {
          panic!("replace_with_fields only be applied to Branch")
        },
      }
    }

    fn ifc(&self) -> Self::IfcT { self.iter().map(|x| x.ifc()).collect() }
  }

  impl<T: InterfaceImpl> Connect<<Vec<T> as InterfaceImpl>::FlipT> for Vec<T> {
    fn connect(self, target: <Vec<T> as InterfaceImpl>::FlipT, c: &mut Cmtc) {
      assert_eq!(self.len(), target.len());
      self.into_iter().zip(target.into_iter()).for_each(|(x, y)| {
        x.connect(y, c);
      });
    }
  }
  impl<T: InterfaceImpl>
    ConnectExpr<<<Vec<T> as InterfaceImpl>::FlipT as InterfaceImpl>::IfcT> for Vec<T>
  {
  }
}

mod array {

  use crate::preclude::*;
  impl<T: SignalTrait, S: IntoValue<T>, const N:usize> IntoValue<[T; N]> for [S; N] {
    fn into_value(self, signal: [T; N]) -> SignalValue {
      let mut v_data = vec![];
      self.into_iter().zip(signal.into_iter()).for_each(|(x, y)| {
        v_data.extend(x.into_value(y).v_data);
      });
      SignalValue {
        v_data,
        name: format!("vec"),
      }
    }
  }

  impl<T: SignalTrait, const N:usize> SignalTrait for [T; N] {
    fn total_width(&self) -> usize {
      self.iter().map(|x| x.total_width()).sum()
    }
    fn v_ir_type(&self) -> Vec<irony_cmt::DataTypeEnum> {
        self.iter().map(|x| x.v_ir_type()).flatten().collect()
    }
  }


  impl<T: Interface, const N: usize> Interface for [T; N] {
    type FlipT = [T::FlipT; N];
    type ImplT = [T::ImplT; N];

    fn name() -> String { format!("vec_{}", T::name()) }

    fn flip(self) -> Self::FlipT { self.map(|x| x.flip()) }

    fn traverse(&self) -> IfcFields {
      IfcFields::Branch(
        self.iter().enumerate().map(|(i, v)| (format!("{}", i), v.traverse())).collect(),
      )
    }

    fn impl_with(self, fields: IfcImplFields) -> Self::ImplT {
      match fields {
        IfcImplFields::Branch(v) => {
          let mut v = v.into_iter();
          self.map(|x| {
            let (_name, fields) = v.next().unwrap();
            x.impl_with(fields)
          })
        },
        _ => {
          panic!("impl_with only be applied to Branch")
        },
      }
    }
  }

  impl<T: InterfaceImpl, const N: usize> InterfaceImpl for [T; N] {
    type FlipT = [T::FlipT; N];
    type IfcT = [T::IfcT; N];

    fn flip(self) -> Self::FlipT { self.map(|x| x.flip()) }

    fn traverse(&self) -> IfcImplFields {
      IfcImplFields::Branch(
        self.iter().enumerate().map(|(i, v)| (format!("{}", i), v.traverse())).collect(),
      )
    }

    fn replace_with_fields(self, fields: IfcImplFields) -> Self {
      match fields {
        IfcImplFields::Branch(v) => {
          let mut v = v.into_iter();
          self.map(|x| {
            let (_name, fields) = v.next().unwrap();
            x.replace_with_fields(fields)
          })
        },
        _ => {
          panic!("replace_with_fields only be applied to Branch")
        },
      }
    }

    fn ifc(&self) -> Self::IfcT {
      self.iter().map(|x| x.ifc()).collect::<Vec<_>>().try_into().unwrap()
    }
  }

  impl<T: InterfaceImpl, const N: usize> Connect<<[T; N] as InterfaceImpl>::FlipT>
    for [T; N]
  {
    fn connect(self, target: [T::FlipT; N], c: &mut Cmtc) {
      self.into_iter().zip(target.into_iter()).for_each(|(x, y)| {
        x.connect(y, c);
      });
    }
  }

  impl<T: InterfaceImpl, const N: usize>
    ConnectExpr<<<[T; N] as InterfaceImpl>::FlipT as InterfaceImpl>::IfcT> for [T; N]
  {
  }

}
mod hash_map {
  // TODO: fill this
}
