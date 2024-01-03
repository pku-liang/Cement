use cmt::preclude::*;

mod array {
  use super::*;

  #[interface(Copy)]
  pub struct ArrToArray<const M: usize, const N: usize, const NN: usize> {
    ia: Arr<M, B<N>>,
    ib: Flip<Array<B<NN>>>,
  }

  impl<const M: usize, const N: usize, const NN: usize> ArrToArray<M, N, NN> {
    pub fn new() -> Self {
      Self {
        ia: Arr::default(),
        ib: Flip(Array(M * N / NN, B::default())),
      }
    }
  }

  module! {
      ArrToArray<4, 8, 4>(c) =>
      arr_to_arry(module) {
          let casted = wire!(module.ia.cast(Array(4*8/4, B4)));
          module.ib %= casted;
      }
  }

  #[test]
  fn test_arr_to_array() {
    let mut c = Cmtc::new(CmtcConfig::default());
    ArrToArray::new().arr_to_arry(&mut c);
    c.print();
  }

  #[interface(Default)]
  pub struct ArrConstantIfc {
    o: Flip<Arr<8, B<8>>>,
    o_2d: Flip<Arr<8, Arr<8, B<8>>>>,
  }

  module! {
      ArrConstantIfc(c) =>
      arr_constant_m(module) {
          module.o %= [0, 1, 2, 3, 4, 5, 6, 7].lit(B8x8);
          module.o_2d %= [[0, 1, 2, 3, 4, 5, 6, 7]; 8].lit(Arr(B8x8));
      }
  }

  #[test]
  fn test_arr_constant() {
    let mut c = Cmtc::new(CmtcConfig::default());
    ArrConstantIfc::default().arr_constant_m(&mut c);
    c.print();
  }

  #[interface(Default)]
  pub struct ArrCreate {
    i0: B<8>,
    i1: B<8>,
    o: Flip<Arr<2, B<8>>>,
  }

  module! {
      ArrCreate(c) =>
      arr_create_m(module) {
          module.o %= [module.i0, module.i1].ac();
      }
  }

  #[test]
  fn test_arr_create() {
    let mut c = Cmtc::new(CmtcConfig::default());
    ArrCreate::default().arr_create_m(&mut c);
    c.print();
  }

  #[interface(Default)]
  pub struct ArrConcat {
    i0: Arr<4, B<8>>,
    i1: Arr<4, B<8>>,
    o: Flip<Arr<8, B<8>>>,
  }

  module! {
      ArrConcat(c) =>
      arr_concat_m(module) {
          module.o %= module.i0.concat(module.i1);
      }
  }

  #[test]
  fn test_arr_concat() {
    let mut c = Cmtc::new(CmtcConfig::default());
    ArrConcat::default().arr_concat_m(&mut c);
    c.print();
  }

  #[interface(Default)]
  pub struct ArrSlice {
    i: Arr<4, B<2>>,
    idx: B<2>,
    o: Flip<Arr<2, B<2>>>,
  }

  module! {
      ArrSlice(c) =>
      arr_slice_m(module) {
          module.o %= module.i.slice(module.idx, 2);
      }
  }

  #[test]
  fn test_arr_slice() {
    let mut c = Cmtc::new(CmtcConfig::default());
    ArrSlice::default().arr_slice_m(&mut c);
    c.print();
  }

  #[interface(Default)]
  pub struct ArrGet {
    i: Arr<4, B<2>>,
    idx: B<2>,
    o: Flip<B<2>>,
  }

  module! {
      ArrGet(c) =>
      arr_get_m(module) {
          module.o %= module.i.get(module.idx);
      }
  }

  #[test]
  fn test_arr_get() {
    let mut c = Cmtc::new(CmtcConfig::default());
    ArrGet::default().arr_get_m(&mut c);
    c.print();
  }
}

mod struct_ty {
  use super::*;

  #[derive(Default, Debug, Clone, Copy, Struct)]
  pub struct Pair<T: DataTypeTrait> {
    x: T,
    y: T,
  }

  #[interface(Default)]
  struct PairPass<T: DataTypeTrait> {
    i: Pair<T>,
    o: Flip<Pair<T>>,
  }

  module! {
      <T: DataTypeTrait> PairPass<T> (c) =>
      pair_pass_m(module) {
          module.o %= module.i;
      }
  }

  #[test]
  fn test_pair_pass() {
    let mut c = Cmtc::new(CmtcConfig::default());
    PairPass::<B<8>>::default().pair_pass_m(&mut c);
    c.print();
  }

  #[interface(Default, Copy)]
  pub struct CreatePair {
    x: B<8>,
    y: B<8>,
    o: Flip<Pair<B<8>>>,
  }

  module! {
      CreatePair(c) =>
      create_pair_m(module) {
          let pair = wire!(Pair::<B<8>>::default().struct_create(module.x, module.y));
          module.o %= pair;
      }
  }

  #[test]
  fn test_create_pair() {
    let mut c = Cmtc::new(CmtcConfig::default());
    CreatePair::default().create_pair_m(&mut c);
    c.print();
  }

  #[interface(Default, Copy)]
  pub struct ExtractPair {
    i: Pair<B<8>>,
    o: Flip<B<8>>,
  }

  module! {
      ExtractPair(c) =>
      extract_pair_m(module) {
          module.o %= module.i.x();
      }
  }

  #[test]
  fn test_extract_pair() {
    let mut c = Cmtc::new(CmtcConfig::default());
    ExtractPair::default().extract_pair_m(&mut c);
    c.print();
  }

  #[interface(Default, Copy)]
  pub struct InjectPair {
    i: Pair<B<8>>,
    x: B<8>,
    o: Flip<Pair<B<8>>>,
  }

  module! {
      InjectPair(c) =>
      inject_pair_m(module) {
          module.o %= module.i.with_x(module.x);
      }
  }

  #[test]
  fn test_inject_pair() {
    let mut c = Cmtc::new(CmtcConfig::default());
    InjectPair::default().inject_pair_m(&mut c);
    c.print();
  }

  #[interface(Default, Copy)]
  pub struct ExplodePair {
    i: Pair<B<8>>,
    o: flip!((B<8>, B<8>)),
  }

  module! {
      ExplodePair(c) =>
      explode_pair_m(module) {
          module.o %= module.i.explode();
      }
  }

  #[test]
  fn test_explode_pair() {
    let mut c = Cmtc::new(CmtcConfig::default());
    ExplodePair::default().explode_pair_m(&mut c);
    c.print();
  }
}

mod tuple {
  use super::*;

  type TupleT = (U<8>, U<8>);
  #[interface(Default, Copy)]
  pub struct Tuple2To1 {
    i0: TupleT,
    i1: TupleT,
    o: flip!(TupleT),
  }
  module! {
      Tuple2To1(c) =>
      tuple_add_m(module) {
          module.o %= (module.i0.0 + module.i1.0, module.i0.1 + module.i1.1);
      }
  }

  #[test]
  fn test_tuple_add() {
    let mut c = Cmtc::new(CmtcConfig::default());
    Tuple2To1::default().tuple_add_m(&mut c);
    c.print();
  }

  #[interface(Default, Copy)]
  pub struct TupleO {
    o: flip!(TupleT),
  }

  module! {
    TupleO =>
    tuple_const(io) {
      io.o %= (1u8, 2u8).lit((B8,B8));
    }
  }

  #[test]
  fn test_tuple_const() {
    let mut c = Cmtc::new(CmtcConfig::default());
    TupleO::default().tuple_const(&mut c);
    c.print();
  }
}

mod signal {
  use super::*;

  #[signal(Default)]
  pub struct Valid<T: SignalTrait + Default>
  where <T as Interface>::FlipT: Default
  {
    pub valid: B<1>,
    pub data: T,
  }

  #[signal(Default)]
  pub struct Double<T: SignalTrait + Default>
  where <T as Interface>::FlipT: Default
  {
    pub a: T,
    pub b: T,
  }

  #[interface(Default)]
  pub struct DoubleO<T: SignalTrait + Default>
  where <T as Interface>::FlipT: Default
  {
    o: <Double<T> as Interface>::FlipT,
  }

  module! {
    DoubleO<Valid<B<8>>> =>
    double_valid_const_m(io) {
      let a_value = ValidLit {
        valid: true,
        data: 1u8,
        _marker0: PhantomData,
      };
      let b_value = ValidLit {
        valid: true,
        data: 2u8,
        _marker0: PhantomData,
      };
      let ifc = io.o.ifc().flip();
      io.o %= DoubleLit {
        a: a_value,
        b: b_value,
        _marker0: PhantomData
      }.lit(ifc);
    }
  }
  #[test]
  fn test_double_valid_const() {
    let mut c = Cmtc::new(CmtcConfig::default());
    DoubleO::<Valid<B<8>>>::default().double_valid_const_m(&mut c);
    // c.print();
    c.generate_workspace()
  }
}
