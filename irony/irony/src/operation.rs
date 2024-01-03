use std::fmt::Debug;

pub use paste::paste;

use super::common::Id;
use super::entity::EntityId;
use crate::printer::OpPrinterTrait;
use crate::{ConstraintTrait, Environ, ReducerTrait, RegionId};

pub trait Op: Id + Debug {
  type DataTypeT;
  type AttributeT;
  type ConstraintT: ConstraintTrait<
    DataTypeT = Self::DataTypeT,
    AttributeT = Self::AttributeT,
  >;
  type PrinterT: OpPrinterTrait<
    DataTypeT = Self::DataTypeT,
    AttributeT = Self::AttributeT,
  >;

  fn get_defs(&self) -> Vec<(String, Vec<Option<EntityId>>)>;
  fn get_uses(&self) -> Vec<(String, Vec<Option<EntityId>>)>;

  fn get_attrs(&self) -> Vec<(String, Self::AttributeT)>;
  fn set_attrs(&mut self, attrs: Vec<(String, Self::AttributeT)>) -> ();
  fn get_constraints(&self) -> Vec<Self::ConstraintT>;

  fn uses(&self, entity: EntityId) -> bool;
  fn defs(&self, entity: EntityId) -> bool;

  fn get_parent(&self) -> Option<RegionId>;
  fn set_parent(&mut self, parent: Option<RegionId>);

  fn get_regions(&self) -> Vec<(String, Vec<Option<RegionId>>)>;

  fn use_region(&self, region: RegionId) -> bool;

  fn get_op_name(&self) -> String;

  fn get_printer(&self) -> Self::PrinterT;

  fn hash_with_reducer(&self, env: &impl Environ, reducer: &mut impl ReducerTrait);

  fn reduce_def_use(self, reducer: &mut impl ReducerTrait) -> Self;

  fn replace_use(&mut self, old: EntityId, new: EntityId) -> ();
}

#[derive(Clone, Copy, PartialEq, Debug, Hash, Eq, Default)]
pub struct OpId(pub usize);
impl From<usize> for OpId {
  fn from(value: usize) -> Self { Self(value) }
}
impl Id for OpId {
  fn id(&self) -> usize { self.0 }

  fn set_id(&mut self, id: usize) { self.0 = id }
}

#[macro_export]
macro_rules! reduce_then_hash {
  ($reducer:ident, $target:expr, $hasher:expr) => {{
    let __target = $target;
    let __reduced = __target.map(|x| $reducer.reduce_entity(x));
    __reduced.hash($hasher);
    // println!("\treduced {:?} to {:?}, then hash it", __target, __reduced);
  }};
}
#[macro_export]
macro_rules! op_def {
    (
        [data_type = $data_ty:ty, attr = $attr_ty:ty, constraint = $constraint_ty:ty]
        $name_enum:ident  = {
            $(
                $name:ident : {
                    defs: [$($def:ident),*$(;$($variadic_def:ident),*)?],
                    uses: [$($use:ident),*$(;$($variadic_use:ident),*)?],
                    $(attrs: [$($attr:ident:$attr_variant:ident($attr_inner_ty:ty)$(($attr_hash:tt))?),*],)?
                    $(regions: [$($region:ident),*$(;$($variadic_region:ident),+)?],)?
                    $(constraints: [$($constraint:expr),*],)?
                    print: ($($print_tt:tt)*)$(,)?
                }
            ),*
            $(,)?
        }
    ) => {

        $(
            irony::op_def_one! {
                [data_type = $data_ty, attr = $attr_ty, constraint = $constraint_ty]
                $name: {
                    defs : [$($def),*$(;$($variadic_def),+)?],
                    uses : [$($use),*$(;$($variadic_use),+)?],
                    $(attrs : [$($attr : $attr_variant($attr_inner_ty)$(($attr_hash))?),*],)?
                    $(regions: [$($region),*$(;$($variadic_region),+)?],)?
                    $(constraints : [$($constraint),*],)?
                    print: ($($print_tt)*)
                }
            }
        )*

        irony::op_enum! {
            [data_type = $data_ty, attr = $attr_ty, constraint = $constraint_ty]
            $name_enum = $($name),*
        }

        irony::op_printer! {
            [data_type = $data_ty, attr = $attr_ty]
            $name_enum = $($name),*
        }


    };
}
#[macro_export]
macro_rules! op_def_one {
    (
        [data_type = $data_ty:ty, attr = $attr_ty:ty, constraint = $constraint_ty:ty]
        $name:ident : {
            defs: [$($def:ident),*$(;$($variadic_def:ident),+)?],
            uses: [$($use:ident),*$(;$($variadic_use:ident),+)?],
            $(attrs: [$($attr:ident:$attr_variant:ident($attr_inner_ty:ty)$(($attr_hash:tt))?),*],)?
            $(regions: [$($region:ident),*$(;$($variadic_region:ident),+)?],)?
            $(constraints: [$($constraint:expr),*],)?
            print: ($($print_tt:tt)*)$(,)?
        }
    ) => {
        #[StructFields(pub)]
        #[derive(PartialEq, Debug, Clone)]
        pub struct $name  {
            id: usize,
            op_name: String,
            $($def: Option<irony::EntityId>,)*
            $($($variadic_def: Vec<Option<irony::EntityId>>,)*)?
            $($use:Option<irony::EntityId>,)*
            $($($variadic_use: Vec<Option<irony::EntityId>>,)*)?
            $($($attr: Option<$attr_inner_ty>,)*)?
            $(
                $($region: Option<irony::RegionId>,)*
                $(
                    $($variadic_region: Vec<Option<irony::RegionId>>,)*
                )?
            )?

            constraints: Vec<$constraint_ty>,
            parent: Option<irony::RegionId>,
            printer: paste!([< $name Printer >]),
        }

        impl irony::Id for $name {
            fn id(&self) -> usize {
                self.id
            }

            fn set_id(&mut self, id: usize) {
                self.id = id;
            }
        }

        impl irony::Op for $name {
            type DataTypeT = $data_ty;
            type ConstraintT = $constraint_ty;
            type AttributeT = $attr_ty;
            type PrinterT = paste!([< $name Printer >]);

            fn get_defs(&self) -> Vec<(String, Vec<Option<irony::EntityId>>)> {
                vec![
                    $((format!("{}", stringify!($def)), vec![self.$def.to_owned()])),*
                    $($((format!("{}", stringify!($variadic_def)), self.$variadic_def.to_owned()))*)?
                ]
            }

            fn get_uses(&self) -> Vec<(String, Vec<Option<irony::EntityId>>)> {
                vec![
                    $((format!("{}", stringify!($use)), vec![self.$use.to_owned()]),)*
                    $($((format!("{}", stringify!($variadic_use)), self.$variadic_use.to_owned())),*)?
                ]

            }

            fn get_attrs(&self) -> Vec<(String, Self::AttributeT)> {
                vec![
                    $(
                        $(
                            if self.$attr.is_some() {
                                (format!("{}", stringify!($attr)), self.$attr.to_owned().unwrap().into())
                            } else {
                                (format!("none"), Self::AttributeT::None)
                            }
                        ),*
                    )?
                ]
            }

            fn set_attrs(&mut self, attrs: Vec<(String, Self::AttributeT)>) ->() {
                $(
                    $(
                        self.$attr = attrs.iter().find(|(k, _)| k == &format!("{}", stringify!($attr))).map(|(_, v)| v.clone().into());
                    )*
                )?
            }

            fn get_constraints(&self) -> Vec<Self::ConstraintT> {
                self.constraints.clone()
            }

            fn uses(&self, entity: irony::EntityId) -> bool {
                self.get_uses().iter().flat_map(|(_, v)| v.iter()).any(|&x| {
                    if let Some(x) = x {
                        x.id() == entity.id()
                    } else {
                        false
                    }}
                )
            }

            fn defs(&self, entity: irony::EntityId) -> bool {
                self.get_defs().iter().flat_map(|(_, v)| v.iter()).any(|&x| {
                    if let Some(x) = x {
                        x.id() == entity.id()
                    } else {
                        false
                    }}
                )
            }


            fn get_parent(&self) -> Option<irony::RegionId> {
                self.parent
            }
            fn set_parent(&mut self, parent: Option<irony::RegionId>) {
                self.parent = parent;
            }

            fn get_regions(&self) -> Vec<(String, Vec<Option<irony::RegionId>>)> {
                vec![
                    $(
                        $((format!("{}", stringify!($region)), vec![self.$region]),)*
                        $(
                            $((format!("{}", stringify!($variadic_region)), self.$variadic_region.to_owned()),)*
                        )?
                    )?
                ]
            }

            fn use_region(&self, region: irony::RegionId) -> bool{
                self.get_regions().iter().any(|(_, v)| v.contains(&Some(region.to_owned())))
            }

            fn get_op_name(&self) -> String {
                self.op_name.clone()
            }

            fn get_printer(&self) -> Self::PrinterT {
                self.printer.clone()
            }


            fn hash_with_reducer(&self, env: &impl Environ, reducer: &mut impl ReducerTrait) {

                // println!("hashing op: {}", self.op_name);

                {
                   self.op_name.hash(env.get_hasher().deref_mut());
                //    println!("\thash {}", self.op_name);
                }
                $(
                    if self.$def.is_some() {
                        reduce_then_hash!(reducer, self.$def.to_owned(), env.get_hasher().deref_mut());

                    }
                )*
                $(
                    $(
                        for def in self.$variadic_def.to_owned() {
                            reduce_then_hash!(reducer, def, env.get_hasher().deref_mut());
                        }
                    )*
                )?

                $(
                    if self.$use.is_some() {
                        reduce_then_hash!(reducer, self.$use.to_owned(), env.get_hasher().deref_mut());
                    }
                )*
                $(
                    $(
                        for used in self.$variadic_use.to_owned() {
                            reduce_then_hash!(reducer, used, env.get_hasher().deref_mut());
                        }
                    )*
                )?

                $(
                    $(
                        $(
                            ${ignore(attr_hash)}
                            self.$attr.hash(env.get_hasher().deref_mut());
                            // println!("\thash {:?}", self.$attr);
                        )?
                    )*
                )?

                $(
                    $(
                        // println!("hash region {:?}", self.$region.unwrap());
                        env.hash_region(self.$region.unwrap(), reducer);
                    )*
                    $(
                        $(
                            for region in self.$variadic_region.to_owned() {
                                // println!("hash region {:?}", region);
                                env.hash_region(region, reducer);
                            }
                        )*
                    )?
                )?

            }

            fn reduce_def_use(self, reducer: &mut impl ReducerTrait) -> Self {
                let backup = self.to_owned();
                Self {
                    $(
                        $def: reducer.reduce_option_entity(self.$def),
                    )*
                    $(
                        $(
                            $variadic_def: self.$variadic_def.into_iter().map(|x| x.map(|x| EntityId(reducer.reduce_entity(x)))).collect(),
                        )*
                    )?
                    $(
                        $use: reducer.reduce_option_entity(self.$use),
                    )*
                    $(
                        $(
                            $variadic_use: self.$variadic_use.into_iter().map(|x| x.map(|x| EntityId(reducer.reduce_entity(x)))).collect(),
                        )*
                    )?
                    .. backup
                }
            }
            fn replace_use(&mut self, old: EntityId, new: EntityId) -> () {
                $(
                    if self.$use.is_some() {
                        if self.$use.unwrap() == old {
                            self.$use = Some(new);
                        }
                    }
                )*
                $(
                    $(
                      let new_variadic_use = self.$variadic_use.to_owned().into_iter().map(|x| if x == Some(old) { Some(new) } else { x }).collect();
                      self.$variadic_use = new_variadic_use;
                    )*
                )?
            }
        }


        impl $name {
            pub fn new(
                $($def: Option<irony::EntityId>,)*
                $($($variadic_def: Vec<Option<irony::EntityId>>,)*)?
                $($use: Option<irony::EntityId>,)*
                $($($variadic_use: Vec<Option<irony::EntityId>>,)*)?
                $($($attr: Option<$attr_inner_ty>,)*)?
                $($($region: Option<irony::RegionId>,)*)?
                $($($($variadic_region: Vec<Option<irony::RegionId>>,)*)?)?
            ) -> Self {

                Self {
                    id: 0,
                    op_name: stringify!($name).to_owned(),
                    $($def,)*
                    $($($variadic_def,)*)?
                    $($use,)*
                    $($($variadic_use,)*)?
                    $($($attr,)*)?
                    $($($region,)*)?
                    $($($($variadic_region,)*)?)?

                    constraints: vec![
                        $($($constraint),*)?
                    ],
                    parent: None,
                    printer: paste!([< $name Printer >]),
                }

            }
        }

        paste! {
            #[derive(Clone, Debug, PartialEq, Hash)]
            pub struct [< $name Printer >];

            impl OpPrinterTrait for [< $name Printer >] {
                type DataTypeT = $data_ty;
                type AttributeT = $attr_ty;

                fn print<'env, E, EntityT: Entity>(
                    &self,
                    env: &'env E,
                    attrs: Vec<(String, Self::AttributeT)>,
                    uses: Vec<(String, Vec<Option<irony::EntityId>>)>,
                    defs: Vec<(String, Vec<Option<irony::EntityId>>)>,
                    regions: Vec<(String, Vec<Option<irony::RegionId>>)>,
                ) -> String
                where
                    E: Environ<EntityT = EntityT, AttributeT = Self::AttributeT>,
                    EntityT: Entity<DataTypeT = Self::DataTypeT, AttributeT = Self::AttributeT> {
                        let f = $($print_tt)*;
                        f(env, attrs, uses, defs, regions)
                    }
            }

        }

    };
}

#[macro_export]
macro_rules! op_enum {
    ([data_type = $data_ty:ty, attr = $attr:ty, constraint = $constraint:ty] $name:ident = $($variant:ident),*) => {
        #[derive(PartialEq, Debug, Clone, Default)]
        pub enum $name {
            #[default]
            None,
            $($variant($variant)),*
        }

        $(
            impl Into<$name> for $variant {
                fn into(self) -> $name {
                    $name::$variant(self)
                }
            }
        )*

        impl irony::Id for $name {
            fn id(&self) -> usize {
                match self {
                    $name::None => panic!(),
                    $($name::$variant(inner) => inner.id(),)*
                }
            }
            fn set_id(&mut self, id: usize) {
                match self {
                    $name::None => panic!(),
                    $($name::$variant(inner) => inner.set_id(id),)*
                }
            }
        }

        impl irony::Op for $name {
            type DataTypeT = $data_ty;
            type AttributeT = $attr;
            type ConstraintT = $constraint;
            type PrinterT = paste!([< $name Printer >]);

            fn get_defs(&self) -> Vec<(String, Vec<Option<irony::EntityId>>)> {
                match self {
                    $name::None => panic!(),
                    $($name::$variant(inner) => inner.get_defs()),*
                }
            }
            fn get_uses(&self) -> Vec<(String, Vec<Option<irony::EntityId>>)> {
                match self {
                    $name::None => panic!(),

                    $($name::$variant(inner) => inner.get_uses()),*
                }
            }

            fn get_attrs(&self) -> Vec<(String, Self::AttributeT)> {
                match self {
                    $name::None => panic!(),
                    $($name::$variant(inner) => inner.get_attrs()),*
                }
            }

            fn set_attrs(&mut self, attrs: Vec<(String, Self::AttributeT)>) -> () {
                match self {
                    $name::None => panic!(),
                    $($name::$variant(inner) => inner.set_attrs(attrs)),*
                }
            }

            fn get_constraints(&self) -> Vec<Self::ConstraintT> {
                match self {
                    $name::None => panic!(),
                    $($name::$variant(inner) => inner.get_constraints()),*
                }

            }

            fn uses(&self, entity: irony::EntityId) -> bool {
                match self {
                    $name::None => panic!(),
                    $($name::$variant(inner) => inner.uses(entity)),*
                }
            }
            fn defs(&self, entity: irony::EntityId) -> bool{
                match self {
                    $name::None => panic!(),
                    $($name::$variant(inner) => inner.defs(entity)),*
                }
            }

            fn get_parent(&self) -> Option<irony::RegionId>{
                match self {
                    $name::None => panic!(),
                    $($name::$variant(inner) => inner.get_parent()),*
                }
            }
            fn set_parent(&mut self, parent: Option<irony::RegionId>) {
                match self {
                    $name::None => panic!(),
                    $($name::$variant(inner) => inner.set_parent(parent)),*
                }
            }

            fn get_regions(&self) -> Vec<(String, Vec<Option<irony::RegionId>>)> {
                match self {
                    $name::None => panic!(),
                    $($name::$variant(inner) => inner.get_regions()),*
                }
            }



            fn use_region(&self, region: irony::RegionId) -> bool{
                match self {
                    $name::None => panic!(),
                    $($name::$variant(inner) => inner.use_region(region)),*
                }
            }

            fn get_op_name(&self) -> String {
                match self {
                    $name::None => panic!(),
                    $($name::$variant(inner) => inner.get_op_name()),*
                }
            }

            fn get_printer(&self) -> Self::PrinterT {
                match self {
                    $name::None => panic!(),
                    $($name::$variant(inner) => inner.get_printer().into()),*
                }
            }

            fn hash_with_reducer(&self, env: &impl Environ, reducer: &mut impl ReducerTrait) {
                match self {
                    $name::None => panic!(),
                    $($name::$variant(inner) => inner.hash_with_reducer(env, reducer)),*
                }
            }

            fn reduce_def_use(self, reducer: &mut impl ReducerTrait) -> Self {
                match self {
                    $name::None => panic!(),
                    $($name::$variant(inner) => $name::$variant(inner.reduce_def_use(reducer))),*
                }
            }

            fn replace_use(&mut self, old: EntityId, new: EntityId) -> () {
                match self {
                    $name::None => panic!(),
                    $($name::$variant(inner) => inner.replace_use(old, new)),*
                }
            }
        }
    };
}

#[macro_export]
macro_rules! op_printer {
    (
        [data_type = $data_ty:ty, attr = $attr:ty]
        $name:ident = $($variant:ident),*
    ) => {
        paste! {
            pub enum [<$name Printer>] {
                $($variant([<$variant Printer>])),*
            }

            impl irony::OpPrinterTrait for [<$name Printer>] {
                type DataTypeT = $data_ty;
                type AttributeT = $attr;

                fn print<'env, E, EntityT: Entity>(
                    &self,
                    env: &'env E,
                    attrs: Vec<(String, Self::AttributeT)>,
                    uses: Vec<(String, Vec<Option<irony::EntityId>>)>,
                    defs: Vec<(String, Vec<Option<irony::EntityId>>)>,
                    regions: Vec<(String, Vec<Option<irony::RegionId>>)>,
                ) -> String
                where
                    E: Environ<EntityT = EntityT, AttributeT = Self::AttributeT>,
                    EntityT: Entity<DataTypeT = Self::DataTypeT, AttributeT = Self::AttributeT> {
                        match self {
                            $([<$name Printer>]::$variant(inner) => inner.print(env, attrs, uses, defs, regions)),*
                        }
                    }
            }

            $(

                impl Into<[<$name Printer>]> for [<$variant Printer>] {
                    fn into(self) -> [<$name Printer>] {
                        [<$name Printer>]::$variant(self)
                    }
                }

            )*
        }
    };
}
