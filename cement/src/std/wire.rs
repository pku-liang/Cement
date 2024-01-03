use crate::preclude::*;

#[interface]
pub struct Wire<T: Interface + 'static> {
  pub i: T,
  pub o: <T as Interface>::FlipT,
}

impl<T: Interface> Wire<T> {
  pub fn new(i: T) -> Self { Self { i: i.to_owned(), o: i.flip() } }
}

module! {
    <T: Interface> Wire<T> =>
    wire(module) {
        module.o %= module.i;
    }
}

// pub struct WireFlipImpl<T: Interface + 'static> {
//   pub i: <<T as Interface>::FlipT as Interface>::ImplT,
//   pub o: <T as Interface>::ImplT,
//   pub ifc: T,
// }