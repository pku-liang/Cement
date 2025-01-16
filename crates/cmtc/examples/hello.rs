// use cmtc::ehdl::*;
// use cmtrs::*;
// use cmtrs::stl::*;

// itfc_declare! {
//   param T;
//   struct A2B {
//     a: input param T,
//     b: output param T,
//   }
//   method run(a) -> (b);
// }

// #[module]
// fn add1() -> A2B {
//   let t = Type::UInt(4);
//   let io = io! { T: &t};
//   let run = method!(
//     (io.a) -> (io.b) {
//       ret!(io.a + 1.lit(&t))
//     }
//   );
// }


// itfc_declare! {
//   struct Tb {}
// }

// #[module]
// fn mk_tb() -> Tb {
//   io!{};
//   anno!("is_tb": "true");
//   let add1 = instance!(add1());
//   let mut cycle = instance!(Integer::new());
//   named_always! {format!("input");
//     [cycle.lt(int(10))]
//     (){
//       let i = int_as(&cycle, &Type::UInt(4));
//       let b = add1.run(i);
//       sim_print!("cycle ", cycle, ": output=", b);
//       cycle %= &cycle + int(1);
//     }
//   };

//   named_always! {format!("exit");
//     [cycle.eq(int(10))]
//     (){
//       sim_print!("exit");
//       sim_exit!();
//     }
//   };
// }

// fn main() -> anyhow::Result<()> {
//   elaborate(add1(), sv_config("add1.sv"))?;
//   Ok(())
// }

fn main() {
  println!("Hello, world!");
}