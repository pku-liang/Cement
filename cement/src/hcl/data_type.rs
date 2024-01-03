use std::fmt::Debug;

use irony_cmt::{ArrayType, ClkType};
pub trait DataTypeTrait: Copy + Debug + 'static + SignalTrait + Interface<FlipT = Flip<Self>, ImplT = I<Self>> {
  fn width(&self) -> usize;
  fn ir_type(&self) -> irony_cmt::DataTypeEnum;
}

pub trait SignalTrait: Clone + Debug + 'static + Interface {
  fn total_width(&self) -> usize;
  fn v_ir_type(&self) -> Vec<irony_cmt::DataTypeEnum>;
}


impl<DataType: DataTypeTrait> SignalTrait for DataType {
  fn total_width(&self) -> usize { self.width() }

  fn v_ir_type(&self) -> Vec<irony_cmt::DataTypeEnum> { vec![self.ir_type()] }
}

#[derive(Clone, Debug, Copy, Default)]
pub struct Clk;
impl DataTypeTrait for Clk {
  fn width(&self) -> usize { 1 }

  fn ir_type(&self) -> irony_cmt::DataTypeEnum { irony_cmt::DataTypeEnum::Clk(ClkType) }
}

#[derive(Clone, Debug, Copy, Default)]
pub struct B<const N: usize>;

impl<const N: usize> DataTypeTrait for B<N> {
  fn width(&self) -> usize { N }

  fn ir_type(&self) -> irony_cmt::DataTypeEnum { irony_cmt::DataTypeEnum::UInt(N.into()) }
}

#[derive(Clone, Debug, Copy)]
pub struct Bits(pub usize);

impl DataTypeTrait for Bits {
  fn width(&self) -> usize { self.0 }

  fn ir_type(&self) -> irony_cmt::DataTypeEnum {
    irony_cmt::DataTypeEnum::UInt(self.0.into())
  }
}

pub type U<const N: usize> = B<N>;
pub type UInt = Bits;

#[derive(Clone, Debug, Copy, Default)]
pub struct Arr<const N: usize, T: DataTypeTrait>(pub T);

impl<const N: usize, T: DataTypeTrait> DataTypeTrait for Arr<N, T> {
  fn width(&self) -> usize { N * self.0.width() }

  fn ir_type(&self) -> irony_cmt::DataTypeEnum {
    irony_cmt::DataTypeEnum::Array(ArrayType(Box::new(self.0.ir_type()), N))
  }
}

#[derive(Clone, Debug, Copy)]
pub struct Array<T: DataTypeTrait>(pub usize, pub T);

impl<T: DataTypeTrait> DataTypeTrait for Array<T> {
  fn width(&self) -> usize { self.0 * self.1.width() }

  fn ir_type(&self) -> irony_cmt::DataTypeEnum {
    irony_cmt::DataTypeEnum::Array(ArrayType(Box::new(self.1.ir_type()), self.0))
  }
}

use cmt_macros::{def_const_1d, def_const_2d};

use super::{Interface, I, Flip};

def_const_1d!(32);
def_const_2d!(16, 16);
