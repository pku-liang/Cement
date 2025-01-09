# cmtir

[<img alt="Static Badge" src="https://img.shields.io/badge/crates-cmtir-orange?style=for-the-badge&logo=rust">](https://crates.io/crates/cmtir)
[<img alt="Static Badge" src="https://img.shields.io/badge/doc-cmtir-green?style=for-the-badge&logo=docs.rs">](https://docs.rs/cmtir)


## Design

### Dependent

- [kir](https://github.com/arch-of-shadow/kir): defining IR with derive-based parse-print/def-use support.

### IR Components

`src/ir` defines different components of `cmtir`:
- `ir::op`: operations.
- `ir::instance`: instance declaration and usage.
- `ir::literal`: literals.
- `ir::rule`: rules.
- `ir::structure`: module and circuit.
- `ir::error`: span and error messages.

All structs in `cmtir` have the consistent pattern of `SExpr` derive, which is a macro that provides parse-print support (see [kir::SExpr](https://github.com/arch-of-shadow/kir/blob/uv/macros/src/sexpr.rs)).

For example, the `CmpOp` is defined as:
```rust
#[derive(Debug, Clone, OpIO, SExpr)]
pub struct CmpOp {
  #[opio(output)]
  pub res: ValueId,
  #[opio(attr)]
  pub cmp: Cmp,
  #[opio(input)]
  pub a: ValueId,
  #[opio(input)]
  pub b: ValueId,
}
```
`CmpOp` parses/prints things like: "%a eq %b %c";

And for the enum `OpEnum` defined as:
```rust
#[derive(Debug, Clone, OpIO, SExpr)]
pub enum OpEnum {
  #[pp(surrounded)]
  Nop(NopOp),
  Assign(AssignOp),
  Lit(LitOp),
  Cmp(CmpOp),
  Prim(PrimOp),
  ...
}
```
It parses/prints things like: "(cmp %a eq %b %c)".

For operations, `cmtir` derives `OpIO` trait (also defined in `kir`), which provides def-use support:
```rust
pub trait OpIO {
  fn num_inputs(&self) -> usize;
  fn input(&self, i: usize) -> ValueId;
  fn inputs(&self) -> impl Iterator<Item = ValueId> + '_ {
    (0..self.num_inputs()).map(move |i| self.input(i))
  }
  ...
}
```

For most operations, the `OpIO` trait is derived automatically when `#[opio(xxx)]` is specified on the struct fields.

## Rationale

### Non-parametric module

Every module in `cmtir` is **non-parametric**. The rationale is that we want to keep the IR as simple as possible, and thereby, moving parametric modules to the frontend. That is, all parameterizations should be applied during lowering to `cmtir`.

### Implicit ports

`cmtir` is at a higher-level compared to RTL HDLs. It is rule-based, or transactional. Auxiliary ports (e.g., `enable`, `ready`, `fire`, etc.) are hidden from the user (implicit) and will be inserted automatically when compiling to RTL. The rationale is that, for transactional modules, we are more interested in the rule-level behavior of the module, rather than detailed signals and connections. What's more, users can explicitly specify the ports through annotations on rules (currently, only supported on external modules).

### External modules

`cmtir` accepts **FIRRTL code as external modules**. This is useful for reusing existing modules from the FIRRTL (or Chisel, Chipyard) ecosystem. Also, since FIRRTL supports blackbox SV modules, `cmtir` is also compatible with SV legacy code indirectly. The question is, how `cmtir` treat RTL external modules in the rule-based semantics? `cmtir` provides syntax to declare **external rules** for external modules, each of which specifies the real hardware behavior of the corresponding rule, like setting up some signals. `cmtir` also provides syntax to **bind** `cmtir` module ports and RTL ports in external modules.

Here is an example (perhaps the easiest one), a register:

```cmtir
(module "Reg_i8" (
    (annotations {"synthesis":"true","cmtrs_span":"..."})
    (inputs (%in:i8))
    (outputs (%out:i8))
    (wires ())
    (ext
      (bindings ("write_data") ("read_data")) (clock "clock") (reset "reset") (fir
module Reg_i8:
  input write_en: UInt<1>
  input write_data: UInt<8>
  input clock: Clock
  input reset: UInt<1>
  output read_data: UInt<8>

  reg r: UInt<8>, clock

  connect read_data, r
  when write_en:
    connect r, write_data))
    (rules
      ((ext? true) (private? false) rule "read" (method () (%out:i8) false) (single_cycle) _ _ () ()) {"cmtrs_span":"..."}
      ((ext? true) (private? false) rule "write" (method (%in:i8) () true) (single_cycle) "write_en" _ () ()) {"cmtrs_span":"..."}
    )
    (rule_rels
      (schedule (self.read self.write))
    )
  ))
  ```
