# TGraph README

## Intro

TGraph is short for transactional graph, which aims to provide a graph-like data structure while solving the disturbing problems caused by mutable references and lifetimes in Rust.

The graph is connected by generated indexes instead of references or smart pointers to avoid lifetime problems. Here is an example of an `Edge` structure, which used `NodeIndex` to indicate the connected nodes.

```rust
struct Edge<DataT>{
    data: DataT,
    from: NodeIndex,
    to: NodeIndex
}
```

All modifications on the graph is stored in a separated `Transaction` structure, so there is no `&mut` reference to the graph itself.

Query and update are separated by such design.

The graph **can** be queried by immutable references, but **can not** be updated except for commiting the transaction.

The transaction **can** be updated by mutable references, but **can not** be queried.

```rust
// Create a context
let context = Context::new();
// Create a graph
let mut graph = Graph::<i64, i64>::new(&context);
// Create an transaction
let mut trans = Transaction::new(&context);

// Add some nodes and edges into the transaction
let n1 = trans.new_node(1);
let n2 = trans.new_node(2);
let e1 = trans.new_edge(-1, n1, n2);

// Commite the transaction back into the graph
graph.commit(trans);
```

There are two kinds of graph structure in TGraph.

+ `tgraph::graph` is a simple directed graph, which have a node type and an edge type. Nodes and edges both holds data.
+ `tgraph::typed_graph` is a directed graph, which only have a node type. An Edge is only a NodeIndex without data. Edges in a node are categorized by "types" as members of the node struct.

## `tgraph::graph`

### Node struct

```rust
pub struct Node<NDataT> {
    idx: NodeIndex, // Index to itself
    data: NDataT,
    in_edges: HashSet<EdgeIndex>,
    out_edges: HashSet<EdgeIndex>,
}
```

### Edge struct

```rust
pub struct Edge<EDataT> {
    idx: EdgeIndex, // Index to itself
    data: EDataT,
    from: NodeIndex,
    to: NodeIndex,
}
```

### Create a graph and transaction

```rust
// Create a context
let context = Context::new();
// Create a graph
let mut graph = Graph::<NDataT, EDataT>::new(&context);
// Create an transaction, data types may be inferred
let mut trans = Transaction::<NDataT, EDataT>::new(&context);
```

### Work on transaction

Add a new node

```rust
let node_idx = trans.new_node(data);
```

Add a new edge, the `in_edges` of `to` and `out_edges` of `from` will be modified after commitment.

```rust
let edge_idx = trans.new_edge(data, from, to);
```

Remove a node or an edge that is in the graph or just added into the transaction.

```rust
trans.remove_node(node_idx);
trans.remove_edge(edge_idx);
```

Mutate the data inside of a node or edge inplace with a closure, where the data is acquired by `&mut`.

```rust
trans.mut_node(node_idx, |&mut node_data| {...});
trans.mut_edge(edge_idx, |&mut edge_data| {...});
```

Update the data inside of a node or edge out-of-place with a closure, where the ownership of the data is moved out of the container.

```rust
trans.update_node(node_idx, |old_data| {...; new_data});
trans.update_edge(edge_idx, |old_data| {...; new_data});
```

### Query

Get a node or an edge by index

```rust
let node_option = graph.get_node(node_idx);
let edge_option = graph.get_edge(edge_idx);
```

Iterate all nodes or edges

```rust
for (idx, node) in graph.iter_nodes() {
    //...
}
for (idx, edge) in graph.iter_edges() {
    //...
}
```

Iterate all in-edges or out-edges of a node

```rust
for (edge_idx, from_idx) in graph.iter_in(node_idx){
    // ...
}
for (edge_idx, to_idx) in graph.iter_out(node_idx){
    // ...
}
```

Get the number of nodes and edges

```rust
graph.len_nodes()
graph.len_edges()
```

### Commitment

Commit a transaction back into a graph. A transaction cannot be committed twice. The graph and the transaction should share the same context so the indexes does not conflict with each other.

```rust
graph.commit(trans);
```

Give up the transaction to prevent being committed into a graph.

```rust
trans.give_up();
```

Commit order:

+ Add new nodes / edges.
+ Modify nodes / edges.
+ Update nodes / edges.
+ Remove nodes / edges. So that new nodes / edges can be removed.

### Printing

If `std::fmt::Display` is implemented for `NDataT` and `EDataT`:

```rust
println!("{}", graph);
```

Result:

```txt
Graph {
  nodes: [
     Node { idx: 4, data: 4, in_edges: [], out_edges: [3, 4, ]},
     Node { idx: 5, data: 15, in_edges: [5, ], out_edges: []},
     Node { idx: 2, data: 10, in_edges: [3, ], out_edges: [5, ]},
     Node { idx: 3, data: 3, in_edges: [4, 2, ], out_edges: []},
  ],
  edges: [
     Edge{ idx: 4, data: -4, from: 4, to: 3 },
     Edge{ idx: 3, data: -3, from: 4, to: 2 },
     Edge{ idx: 5, data: -5, from: 2, to: 5 },
     Edge{ idx: 2, data: -2, from: 1, to: 3 },
  ]
}
```

## `tgraph::typed_graph`

### Node structure

Each type of node is almost purely user-defined, the general form is as followed.

```rust
#[derive(TypedNode)]
struct MyNodeType{
    // data members can have any types
    some_data: DataType1,
    some_other_data: DataType2,
    // ...

    // Edges to other nodes

    // An edge with name `a`
    a: NodeIndex
    // An edge with name `b`
    b: NodeIndex
    // An set of edges with name 'c'
    c: HashSet<NodeIndex>
    // Currently only NodeIndex or HashSet<NodeIndex> is supported
}
```

For example, an binary operation node.

```rust
#[derive(TypedNode)]
struct BinaryOp{
    op_type: OpType,
    in1: NodeIndex,
    in2: NodeIndex,
    out: NodeIndex
}
```

An reduce operation node that have indefinite inputs.

```rust
#[derive(TypedNode)]
struct ReduceOp{
    op_type: OpType,
    inputs: HashSet<NodeIndex>,
    out: NodeIndex
}
```

### NodeEnum

All types of nodes in a graph is integrated into a NodeEnum, which have the following general form.

```rust
// Should derive NodeEnum and have no generics
#[derive(NodeEnum)]
enum NodeTypeEnum {
    // Name(Type)
    A(NodeTypeA),
    B(NodeTypeB),
    Edge(Edge<i32>),
    // ...
}
```

Create a graph with the node enum

```rust
let context = Context::new();
let mut graph = Graph::<NodeTypeEnum>::new(&context);
```

### Transaction and queries

The operation on the transation and the queries on the graph is similar to `tgraph::graph`. Here only list the different ones.

Iterate nodes by type.

```rust
for (idx, node) in NodeTypeA::iter_by_type(&graph) {
    // node: NodeTypeA
    // ...
}
```

Replace a node. All nodes connecting to the old node in the graph will be modified. For example, if `a.input = b`, after replacing `b` with `c`, we will have `a.input = c`. Replacement happens after update and before remove when commit.

```rust
trans.replace_node(old_idx, new_idx);
```

### Performance notifications

The graph maintains some backward links to help the replace operation. In modify and update operations, we don't know which edges the user exactly changed, so we need to remove all the old backward links and scan the node again.
