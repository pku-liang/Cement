# TGraph macro

## TypedNode

```rust
#[TypedNode]
struct NodeName{
    x: NodeIndex, // Direct link
    ys: HashSet<NodeIndex> // Set link
    z: NIEWrap<NIEnum> // Enum link
    u: Vec<NodeIndex> // Vec link
    irelevant_members: WhatEver
}
```

Generics and visibility is taken into consideration.

### Source

Generated Source(names are camel-cased):

```rust
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
#vis enum NodeNameSource{
    X,
    Ys,
    Z,
    U(usize)
}
```

Source Iterator:

```rust
struct NodeNameSourceIterator{
    sources: Vec<(NodeIndex, NodeNameSource)>,
    cur: usize
}
impl tgraph::typed_graph::SourceIterator<NodeName> for NodeNameIterator{ 
    // ...
}

impl std::iter::Iterator for NodeNameIterator { }
impl tgraph::typed_graph::TypedNode for NodeNameIterator {
    type Source = NodeNameSource;
    type Iter = NodeNameSourceIterator;
    fn iter_source(&self) -> Self::Iter { }
    fn modify(&mut self, source: Self::Source, old_idx:NodeIndex, new_idx: NodeIndex) {
        match source{
            NodeNameSource::X => self.x = new_idx, // Direct link
            NodeNameSource::Ys => { // Set link
                self.ys.remove(old_idx);
                self.ys.insert(new_idx);
            }
            NodeNameSource::Z => { // enum link
                tgraph::typed_graph::IndexEnum::modify(&mut self.z.value, new_idx);
            }
            NodeNameSource::U(idx) => { // vec link
                self.u[idx] = new_idx;
            }
        }
    }
}
```

## NodeIndexEnum

```rust
#[derive(IndexEnum)]
enum NIEnum {
    A(NodeIndex),
    B(NodeIndex),
}
```

### NodeIndexEnum trait

Assume modify does not change the variant of the index enum

```rust
impl tgraph::typed_graph::IndexEnum for NIEnum {
    fn modify(&mut self, new_idx: NodeIndex) {
        *self = match self {
            NIEnum::A(idx) => NIEnum::A(new_idx),
            NIEnum::B(idx) => NIEnum::B(new_idx),
        };
    }
    fn index(&self) -> NodeIndex {
        match self {
            NIEnum::A(idx) => idx.clone(),
            NIEnum::B(idx) => idx.clone(),
        }
    }
}
```

## NodeEnum

```rust
#[derive(NodeEnum)]
enum NodeType {
    A(NodeA),
    B(NodeB),
}
```

Variants should have exactly the form `Name(Type)`

### GeneratedTrait

```rust
// Generated helper trait
trait TGGenTraitNodeType<'a, IterT> {
    fn iter_by_type(graph: &'a tgraph::typed_graph::Graph<NodeType>) -> IterT;
    fn get_by_type(graph: &'a tgraph::typed_graph::Graph<NodeType>, idx: tgraph::typed_graph::NodeIndex)
       -> Option<&Self>;
}

// Impl For NodeA
impl<'a> TGGenTraitNodeType<'a, IterA<'a>> for NodeA {
    fn iter_by_type(graph: &'a tgraph::typed_graph::Graph<NodeType>) -> IterA<'a> {
        IterA { it: graph.iter_nodes() }
    }
    fn get_by_type(graph: &'a tgraph::typed_graph::Graph<NodeType>, idx: tgraph::typed_graph::NodeIndex)
       -> Option<&NodeA>
    {
        // ...
    }
}

// Generated Iterator for A(NodeA)
struct IterA<'a> {
    it: tgraph::typed_graph::Iter<'a, NodeType>,
}
impl<'a> std::iter::Iterator for IterA<'a> {
    type Item = (NodeIndex, &'a NodeA);
    fn next(&mut self) -> Option<Self::Item> {
        self.it
            .next()
            .and_then(|(idx, node)| {
                if let NodeType::A(x) = &node { Some((*idx, x)) } else { None }
            })
        // Iterate and filter
    }
}
impl<'a> std::iter::FusedIterator for IterA<'a> {}
```

### SourceEnum

```rust
#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
enum NodeTypeSourceEnum {
    A(<NodeA as TypedNode>::Source),
    B(<NodeB as TypedNode>::Source),
    Edge(<Edge<i32> as TypedNode>::Source),
}

impl NodeEnum for NodeType {
    type SourceEnum = NodeTypeSourceEnum;
    fn iter_source(
        &self,
    ) -> Box<dyn Iterator<Item = (NodeIndex, Self::SourceEnum)>> {
        match self {
            Self::A(x) => {
                Box::new(
                    <NodeA as TypedNode>::iter_source(&x)
                        .map(|(idx, src)| (idx, NodeTypeSourceEnum::A(src))),
                )
            }
            Self::B(x) => {
                Box::new(
                    <NodeB as TypedNode>::iter_source(&x)
                        .map(|(idx, src)| (idx, NodeTypeSourceEnum::B(src))),
                )
            }
        }
    }
    fn modify(
        &mut self,
        source: Self::SourceEnum,
        old_idx: NodeIndex,
        new_idx: NodeIndex,
    ) {
        match self {
            Self::A(x) => {
                if let NodeTypeSourceEnum::A(src) = source {
                    <NodeA as TypedNode>::modify(x, src, old_idx, new_idx)
                } else {
                    ::core::panicking::panic_fmt(
                        format_args!("Unmatched node type and source type!"),
                    )
                }
            }
            Self::B(x) => {
                if let NodeTypeSourceEnum::B(src) = source {
                    <NodeB as TypedNode>::modify(x, src, old_idx, new_idx)
                } else {
                    ::core::panicking::panic_fmt(
                        format_args!("Unmatched node type and source type!"),
                    )
                }
            }
        }
    }
}
```