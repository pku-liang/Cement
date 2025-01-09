//! `cmtrs` is an embedded domain-specific language in Rust for digital chip design.
//!
//! `cmtrs` provides:
//! + Rule-based RTL hardware description
//! + Port connections with methods
//! + Multi-cycle cycle-accurate high level control statements to simplify finite state machines (FSMs)
//! + Embedded hardware generators for complex parameterization
//!
//! # `cmtrs` Basics
//!
//! ## VERY IMPORTANT!!!
//!
//! `cmtrs` use nightly features for span support. You need add a file
//! `.cargo/config.toml` with the following content: ```toml
//! [build]
//! rustflags = "--cfg procmacro2_semver_exempt"
//! ```
//! as well as a `rust-toolchain.toml` file with the following content:
//! ```toml
//! [toolchain]
//! channel = "nightly-2024-12-25"
//! ```
//! 
//! ## Interface declaration
//!
//! An interface is how a module looks like from the outside, which describes its type parameters, IO ports and methods.
//!
//! Modules with the same interface will have the same Rust type in `cmtrs`, which means different implementation can be selected by during generation.
//! From this point of view, interfaces are similar to traits.
//!
//! Module methods will become real Rust methods after instantiation, which can be invoked to trigger certain behavior of the module.
//! IO ports will be automatically connected according to method invocations.
//!
//! ### Example
//!
//! ```
//! use cmtrs::*;
//! itfc_declare!(
//!   param T; // Type parameter
//!   // IO declaration
//!   struct Counter { // Name of the interface
//!     set_val: input param T, // input port
//!     count: output param T,  // output port
//!   }
//!   // methods declaration
//!   method read() -> (count);  // output to 'count' port
//!   method set(set_val) ;      // input from 'set_val' port
//! );
//! ```
//!
//! ## Module generation
//!
//! To implement a module of certain interface, one must write a generator function, which generates hardware through function execution.
//!
//! A generator function is a function with `#[module]` attribute and returns a module interface.
//! The `#[module]` attribute is a proc-macro that will record all hardware behaviors inside of the function to produce a module.
//!
//! ### Example
//!
//! ```
//! # use cmtrs::*;
//! # itfc_declare!(
//! #   param T; // Type parameter
//! #   // IO declaration
//! #   struct Counter { // Name of the interface
//! #     set_val: input param T, // input port
//! #     count: output param T,  // output port
//! #   }
//! #   // methods declaration
//! #   method read() -> (count);  // output to 'count' port
//! #   method set(set_val) ;      // input from 'set_val' port
//! # );
//!
//! #[module]
//! fn make_counter(lim: usize, ty: &Type) -> Counter {
//!   // Provide type parameters and create ios.
//!   let io = io! {
//!     T: ty
//!   };
//!
//!   // Instantiate a register sub-module
//!   // `mut` here is required for the %= operator, though it is not mutable in-fact
//!   // `stl::reg` is the generator function of reg
//!   let mut reg_i = instance!(stl::reg(ty));
//!
//!   // an always rule will be automatically fired if the conditions are met
//!   // increase reg_i if i < lim
//!   let inc = always! {
//!     [reg_i.lt(lim.lit(ty))] // guard expression
//!     () { // ios
//!       reg_i %= &reg_i + 1.lit(ty); // body
//!       // equals reg_i.write(reg_i.read() + 1.lit(ty))
//!     }
//!   };
//!
//!   // reset reg_i to 0 if i>=lim
//!   let rst = always! {
//!     [reg_i.ge(lim.lit(ty))]
//!     () {
//!       reg_i %= 0.lit(ty);
//!     }
//!   };
//!
//!   // a method will be fired if it is invoke and conditions are met
//!   // set reg_i to the input `set_val`
//!   let set = method!(
//!     (io.set_val) {
//!       reg_i %= io.set_val;
//!     }
//!   );
//!
//!   // read the value of reg_i to the output `count`
//!   let read = method! {
//!     () -> (io.count) {
//!       ret!(&reg_i)
//!     }
//!   };
//!
//!   // `set` method has confict with itself,
//!   // so only one `set` can be called in one cycle
//!   method_rel!(set C set);
//!    
//!   // combinational order of the rules and methods in one cycle
//!   // rules
//!   schedule!(read, set, inc, rst);
//!
//!   // Automatically returns the generated module
//!   // Do not try to return anything here
//! }
//! ```
//!
//! ## List of commonly used macros in module generation
//!
//! + [`itfc_declare!`] Declares an interface.
//! + [`#[module]`](`module`) Make a module generator.
//! + [`io!`] Provides type parameters and sub-interface instances to the module, returns a struct containing all ios and sub-interfaces.
//! + [`anno!`] Give annotations to the module.
//! + [`always!`] Declare a always rule.
//! + [`method!`] Declare a method rule.
//! + [`ext_method!`] Declare a external method.
//! + [`method_rel!`] Declare the relationship between rules and methods.
//! + [`schedule!`] Declare a timing order for the rules and methods within one cycle.
//! + [`var!`] Make a single assign multiple use variable (wire) that can be used within a rule/method.
//! + [`if_!`] Combinational condition.
//! + [`ret!`] Return a value to statements at higher-level.
//! + [`statement!`] Make an expression into a statement.
//! + [`#[gen_fn]`](`gen_fn`) Make a generate function, which records macros calls and can be transferred back to its caller.
//! + [`generate!`] Actually generate the things into the module returned by a generate function.
//! + [`move_!`] Use on `move ||` closures so that macros can be called inside of it.
//! + [`sim_exit!`] Declare the finish of simulation.
//! + [`sim_print!`] Print values during simulation.
//!
//! ## High-level control primitives
//!
//! High level control primitives describes hardware behaviors that takes multiple cycles to finish. Currently we provide a sub-set of C-like primitives.
//!
//! + [`step!`] Group operations into an atomic step controlled by FSMs.
//! + [`seq!`] Execute children statements in sequential cycles.
//! + [`par!`] Execute children statements in parallel, then join them.
//! + [`branch!`] Make a fork on the FSM, decide which statement to execute in the next cycle.
//! + [`for_!`] Execute statements in C-like for loop.
//!
//! High-level primitives can only be used in rules with `fsm` or `pipeline` keyword. Such rule will become stateful and will be executed through multiple cycles once fired.
//! `fsm` rules are similar to a process, which takes inputs, does computation in the next cycles, and returns the result.
//! `pipeline` rules are similar to coroutines, which can be repeatedly invoked for multiple inputs and outputs.
//! Each statement at top-level in a pipeline works concurrently, and will be fired once the previous stage is finish.
//!
//! ### Example
//!
//! The Collatz Conjucture is a sequence, where `a[i+1] = a[i]/2` if a[i] is even, otherwise `a[i+1]=a[i]*3+1`.
//! The following example wil calculate such sequence.
//!
//! ```
//! use cmtrs::*;
//!
//! itfc_declare! {
//!   struct Collatz {
//!     in_: input Type::UInt(8),
//!     out: output Type::UInt(8)
//!   }
//!   method out () -> (out);
//!   method start (in_);
//! }
//!
//! #[module]
//! fn make_collatz() -> Collatz {
//!   let io = io! {};
//!
//!   let t = Type::UInt(8);
//!   let r = instance!(stl::reg(&t));
//!
//!   let out = method!(
//!     () -> (io.out) { ret!(r.read()) }
//!   );
//!
//!   let start = method!(
//!     fsm; // the fsm keyword
//!     (io.in_) {
//!       for_!{(r.write(io.in_);true; ; r.read().gt(literal(1, &t))) {
//!         branch!{r.read() & literal(1, &t) { // odd
//!           branch!{r.read().ne(1.lit(&t)){ // not one
//!             seq!{
//!               step!{ r.write(r.read() * literal(3, &t)); };
//!               step!{ r.write(r.read() + literal(1, &t)); };
//!             }
//!           } }
//!         } else { // even
//!           step!{ r.write(r.read() >> 1); };
//!         }
//!         };
//!       } }
//!     }
//!   );
//! }
//! ```
//!
//! ## Nested Interface
//!
//! An interface can be part of the interface of the parent module. This is useful for modules like FIFO or memory.
//! Within the module generator, nested interfaces can be accessed from `io!{}`.
//! From the outside, nested interfaces are public members of the Rust type of the module instance.
//!
//! ### Example
//!
//! The sequence merge example merges two sequence from FIFOs, and put them into the output FIFO.
//!
//! ```
//! use cmtrs::*;
//!
//! itfc_declare! {
//!   param T;
//!   struct SeqMerge {
//!     // a is a nested itfc, of type FIFO,
//!     // with parameter T, equal to the parameter T of SeqMerge
//!     a: itfc stl::FIFO{T: param T},
//!     b: itfc stl::FIFO{T: param T},
//!     c: itfc stl::FIFO{T: param T},
//!   }
//! }
//!
//! #[module]
//! fn seq_merge(t: &Type) -> SeqMerge {
//!   let io = io! {
//!     T: t,
//!     // use io macro to provide the exact instances to the nested itfc
//!     a: stl::fifo_default(4, t),
//!     b: stl::fifo_default(4, t),
//!     c: stl::fifo_default(4, t)
//!   };
//!   anno!("synthesis":"true");
//!
//!   let reg_a = instance!(stl::Reg::new(t));
//!   let reg_b = instance!(stl::Reg::new(t));
//!
//!   let reg_a_valid = instance!(stl::Reg::new(&Type::UInt(1)));
//!   let reg_b_valid = instance!(stl::Reg::new(&Type::UInt(1)));
//!
//!   let in_a = always!(
//!     [!reg_a_valid.read()]
//!     () {
//!       // use nested itfc from the io
//!       reg_a.write(io.a.deq());
//!       reg_a_valid.write(true);
//!     }
//!   );
//!   let in_b = always!(
//!     [!reg_b_valid.read()]
//!     () {
//!       reg_b.write(io.b.deq());
//!       reg_b_valid.write(true);
//!     }
//!   );
//!
//!   let out = always! {
//!     [reg_a_valid.read() & reg_b_valid.read() & !io.c.full()]
//!     () {
//!        let a = var!(reg_a.read());
//!        let b = var!(reg_b.read());
//!        if_!{(&a).le(&b) {
//!          io.c.enq(a);
//!          reg_a_valid.write(false);
//!        } else {
//!          io.c.enq(b);
//!          reg_b_valid.write(false);
//!        } }
//!      }
//!   };
//!
//!   schedule!(out, in_a, in_b);
//! }
//!
//! itfc_declare!(
//!   param T;
//!   struct Top {
//!     in_a: input param T,
//!     in_b: input param T,
//!     out_c: output param T,
//!   };
//!   method input_a(in_a);
//!   method input_b(in_b);
//!   method output_c()->(out_c);
//! );
//!
//! #[module]
//! fn make_top(t: &Type) -> Top {
//!   let io = io! {T: t};
//!   anno!("synthesis":"true");
//!
//!   let merger = instance!(seq_merge(t));
//!
//!   let input_a = method! {
//!     [!merger.a.full()]
//!     (io.in_a) {
//!       // use nested itfc from the outside
//!       merger.a.enq(io.in_a);
//!     }
//!   };
//!
//!   let input_b = method! {
//!     [!merger.b.full()]
//!     (io.in_b) {
//!       merger.b.enq(io.in_b);
//!     }
//!   };
//!
//!   let output_c = method! {
//!     () -> (io.out_c) {
//!       merger.c.deq()
//!     }
//!   };
//!
//!   schedule!(input_a, input_b, output_c);
//! }
//! ```
//!
//! ## Macros for dynamic generation
//!
//! Module can be completely dynamically generated, with arbitary IOs and methods. The following macros are helpful for this purpose.
//!
//! + [`named_always!`]
//! + [`named_method!`]
//! + [`named_ext_method!`]
//! + [`named_var!`]
//! + [`input!`]
//! + [`output!`]
//! + [`set_name!`]
//! + [`method_rel_raw!`]
//! + [`schedule_raw!`]
//! + [`ret_raw!`]
//!
//! ## Type System
//!
//! Currently `cmtrs` use completely dynamic types. See [`Type`] for more information.
//!
//! ## Operators
//!
//! Currenly `cmtrs` support a limitted range of operators. See [`ops`] for more information.
//!
//! ## Standard library
//!
//! `cmtrs` provide some standard templates in hardware, including [`Wire`](stl::Wire), [`Reg`](stl::Reg), [`FIFO`](stl::FIFO) and memories.
//! See [`stl`] for more information.
//!
//! ## Elaboration and Simumlation
//!
//! Elaboration and Simulation depends on crate [`cmtc`]. `cmtrs` can be elaborated into FIRRTL or System Verilog.
//! We also support writing testbenches in `cmtrs`, which will be transformed into C testbench for Verilator or Khronos.
//!
//! ### Elaboration
//!
//! ```
//! use cmtrs::*;
//! use cmtc::*;
//! # itfc_declare!(struct SomeModule{});
//! # #[module] fn make_module() -> SomeModule {io!{}};
//! // To System Verilog
//! let module = make_module();
//! elaborate(module, sv_config("path/to/sv.sv")).unwrap();
//! // To FIRRTL
//! let module = make_module();
//! elaborate(module, fir_config("path/to/fir.fir")).unwrap();
//! ```
//!
//! ### Testbench
//!
//! A `cmtrs` testbench is a non-synthesizable top-module.
//! It can use anything in normal modules, and additonally can use simulation only functionalities like [`Interger`](stl::Integer) and [`sim_print`].
//!
//! ```ignore
//! use cmtrs::*;
//! use cmtc::*;
//!
//! # itfc_declare!(param T; struct Counter{out: output param T} method read() -> (out););
//! # #[module] fn make_counter(n: usize, ty: &Type) -> Counter { io!{T: ty}; }
//!
//! itfc_declare!(
//!   struct Tb {}
//! );
//!
//! #[module]
//! fn make_tb() -> Tb {
//!   io! {}
//!   anno!("is_tb": "true");
//!
//!   let counter = instance!(make_counter(10, &Type::UInt(4)));
//!   let mut cycle = instance!(stl::integer());
//!
//!   let sim = always!(
//!     [cycle.lt(stl::int(20))]
//!     () {
//!       cycle %= &cycle + stl::int(1);
//!       sim_print!("cycle: ", cycle, "count: ", counter.read());
//!     }
//!   );
//!
//!   let exit = always!(
//!     [cycle.ge(stl::int(20))]
//!     () {
//!       sim_exit!();
//!     }
//!   );
//!
//!   method_rel!(sim CF exit);
//! }
//!
//! // Make Verilator workspace
//! let tb = make_tb();
//! elaborate(tb, verilator_config("path/to/dir")).unwrap();
//!
//! // Make Khronos workspace
//! let tb = make_tb();
//! elaborate(tb, ksim_config("path/to/dir")).unwrap();
//! ```
//!
//! The generated Veriloator workspace is configured with CMake. The executable is named `Vsim`.
//! ```bash
//! cd path/to/dir
//! mkdir build
//! cd build
//! cmake -GNinja ..
//! ninja
//! ./Vsim
//! ```
//!
//! The generated Khonos workspace is configured with MakeFile. The executable have the same name as the top module.
//! ```bash
//! cd path/to/dir
//! make all
//! ./name_of_top_module
//! ```
//!
pub mod gen_ir;
pub mod interface;
pub mod stl;
pub mod type_sys;

pub use std::panic::Location;

use cmtir as ir;
pub use cmtir::{utils, IRDump, MethodRel, MySpan, PrintItem, RuleSignature, RuleTiming};
pub use cmtrs_macros::*;
pub use interface::*;
// pub use paste::paste;
pub use type_sys::Type;

pub fn extract_span_from_location(location: &'static Location<'static>) -> MySpan {
  let file = location.file();
  let start_line = location.line() as usize;
  let start_column = location.column() as usize;
  let end_line = location.line() as usize;
  let end_column = location.column() as usize;
  let start_byte = 0;
  let end_byte = 0;
  MySpan {
    file: file.to_string(),
    start_line,
    start_column,
    end_line,
    end_column,
    start_byte,
    end_byte,
  }
}
