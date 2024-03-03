# Cement

## Usage

### Setup

First, clone the repo.

```bash
git clone git@github.com:pku-liang/Cement.git
```

Then, initialize the submodules: `irony` for IR and passes, `tgraph` for control synthesis.

```bash
git submodule update --init
```

### Examples

We provide examples under `cement_examples/tsc/tests`;

For example, `cement_examples/src/tests/basics.rs` contains a `Pass::pass_m` module and a `TopPass::top_m` module, where `Pass` and `TopPass` are types for module interfaces.

Then the test function `test_top_pass` print the produced CIRCT IR. The function can be run by the following command:

```bash
cargo test --package cement_examples --bin cement_examples -- tests::basics::test_top_pass  --exact --nocapture
```

If you want to look at the produced SystemVerilog, see `cement_examples/src/tests/file_sys.rs`, and run:

```bash
cargo test --package cement_examples --bin cement_examples -- tests::file_sys::test_fs --exact --nocapture 
```

## TODO

The following features are under **RECONSTRUCTION**, and we will make them available as soon as they get ready:

* Control synthesis: we are working on a new control synthesis engine (fixing bugs), which is based on the `tgraph` library.
* Exernal IPs or Verilog import
* Timing analysis: statis analysis and dynamic monitering
* Pure-Rust RTL simulation
