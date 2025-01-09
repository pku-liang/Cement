use cmtc::*;
use cmtrs::*;

// itfc_declare!(
//   struct Top{
//     #[name("in")]
//     in_: input Type::Int(4),
//     out: output Type::Int(4),
//   }
//   method input (in_);
//   method output ()->(out);
// );

// #[module]
// fn make_top() -> Top {
//   let io = io! {};

//   let fifo = instance!(stl::fifo1_d(Type::Int(4)));

//   let deqed = instance!(stl::Wire::new(Type::Int(1)));

//   let output = method!(
//     () -> (io.out) {
//       deqed.write(literal(1, Type::Int(1)));
//       fifo.deq()
//     }
//   );
//   let deqed_default = rule! {
//     () { deqed.write(literal(0, Type::Int(1))); }
//   };

//   let input = method!(
//     !fifo.full() | deqed.read();
//     (io.in_) {
//       fifo.enq(io.in_);
//     }
//   );

//   schedule!(output, deqed_default, input);
// }

fn main() -> anyhow::Result<()> {
  utils::setup_logger();
  let mut fifo = stl::fifo(4, stl::FifoTy::Push, &Type::UInt(4));
  fifo.set_name("FIFO".to_string());

  // let circuit = top.to_cmtir();
  // println!("{}", circuit.ir_dump());

  elaborate(fifo, sv_config("tb/fifo/fifo.sv"))?;

  // let mut fifo = stl::fifo2_i(Type::Int(4));
  let mut fifo = stl::fifo(4, stl::FifoTy::I, &Type::UInt(4));
  fifo.set_name("FIFO".to_string());

  elaborate(fifo, sv_config("tb/fifo2/fifo.sv"))?;
  Ok(())
}
