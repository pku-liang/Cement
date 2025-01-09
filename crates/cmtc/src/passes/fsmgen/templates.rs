use super::*;
use cmtrs::*;
use itertools::chain;

itfc_declare! {
  pub struct FSMModule{
    finish: output Type::UInt(1),
    restartable: output Type::UInt(1),
  }
  method start();
  method finish()->(finish);
  method restartable()->(restartable);
}

#[module]
pub(crate) fn make_step(module_name: String, step_name: String) -> FSMModule {
  let io = io! {};
  set_name!(module_name);
  anno!("synthesis": "true");

  let io_step = output!(step_name.clone(), Type::UInt(1));

  let start_wire = instance!(stl::Wire::new(&Type::UInt(1)));

  named_method! {"start".to_string();
    () {
      start_wire.write(true);
    }
  };

  named_always! {"start_dft".to_string();
    () { start_wire.write(false); }
  };

  named_method! {
    step_name.clone();
    () -> (io_step) {
      start_wire.read()
    }
  };

  named_method! {"finish".to_string();
    () -> (io.finish) {
      ret!(true)
    }
  };

  named_method! {"restartable".to_string();
    () -> (io.restartable) {
      ret!(true)
    }
  };
}

#[module]
pub(crate) fn make_seq(
  module_name: String,
  children: Vec<FSMChild>,
) -> FSMModule {
  let io = io! {};
  set_name!(module_name);
  anno!("synthesis": "true");

  let n = children.len();
  let children: Vec<FSMChild> = children
    .into_iter()
    .enumerate()
    .map(|(i, c)| FSMChild {
      module: named_instance!(format!("child{i}");c.module),
      ..c
    })
    .collect();

  let states: Vec<_> = (0..n)
    .map(
      |i| named_instance!(format!("state{i}");stl::reg_init(&Type::UInt(1),0)),
    )
    .collect();
  let state_fin = instance!(stl::reg_init(&Type::UInt(1), 0));
  let finish_wire = instance!(stl::wire(&Type::UInt(1)));

  generate!(make_io_methods(&children));

  let mut finish_rules = Vec::new();
  finish_rules.push(named_always! {"finish_idle".to_string();
    [!states.iter().map(|x|x.read()).reduce(|a,b|a|b).unwrap()]
    () { finish_wire.write(true); }
  });

  named_method! { "start".to_string();
    () {
      states[0].write(true);
      children.first().unwrap().module.start();
    }
  };

  for (i, c) in children.iter().enumerate() {
    named_always! {
      format!("update_{i}");
      [states[i].read() & c.module.finish()]
      () {
        states[i].write(false);
      }
    };
    if i + 2 < n {
      named_always! {
        format!("next_{i}");
        [states[i].read() & c.module.finish()]
        () {
          states[i+1].write(true);
          children[i+1].module.start();
        }
      };
    } else if i + 2 == n {
      named_always! {
        format!("next_{i}_start");
        [states[i].read() & c.module.finish()]
        () { children[i+1].module.start(); }
      };
      named_always! {
        format!("next_{i}");
        [states[i].read() & c.module.finish() & !children[i+1].module.restartable()]
        () { states[i+1].write(true); }
      };
      named_always! {
        format!("next_{i}_fin");
        [states[i].read() & c.module.finish() & children[i+1].module.restartable()]
        () { state_fin.write(true); }
      };
    } else if i + 1 == n {
      finish_rules.push(named_always! {
        format!("next_{i}");
        [states[i].read() & c.module.finish()]
        () { finish_wire.write(true); }
      });
    }
  }
  finish_rules.push(named_always! {"do_state_finish".to_string();
    [&state_fin] () { finish_wire.write(true); }
  });
  generate!(cf_all(&finish_rules));

  named_always! {"update_state_finish".to_string();
    [&state_fin] () { state_fin.write(false); }
  };

  named_always! { "finish_dft".to_string();
    () { finish_wire.write(false); }
  };
  named_method! {"finish".to_string();
    () -> (io.finish) { finish_wire.read() }
  };

  named_method! {"restartable".to_string();
    () -> (io.restartable) {
      ret!((states[n-1].read()| (states[n-2].read() & children[n-2].module.finish())) & children.last().unwrap().module.restartable())
    }
  };
}

#[module]
pub(crate) fn make_par(
  module_name: String,
  children: Vec<FSMChild>,
) -> FSMModule {
  let io = io! {};
  set_name!(module_name);
  anno!("synthesis": "true");

  let children: Vec<FSMChild> = children
    .into_iter()
    .enumerate()
    .map(|(i, c)| FSMChild {
      module: named_instance!(format!("child{i}");c.module),
      ..c
    })
    .collect();

  let mut state_exec = instance!(stl::reg_init(&Type::UInt(1), 0));
  let mut state_fin = instance!(stl::reg_init(&Type::UInt(1), 0));
  let mut finish_wire = instance!(stl::wire(&Type::UInt(1)));
  let mut start_wire = instance!(stl::wire(&Type::UInt(1)));

  generate!(make_io_methods(&children));

  let mut finish_rules = Vec::new();
  finish_rules.push(named_always! {"finish_idle".to_string();
    [!state_exec.read()]
    () { finish_wire %= true; }
  });

  named_method! {"start".to_string();
    () {
      for c in &children {
        c.module.start();
      }
      start_wire %= true;
    }
  };
  named_always! {"start_dft".to_string();
    (){ start_wire %= false; }
  };
  named_always!("next_start_exec".to_string();
    [&start_wire & !children.iter().map(|c|c.module.restartable()).reduce(|a, b| a & b).unwrap() ]
    () { state_exec %= true; }
  );
  named_always!("next_start_fin".to_string();
    [&start_wire & children.iter().map(|c|c.module.restartable()).reduce(|a, b| a & b).unwrap() ]
    () { state_fin %= true; }
  );

  named_always! { "update_state_exec".to_string();
    [&state_exec.read() & children.iter().map(|c|c.module.finish()).reduce(|a, b| a & b).unwrap()]
    () { state_exec %= false; }
  };

  finish_rules.push( named_always! {"state_finish_write".to_string();
    [state_exec.read() & children.iter().map(|c|c.module.finish()).reduce(|a, b| a & b).unwrap()]
    () {
      finish_wire %= true;
    }
  });

  finish_rules.push(named_always! {"do_state_finish".to_string();
    [&state_fin] () { finish_wire.write(true); }
  });
  generate!(cf_all(&finish_rules));

  named_always! {"update_state_finish".to_string();
    [&state_fin] () { state_fin.write(false); }
  };

  named_always! { "finish_dft".to_string();
    () { finish_wire %= false; }
  };
  named_method! {"finish".to_string();
    () -> (io.finish) { finish_wire.read() }
  };
  named_method! {"restartable".to_string();
    () -> (io.restartable) {
      ret!((start_wire.read() | state_exec.read()) & children.iter().map(|c|c.module.restartable()).reduce(|a, b| a & b).unwrap());
    }
  };
}

#[module]
pub(crate) fn make_branch(
  module_name: String,
  cond_name: String,
  then_body: FSMChild,
  else_body: Option<FSMChild>,
  loose: bool,
) -> FSMModule {
  let io = io! {};
  set_name!(module_name);
  anno!("synthesis": "true");

  // assert!(loose, "Tight branch is not supported yet");

  let io_cond = input!(cond_name.clone(), Type::UInt(1));

  let state_cond = if loose {
    Some(instance!(stl::reg_init(&Type::UInt(1), 0)))
  } else {
    None
  };

  let state_then = instance!(stl::reg_init(&Type::UInt(1), 0));

  let state_else = if else_body.is_some() {
    Some(instance!(stl::reg_init(&Type::UInt(1), 0)))
  } else {
    None
  };

  let all_states: Vec<_> = chain!(
    state_cond.as_ref().into_iter(),
    [&state_then],
    state_else.as_ref().into_iter()
  )
  .collect();

  let mut finish_wire = instance!(stl::wire(&Type::UInt(1)));

  let child0 = instance!(then_body.module);
  let mut input_methods: Vec<_> = then_body
    .in_rules
    .iter()
    .map(|i| {
      let io = input!(i.clone(), cmtrs::Type::UInt(1));
      let rule = named_method! {
        i.clone();
        (io)  { child0.invoke(&i, &[&io], 0, None); }
      };
      rule
    })
    .collect();
  let mut output_methods: Vec<RuleHandle> = then_body
    .out_rules
    .iter()
    .map(|o| {
      let io = output!(o.clone(), Type::UInt(1));
      let rule = named_method! {
        o.clone();
        () -> (io) { ret!(child0.invoke(&o, &[], 1, None)) }
      };
      rule
    })
    .collect();
  let child1 = if let Some(else_body) = else_body {
    let child1 = instance!(else_body.module);
    for i in else_body.in_rules {
      let io = input!(i.clone(), cmtrs::Type::UInt(1));
      let rule = named_method! {
        i.clone();
        (io)  { child1.invoke(&i, &[&io], 0, None); }
      };
      input_methods.push(rule);
    }
    for o in else_body.out_rules {
      let io = output!(o.clone(), Type::UInt(1));
      let rule = named_method! {
        o.clone();
        () -> (io) { ret!(child1.invoke(&o, &[], 1, None)) }
      };
      output_methods.push(rule);
    }
    Some(child1)
  } else {
    None
  };

  let mut state_fin = instance!(stl::reg_init(&Type::UInt(1), 0));

  let mut finish_rules = Vec::new();
  finish_rules.push(named_always! {"finish_idle".to_string();
    [!all_states.iter().map(|x|x.read()).reduce(|a,b|a&b).unwrap()]
    () { finish_wire %= true; }
  });

  if let Some(state_cond) = state_cond.as_ref() {
    // loose
    let mut cond_reg = instance!(stl::reg_init(&Type::UInt(1), 0));
    named_method! {cond_name;
      (io_cond) { cond_reg %= io_cond; }
    };
    named_method!("start".to_string();
      () { state_cond.write(true); }
    );
    named_always!("update_state_cond".to_string();
      [state_cond.read()]
      () { state_cond.write(false); }
    );
    named_always!("start_then".to_string();
      [state_cond.read() & cond_reg.read()]
      () { child0.start(); }
    );
    named_always!("next_state_cond_then".to_string();
      [state_cond.read() & cond_reg.read() &!child0.restartable()]
      () { state_then.write(true); }
    );
    let next_cond_then_fin = always!(
      [state_cond.read() & cond_reg.read() &child0.restartable()]
      () { state_fin.write(true); }
    );
    if let Some(state_else) = state_else.as_ref() {
      let child1 = child1.as_ref().unwrap();
      named_always!("start_else".to_string();
        [state_cond.read() & !cond_reg.read()]
        () { child1.start(); }
      );
      named_always!("next_state_cond_else".to_string();
        [state_cond.read() & !cond_reg.read() &!child1.restartable()]
        () { state_else.write(true); }
      );
      let next_cond_else_fin = always!(
        [state_cond.read() & !cond_reg.read() &child1.restartable()]
        () { state_fin.write(true); }
      );
      method_rel!(next_cond_then_fin CF next_cond_else_fin);
      named_method! {"restartable".to_string();
        () -> (io.restartable) { ret!(
          (((state_cond.read() & cond_reg.read()) | state_then.read()) & child0.restartable()) |
          (((state_cond.read() & !cond_reg.read()) | state_else.read()) & child1.restartable())
        ) }
      };
    } else {
      named_method! {"restartable".to_string();
        () -> (io.restartable) { ret!(
          (((state_cond.read() & cond_reg.read()) | state_then.read()) & child0.restartable()) |
          (state_cond.read() & !cond_reg.read())
        ) }
      };
    }
  } else {
    let mut cond_wire = instance!(stl::wire(&Type::UInt(1)));
    named_method! {cond_name;
      (io_cond) { cond_wire %= io_cond; }
    };
    named_always! { "cond_dft".to_string();
      () { cond_wire %= false; }
    };

    let mut start_wire = instance!(stl::Wire::new(&Type::UInt(1)));
    named_method!("start".to_string();
      () { start_wire %= true; }
    );
    named_always!("start_dft".to_string();
      () { start_wire %= false; }
    );
    named_always!("start_then".to_string();
      [start_wire.read() & cond_wire.read()]
      () { child0.start(); }
    );
    named_always!("next_start_then".to_string();
      [start_wire.read() & cond_wire.read() &!child0.restartable()]
      () { state_then.write(true); }
    );
    let next_start_then_fin = always!(
      [start_wire.read() & cond_wire.read() &child0.restartable()]
      () { state_fin.write(true); }
    );
    if let Some(state_else) = state_else.as_ref() {
      let child1 = child1.as_ref().unwrap();
      named_always!("start_else".to_string();
        [start_wire.read() & !cond_wire.read()]
        () {
          child1.start();
        }
      );
      named_always!("next_start_else".to_string();
        [start_wire.read() & !cond_wire.read() &!child1.restartable()]
        () { state_else.write(true); }
      );
      let next_start_else_fin = always!(
        [start_wire.read() & !cond_wire.read() &child1.restartable()]
        () { state_fin.write(true); }
      );
      method_rel!(next_start_then_fin CF next_start_else_fin);
      named_method! {"restartable".to_string();
        () -> (io.restartable) { ret!(
          ((state_then.read() | (start_wire.read() & cond_wire.read())) & child0.restartable()) |
          ((state_else.read() | (start_wire.read() & !cond_wire.read())) & child1.restartable())
        ) }
      };
    } else {
      named_method! {"restartable".to_string();
        () -> (io.restartable) { ret!(
          ((state_then.read() | (start_wire.read() & cond_wire.read())) & child0.restartable()) |
          (start_wire.read() & !cond_wire.read())
        ) }
      };
    }
  }

  named_always!("update_state_then".to_string();
    [state_then.read() & child0.finish()]
    () { state_then.write(false); }
  );
  finish_rules.push(named_always!("fin_state_then".to_string();
    [state_then.read() & child0.finish()]
    () { finish_wire %= true; }
  ));

  if let Some(state_else) = state_else.as_ref() {
    named_always!("update_state_else".to_string();
      [state_else.read() & child1.as_ref().unwrap().finish()]
      () { state_else.write(false); }
    );
    finish_rules.push(named_always!("fin_state_else".to_string();
      [state_else.read() & child1.as_ref().unwrap().finish()]
      () { finish_wire %= true; }
    ));
  }

  finish_rules.push(named_always! {"do_state_finish".to_string();
    [&state_fin] () { finish_wire.write(true); }
  });
  generate!(cf_all(&finish_rules));

  named_always! {"update_state_finish".to_string();
    [&state_fin] () { state_fin.write(false); }
  };
  named_always! { "finish_dft".to_string();
    () { finish_wire.write(false); }
  };
  named_method! {"finish".to_string();
    () -> (io.finish) { finish_wire.read() }
  };
}

// T: (1+c)*n+2+init
#[module]
pub(crate) fn make_loose_for(
  module_name: String,
  init_name: Option<String>,
  cond_name: String,
  update_name: String,
  body: FSMChild,
) -> FSMModule {
  let io = io! {};
  set_name!(module_name);
  anno!("synthesis": "true");

  let child = instance!(body.module);

  let io_cond = input!(cond_name.clone(), Type::UInt(1));
  let io_update = output!(update_name.clone(), Type::UInt(1));
  let mut state_cond = instance!(stl::reg_init(&Type::UInt(1), 0));
  let mut state_post_cond = instance!(stl::reg_init(&Type::UInt(1), 0));
  let mut state_exec = instance!(stl::reg_init(&Type::UInt(1), 0));
  let mut finish_wire = instance!(stl::wire(&Type::UInt(1)));
  let mut cond_reg = instance!(stl::reg_init(&Type::UInt(1), 0));
  let init_wire = if init_name.is_some() {
    Some(instance!(stl::wire(&Type::UInt(1))))
  } else {
    None
  };
  let mut update_wire = instance!(stl::wire(&Type::UInt(1)));

  for i in &body.in_rules {
    let io = input!(i.clone(), cmtrs::Type::UInt(1));
    named_method! {
      i.clone();
      (io)  { child.invoke(&i, &[&io], 0, None); }
    };
  }

  for o in &body.out_rules {
    let io = output!(o.clone(), Type::UInt(1));
    named_method! {
      o.clone();
      () -> (io) { ret!(child.invoke(&o, &[], 1, None)) }
    };
  }

  named_method! {cond_name;
    (io_cond) { cond_reg %= io_cond; }
  };
  named_always! { "cond_dft".to_string();
    () { cond_reg %= false; }
  };

  let finish_idle = always! {
    [!state_cond.read() & !state_exec.read() & !state_post_cond.read()]
    () { finish_wire.write(true); }
  };

  let start = method! {
    () {
      if let Some(init_wire) = &init_wire {
        init_wire.write(true);
        state_cond %= true;
      } else {
        state_post_cond %= true;
      }
    }
  };

  let update_state_cond = always!(
    [state_cond.read()]
    () { state_cond %= false; }
  );
  let next_state_cond = always!(
    [state_cond.read()]
    () {
      state_post_cond %= true;
    }
  );

  named_always!("update_state_post_cond".to_string();
    [state_post_cond.read()]
    () {state_post_cond %= false;}
  );
  named_always!("do_state_post_cond".to_string();
    [state_post_cond.read() & cond_reg.read()]
    () { child.start(); }
  );
  let do_state_post_not_cond = always!(
    [state_post_cond.read() & !cond_reg.read()]
    () { finish_wire %= true; }
  );
  method_rel!(finish_idle CF do_state_post_not_cond);

  named_always! {"next_state_post_cond_exec".to_string();
    [state_post_cond.read() & cond_reg.read() & !child.restartable()]
    () { state_exec %= true; }
  };

  named_always!("update_state_exec".to_string();
    [&state_exec & child.restartable()]
    () {
      state_exec %= false;
    }
  );
  let next_iter = always!(
    [(state_exec.read() | (state_post_cond.read() & cond_reg.read())) & child.restartable()]
    () {
      update_wire.write(true);
      state_cond %= true;
    }
  );
  schedule!(next_iter, update_state_cond);
  if init_wire.is_some() {
    method_rel!(start CF next_iter);
  } else {
    method_rel!(start CF next_state_cond);
  }

  named_always! {"finish_dft".to_string();
    () { finish_wire %= false; }
  };
  named_method! {"finish".to_string();
    () -> (io.finish) { finish_wire.read() }
  };

  if let Some(init_wire) = init_wire {
    named_always! {"init_dft".to_string();
      () { init_wire.write(false); }
    };
    let init_name = init_name.unwrap();
    let io = output!(init_name.clone(), Type::UInt(1));
    named_method! {init_name;
      () -> (io) { init_wire.read() }
    };
  }
  named_always! {"update_dft".to_string();
    () { update_wire %= false; }
  };
  named_method! {update_name;
    () -> (io_update) { update_wire.read() }
  };

  named_method! {"restartable".to_string();
    () -> (io.restartable) { ret!(state_post_cond.read() & !cond_reg.read()) }
  };
}

#[module]
pub(crate) fn make_tight_for(
  module_name: String,
  init_name: Option<String>,
  init_cond_name: String,
  update_name: String,
  update_cond_name: String,
  body: FSMChild,
) -> FSMModule {
  let io = io! {};
  set_name!(module_name);
  anno!("synthesis": "true");

  let child = instance!(body.module);

  let io_init_cond = input!(init_cond_name.clone(), Type::UInt(1));
  let io_update = output!(update_name.clone(), Type::UInt(1));
  let io_update_cond = input!(update_cond_name.clone(), Type::UInt(1));
  let mut state_start = instance!(stl::reg_init(&Type::UInt(1), 0));
  let mut state_exec = instance!(stl::reg_init(&Type::UInt(1), 0));
  let mut state_finish = instance!(stl::reg_init(&Type::UInt(1), 0));
  let mut finish_wire = instance!(stl::wire(&Type::UInt(1)));
  let mut ucond_wire = instance!(stl::wire(&Type::UInt(1)));
  let mut icond_wire = instance!(stl::wire(&Type::UInt(1)));
  let mut start_wire = instance!(stl::wire(&Type::UInt(1)));
  let init_wire = if init_name.is_some() {
    Some(instance!(stl::wire(&Type::UInt(1))))
  } else {
    None
  };
  let mut update_wire = instance!(stl::wire(&Type::UInt(1)));

  for i in &body.in_rules {
    let io = input!(i.clone(), cmtrs::Type::UInt(1));
    named_method! {
      i.clone();
      (io)  { child.invoke(&i, &[&io], 0, None); }
    };
  }

  for o in &body.out_rules {
    let io = output!(o.clone(), Type::UInt(1));
    named_method! {
      o.clone();
      () -> (io) { ret!(child.invoke(&o, &[], 1, None)) }
    };
  }

  named_method! {init_cond_name;
    (io_init_cond) { icond_wire %= io_init_cond; }
  };
  named_always! { "icond_dft".to_string();
    () { icond_wire %= false; }
  };
  named_method! {update_cond_name;
    (io_update_cond) { ucond_wire %= io_update_cond; }
  };
  named_always! { "ucond_dft".to_string();
    () { ucond_wire %= false; }
  };

  let finish_idle = always! {
    [!state_exec.read() & !state_start.read()]
    () { finish_wire.write(true); }
  };

  named_method! {"start".to_string();
    () {
      start_wire %= true;
    }
  };
  named_always!("start_dft".to_string();
    (){ start_wire %= false; }
  );

  let mut update_rules = Vec::new();
  let mut to_start_rules = Vec::new();
  let mut do_start_rules = Vec::new();
  let mut to_exec_rules = Vec::new();
  let mut to_finish_rules = Vec::new();

  if let Some(init_wire) = &init_wire {
    named_always!("do_init".to_string();
      [&start_wire]
      (){ init_wire.write(true); }
    );

    to_start_rules.push(named_always!("next_start".to_string();
      [&start_wire & icond_wire.read()]
      (){ state_start %= true; }
    ));
  } else {
    do_start_rules.push(named_always!("start_child".to_string();
      [&start_wire & icond_wire.read()]
      (){ child.start() }
    ));
    to_start_rules.push(named_always!("next_start_restart".to_string();
      [&start_wire & icond_wire.read() & child.restartable() & ucond_wire.read()]
      (){ state_start %= true; }
    ));
    to_finish_rules.push(named_always!("next_start_fin".to_string();
      [&start_wire & icond_wire.read() & child.restartable() & !ucond_wire.read()]
      (){ state_finish %= true; }
    ));
    to_exec_rules.push(named_always!("next_start_exec".to_string();
      [&start_wire & icond_wire.read() & !child.restartable()]
      (){ state_exec %= true; }
    ));
    update_rules.push(named_always!("start_do_update".to_string();
      [&start_wire & icond_wire.read() & child.restartable()]
      (){ update_wire %= true; }
    ));
  }

  let update_state_start = always!(
    [state_start.read()]
    () {state_start %= false;}
  );
  do_start_rules.push(named_always!("do_state_start".to_string();
    [state_start.read()]
    () { child.start(); }
  ));
  update_rules.push(named_always!("state_start_do_update".to_string();
    [state_start.read() & child.restartable()]
    () { update_wire %= true; }
  ));
  to_start_rules.push(named_always!("next_state_start_restart".to_string();
    [state_start.read() & child.restartable() & ucond_wire.read()]
    () {state_start %= true;}
  ));

  to_finish_rules.push(named_always!("next_state_start_fin".to_string();
    [state_start.read() & child.restartable() & !ucond_wire.read()]
    () {state_finish %= true;}
  ));

  to_exec_rules.push(named_always!("next_state_start_exec".to_string();
    [state_start.read() & !child.restartable()]
    () {state_exec %= true;}
  ));

  named_always!("update_state_exec".to_string();
    [state_exec.read() & child.restartable()]
    () {state_exec %= false;}
  );
  update_rules.push(named_always!("state_exec_do_update".to_string();
    [state_exec.read() & child.restartable()]
    () { update_wire %= true; }
  ));
  to_start_rules.push(named_always!("next_state_exec_restart".to_string();
    [state_exec.read() & child.restartable() & ucond_wire.read()]
    () {state_start %= true;}
  ));
  to_finish_rules.push(named_always!("next_state_exec_fin".to_string();
    [state_exec.read() & child.restartable() & !ucond_wire.read()]
    () {state_finish %= true;}
  ));

  named_always!("update_state_finish".to_string();
    [&state_finish] () { state_finish %= false; }
  );
  let do_state_finish = always!(
    [&state_finish] () { finish_wire %= true; }
  );
  method_rel!(finish_idle CF do_state_finish);

  generate!(cf_all(&to_start_rules));
  for r in to_start_rules {
    schedule!(r, update_state_start);
  }
  generate!(cf_all(&update_rules));
  generate!(cf_all(&do_start_rules));
  generate!(cf_all(&to_exec_rules));
  generate!(cf_all(&to_finish_rules));

  named_always! {"finish_dft".to_string();
    () { finish_wire %= false; }
  };
  named_method! {"finish".to_string();
    () -> (io.finish) { finish_wire.read() }
  };

  if let Some(init_wire) = &init_wire {
    named_always! {"init_dft".to_string();
      () { init_wire.write(false); }
    };
    let init_name = init_name.unwrap();
    let io = output!(init_name.clone(), Type::UInt(1));
    named_method! {init_name;
      () -> (io) { init_wire.read() }
    };
  }
  named_always! {"update_dft".to_string();
    () { update_wire %= false; }
  };
  named_method! {update_name;
    () -> (io_update) { update_wire.read() }
  };

  if init_wire.is_some() {
    named_method! {"restartable".to_string();
      () -> (io.restartable) { ret!(
        (start_wire.read() & !icond_wire.read()) |
        (state_start.read() & child.restartable() & !ucond_wire.read()) |
        (state_exec.read() & child.restartable() & !ucond_wire.read())
      ) }
    };
  } else {
    named_method! {"restartable".to_string();
      () -> (io.restartable) { ret!(
        (start_wire.read() & !icond_wire.read()) |
        (start_wire.read() & icond_wire.read() & child.restartable() & !ucond_wire.read()) |
        (state_start.read() & child.restartable() & !ucond_wire.read()) |
        (state_exec.read() & child.restartable() & !ucond_wire.read())
      ) }
    };
  }
}

#[module]
pub(crate) fn make_static_pipeline(
  module_name: String,
  children: Vec<FSMChild>,
) -> FSMModule {
  let io = io! {};
  set_name!(module_name);
  anno!("synthesis": "true");

  let children: Vec<FSMChild> = children
    .into_iter()
    .enumerate()
    .map(|(i, c)| FSMChild {
      module: named_instance!(format!("child{i}");c.module),
      ..c
    })
    .collect();

  let mut schedule = Vec::new();

  for i in (0..children.len()).rev() {
    let module = &children[i].module;
    // input
    schedule.extend(children[i].in_rules.iter().map(|i| {
      let io = input!(i.clone(), cmtrs::Type::UInt(1));
      let rule = named_method! {
        i.clone();
        (io)  {
          module.invoke(&i, &[&io], 0, None);
        }
      };
      rule
    }));
    // output
    schedule.extend(children[i].out_rules.iter().map(|o| {
      let io = output!(o.clone(), Type::UInt(1));
      let rule = named_method! {
        o.clone();
        () -> (io) { ret!(module.invoke(&o, &[], 1, None)) }
      };
      rule
    }));

    if i != 0 {
      let rule = named_always! {format!("stage{i}");
        [children[i-1].module.finish()]
        () {
          children[i].module.start();
        }
      };
      schedule.push(rule);
    } else {
      let start = method! {
        () { children[0].module.start(); }
      };
      let finish = method! {
        () -> (io.finish) { children[0].module.finish() }
      };
      schedule.push(start);
      schedule.push(finish);
    }
  }
  named_method!("restartable".to_string();
    () -> (io.restartable) { children[0].module.restartable() }
  );
  schedule_raw!(&schedule);
}

#[gen_fn]
fn make_io_methods(children: &[FSMChild]) {
  for c in children {
    for i in &c.in_rules {
      let io = input!(i.clone(), cmtrs::Type::UInt(1));
      named_method! {
        i.clone();
        (io)  {
          c.module.invoke(&i, &[&io], 0, None);
        }
      };
    }
  }
  for c in children {
    for o in &c.out_rules {
      let io = output!(o.clone(), cmtrs::Type::UInt(1));
      named_method! {
        o.clone();
        ()->(io)  {
          c.module.invoke(&o, &[], 1, None);
        }
      };
    }
  }
}

#[gen_fn]
fn cf_all(rules: &[RuleHandle]) {
  for i in 0..rules.len() {
    for j in i + 1..rules.len() {
      method_rel!(rules[i] CF rules[j]);
    }
  }
}
