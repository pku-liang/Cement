use cmtc::*;
use cmtrs::stl::*;
use cmtrs::*;

itfc_declare!(
  param T;
  struct Mac{
    a: input param T,
    b: input param T,
    c: input param T,
    y: output param T,
  }
  method mac(a, b, c) -> (y);
);

#[module]
fn mac(t: &Type) -> Mac {
  let io = io! {T: t};

  named_method!("mac".to_string();
    (io.a, io.b, io.c) -> (io.y) { ret!(io.a * io.b + io.c) }
  );
}

itfc_declare!(
  param DT;
  param AT;
  struct GEMM {
    amem : itfc Mem1r1w2d{DT: param AT, AT: param AT},
    bmem : itfc Mem1r1w2d{DT: param DT, AT: param AT},
    cmem : itfc Mem1r1w2d{DT: param DT, AT: param AT},
    finish: output Type::UInt(1),
  };
  method start();
  method finish() -> (finish);
);

#[module]
fn gemm(n: usize, dt: &Type) -> GEMM {
  let aw = (n * n).ilog2();
  let at = Type::UInt(aw);

  let io = io! {
     DT: dt,AT: &at,
    amem: mem1r1w2d(dt, &at, n, n),
    bmem: mem1r1w2d(dt, &at, n, n),
    cmem: mem1r1w2d(dt, &at, n, n),
  };
  anno!("synthesis": "true");

  let mut i = instance!(reg(&at));
  let mut j = instance!(reg(&at));

  let mac = instance!(mac(dt));
  let mut acc = instance!(reg(dt));

  let mut finish_wire = instance!(wire(&Type::UInt(1)));

  let start = method!(
    fsm;
    () { seq!(
      for_!((
        i %= 0.lit(&at); // init
        0.lit(&at).lt(n.lit(&at)); // init_cond
        i %= &i + 1.lit(&at);  // update
        i.lt((n-1).lit(&at)) // update_cond
      ) {
        for_!((j %= 0.lit(&at); 0.lit(&at).lt(n.lit(&at)) ; j %= &j + 1.lit(&at); j.lt((n-1).lit(&at))) {
          seq!{
            for k in 0..n {
              step!{
                io.amem.rd0(&i, k.lit(&at));
                io.bmem.rd0(k.lit(&at), &j);
                if k != 0 {
                  acc %= mac.mac(io.amem.rd1(), io.bmem.rd1(), &acc);
                }
              }
            }
            step!{
              io.cmem.write(mac.mac(io.amem.rd1(), io.bmem.rd1(), &acc), &i, &j);
              acc %= 0.lit(dt);
            }
          }
        })
      });
      step!{finish_wire %= true;};
    )}
  );

  let fin_default = always! {
    () { finish_wire %= false; }
  };

  let finish = method! {
    () -> (io.finish) { finish_wire.read() }
  };
}

itfc_declare!(
  struct Tb {}
);

#[module]
fn make_tb(n: usize, dt: &Type) -> Tb {
  io! {};
  anno!("is_tb": "true");

  let gemm = instance!(gemm(n, dt));

  let mut cycle = instance!(Integer::new());

  named_always! {format!("input");
    [cycle.lt(int(n*n))]
    (){
      let i = int_as(&cycle / int(n), &gemm.AT);
      let j = int_as(&cycle % int(n), &gemm.AT);
      let a = var!(&i+&j);
      let b = var!((&i ^ &j) & 1.lit(&gemm.AT));
      sim_print!("Input ", i, " ", j, ": a[", i, "][", j,"]=", a," b[", j, "][", i,"]=", b);
      gemm.amem.write(a, &i, &j);
      gemm.bmem.write(b, &j, &i);
    }
  };

  let start = always!(
    [cycle.eq(int(n*n))]
    () {
      gemm.start();
      sim_print!("Start at cycle ", cycle);
    }
  );

  let k_cycle = n + 1;
  let j_cycle = k_cycle * n + 1;
  let i_cycle = j_cycle * n + 1;
  let end_cycle = i_cycle + 1 + n * n;

  let out_addr_offset = end_cycle + 1;
  named_always! {format!("out_addr");
    [cycle.lt(int(n*n+out_addr_offset)) & cycle.ge(int(out_addr_offset))]
    (){
      let i = int_as((&cycle - int(out_addr_offset)) / int(n), &gemm.AT);
      let j = int_as((&cycle - int(out_addr_offset)) % int(n), &gemm.AT);
      gemm.cmem.rd0(i, j);
    }
  };
  let done = always!(
    [gemm.finish()]
    () {
      sim_print!("Complete at cycle ", cycle);
    }
  );
  let out_data_offset = end_cycle + 3;
  named_always! {format!("out_data");
    [cycle.lt(int(n*n+out_data_offset)) & cycle.ge(int(out_data_offset))]
    (){
      let i = (&cycle - int(out_data_offset)) / int(n);
      let j =(&cycle - int(out_data_offset)) % int(n);
      sim_print!("C[", i, "][", j, "]= ", gemm.cmem.rd1());
    }
  };

  let finish = always!(
    [cycle.eq(n*n+end_cycle+3)]
    () {
      sim_print!("Finish at cycle ", cycle);
      sim_exit!();
    }
  );

  let tick = always!(
    () {
      cycle %= &cycle + int(1);
    }
  );
}

fn main() -> anyhow::Result<()> {
  utils::setup_logger();
  let gemm = gemm(16, &Type::SInt(16));
  elaborate(gemm, sv_config("gemm.sv"))?;

  let tb = make_tb(16, &Type::SInt(16));
  elaborate(tb, ksim_config("./tb/gemm_ksim"))?;
  Ok(())
}
