use std::marker::PhantomData;

use super::entity::Entity;
use super::environ::Environ;
use crate::{EntityId, RegionId};

pub trait ConstraintTrait {
  type DataTypeT;
  type AttributeT;
  fn verify<'env, E, EntityT: Entity>(
    &self, env: &'env E, attrs: Vec<(String, Self::AttributeT)>,
    uses: Vec<(String, Vec<Option<EntityId>>)>,
    defs: Vec<(String, Vec<Option<EntityId>>)>, regions: Vec<(String, Vec<Option<RegionId>>)>,
  ) -> bool
  where
    E: Environ<EntityT = EntityT, AttributeT = Self::AttributeT>,
    EntityT: Entity<DataTypeT = Self::DataTypeT, AttributeT = Self::AttributeT>;
}

#[derive(PartialEq, Clone, Copy, Debug, Hash)]
pub struct SameTypeConstraint<D, A> {
  _marker: PhantomData<(D, A)>,
}

impl<D: PartialEq, A: Clone + PartialEq> ConstraintTrait for SameTypeConstraint<D, A> {
  type AttributeT = A;
  type DataTypeT = D;

  fn verify<'env, E, EntityT: Entity>(
    &self, env: &'env E, _attrs: Vec<(String, Self::AttributeT)>,
    uses: Vec<(String, Vec<Option<EntityId>>)>,
    defs: Vec<(String, Vec<Option<EntityId>>)>, _regions: Vec<(String, Vec<Option<RegionId>>)>,
  ) -> bool
  where
    E: Environ<EntityT = EntityT>,
    EntityT: Entity<DataTypeT = Self::DataTypeT>,
  {
    let uses_tys = uses.into_iter().map(|pair| pair.1).flat_map(|v| {
      v.iter()
        .filter(|x| x.is_some())
        .map(|x| {
          let Some(x) = x else { panic!() };
          env.get_entity(x.to_owned()).get_dtype()
        })
        .collect::<Vec<_>>()
    });
    let defs_tys = defs.into_iter().map(|pair| pair.1).flat_map(|v| {
      v.iter()
        .filter(|x| x.is_some())
        .map(|x| {
          let Some(x) = x else { panic!() };
          env.get_entity(x.to_owned()).get_dtype()
        })
        .collect::<Vec<_>>()
    });

    let mut ty_collect = (uses_tys).chain(defs_tys);
    if let Some(first) = ty_collect.next() {
      ty_collect.all(|item| item == first)
    } else {
      true
    }
  }
}

impl<D, A> SameTypeConstraint<D, A> {
  pub fn new() -> Self { Self { _marker: PhantomData } }
}

#[derive(PartialEq, Clone, Copy, Debug, Hash)]
pub struct SameTypeOperandConstraint<D, A> {
  _marker: PhantomData<(D, A)>,
}

impl<D: PartialEq, A> ConstraintTrait for SameTypeOperandConstraint<D, A> {
  type AttributeT = A;
  type DataTypeT = D;

  fn verify<'env, E, EntityT: Entity>(
    &self, env: &'env E, _attrs: Vec<(String, Self::AttributeT)>,
    uses: Vec<(String, Vec<Option<EntityId>>)>,
    _defs: Vec<(String, Vec<Option<EntityId>>)>, _regions: Vec<(String, Vec<Option<RegionId>>)>,
  ) -> bool
  where
    E: Environ<EntityT = EntityT, AttributeT = Self::AttributeT>,
    EntityT: Entity<DataTypeT = Self::DataTypeT, AttributeT = Self::AttributeT>,
  {
    let mut uses_tys = uses.into_iter().map(|pair| pair.1).flat_map(|v| {
      v.iter()
        .filter(|x| x.is_some())
        .map(|x| {
          let Some(x) = x else { panic!() };
          env.get_entity(x.to_owned()).get_dtype()
        })
        .collect::<Vec<_>>()
    });

    if let Some(first) = uses_tys.next() {
      uses_tys.all(|item| item == first)
    } else {
      true
    }
  }
}

impl<D, A> SameTypeOperandConstraint<D, A> {
  pub fn new() -> Self { Self { _marker: PhantomData } }
}

#[macro_export]
macro_rules! constraint_def {
    (
        [data_type = $dtype:ty, attr = $attr:ty]
        $name:ident = {
            $($variant:ident($variant_ty:ident$(,$($tt:tt)*)?)),*
            $(,)?
        }
    ) => {
        #[derive(Clone, Debug, PartialEq, Hash)]
        pub enum $name {
            $($variant($variant_ty)),*
        }

        impl irony::ConstraintTrait for $name {
            type DataTypeT = $dtype;
            type AttributeT = $attr;
            fn verify<'env, E, EntityT: irony::Entity>(
                &self,
                env: &'env E,
                attrs: Vec<(String, Self::AttributeT)>,
                uses: Vec<(String, Vec<Option<irony::EntityId>>)>,
                defs: Vec<(String, Vec<Option<irony::EntityId>>)>,
                regions: Vec<(String, Vec<Option<irony::RegionId>>)>,
            ) -> bool
            where
                E: irony::Environ<EntityT = EntityT, AttributeT = Self::AttributeT>,
                EntityT: irony::Entity<DataTypeT = Self::DataTypeT, AttributeT = Self::AttributeT> {
                    match self {
                        $($name::$variant(inner) => inner.verify(env, attrs, uses, defs, regions)),*
                    }
                }
        }

        $(
        impl Into<$name> for $variant_ty {
            fn into(self) -> $name {
                $name::$variant(self)
            }
        }
        )*

        $(
            $(
            irony::constraint_struct_impl!($variant_ty, $dtype, $attr, $($tt)*);
            )?
        )*
    };

}

#[macro_export]
macro_rules! constraint_struct_impl {
    ($variant_ty:ident, $dtype:ty, $attr:ty, $($tt:tt)*) => {
        #[derive(Default, Clone, Debug, PartialEq, Hash)]
        pub struct $variant_ty;
        impl irony::ConstraintTrait for $variant_ty {
            type DataTypeT = $dtype;

            type AttributeT = $attr;

            fn verify<'env, E, EntityT: irony::Entity>(
                &self,
                env: &'env E,
                attrs: Vec<(String, Self::AttributeT)>,
                uses: Vec<(String, Vec<Option<irony::EntityId>>)>,
                defs: Vec<(String, Vec<Option<irony::EntityId>>)>,
                regions: Vec<(String, Vec<Option<irony::RegionId>>)>,
            ) -> bool
            where
                E: irony::Environ<EntityT = EntityT, AttributeT = Self::AttributeT>,
                EntityT: irony::Entity<DataTypeT = Self::DataTypeT, AttributeT = Self::AttributeT> {
                    let f = $($tt)*;
                    f(env, attrs, uses, defs, regions)
                }
        }
    };
}
