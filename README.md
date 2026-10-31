# rusty_graph

[![Binder](https://mybinder.org/badge_logo.svg)](https://mybinder.org/v2/gh/LosCommodore/RustyGraph/HEAD?labpath=petgraph_dag.ipynb)

A small playground for exploring the [petgraph](https://docs.rs/petgraph) crate in Rust.

It builds a tiny computation graph, a **directed acyclic graph (DAG)**. Only the input layer starts with values; every other node is a sum (`+`) or a difference (`−`) and starts empty. Then it:

1. checks that the graph really is acyclic (`is_cyclic_directed`, `toposort`),
2. shows what happens when a cycle is added, and finds it with `tarjan_scc`,
3. sorts the nodes into layers (longest-path layering),
4. fills in the empty nodes layer by layer, since every node's inputs sit in earlier layers,
5. draws the graph with Graphviz, coloured by layer and labelled with the results.

The graph, from [`src/main.rs`](src/main.rs). Each edge is `(from, to, input number)`:

```rust
fn build_graph() -> CalcGraph {
    let mut g = CalcGraph::new();
    let mut node = |name, op, value| g.add_node(Node { name, op, value });

    let a = node("a", Op::Input, Some(10));
    let b = node("b", Op::Input, Some(4));
    let c = node("c", Op::Input, Some(3));
    let d = node("d", Op::Input, Some(7));
    let s1 = node("s1", Op::Add, None);
    let s2 = node("s2", Op::Sub, None);
    let s3 = node("s3", Op::Sub, None);
    let s4 = node("s4", Op::Add, None);
    let s5 = node("s5", Op::Sub, None);
    let out1 = node("out1", Op::Sub, None);
    let out2 = node("out2", Op::Add, None); // more than one output is fine

    let edges = [
        (a, s1, 1), (b, s1, 2),              // s1   = a + b
        (b, s2, 1), (c, s2, 2),              // s2   = b - c
        (s1, s3, 1), (s2, s3, 2),            // s3   = s1 - s2
        (s2, s4, 1), (d, s4, 2), (a, s4, 3), // s4   = s2 + d + a
        (s1, s5, 1), (d, s5, 2),             // s5   = s1 - d
        (s3, out1, 1), (s4, out1, 2),        // out1 = s3 - s4
        (s4, out2, 1), (s5, out2, 2),        // out2 = s4 + s5
    ];
    for (from, to, pos) in edges {
        g.add_edge(from, to, pos);
    }
    g
}
```

> [!IMPORTANT]
> **Each node has exactly one result** (its `= …` value). It is passed unchanged along every edge leaving the node.
>
> **The numbers on the edges are not values.** They give the input order of the receiving node: a `−` node computes input 1 − input 2 − ….

![The computation graph](docs/graph.svg)

Each layer has its own colour (inputs blue, then green, then red, outputs white), and every edge takes the colour of the layer it leaves. The graph has two outputs, `out1` and `out2`.

## Two versions

| | File | Run with |
|---|---|---|
| Plain Rust program | [`src/main.rs`](src/main.rs) | `cargo run` |
| Jupyter notebook | [`petgraph_dag.ipynb`](petgraph_dag.ipynb) | Jupyter with the evcxr Rust kernel |

Both use the same graph. The notebook walks through it one step at a time with explanations, and adds the cycle example and a "play with it" cell.

The drawing code is shared by both and kept out of the way in [`src/viz.rs`](src/viz.rs). It turns any petgraph `DiGraph` into a Graphviz picture; a node type only has to say how it looks by implementing the small `NodeStyle` trait.

## Requirements

- Rust (edition 2024)
- [Graphviz](https://graphviz.org/) (`dot`) to render the pictures
- For the notebook only: Jupyter and [evcxr_jupyter](https://github.com/evcxr/evcxr/blob/main/evcxr_jupyter/README.md)

```sh
cargo install evcxr_jupyter
evcxr_jupyter --install
```

## Running

**Program:**

```sh
cargo run
```

This prints the topological order, the layers and each layer's results as they are computed. It writes `graph_before.svg` (only inputs filled in) and `graph.svg` (everything computed).

**Notebook:**

```sh
jupyter notebook petgraph_dag.ipynb
```

You can also open it in VS Code and choose the **Rust** kernel. The first cell compiles petgraph, which takes a few seconds.

## Publishing

GitHub shows the notebook directly, with its saved outputs. [`.github/workflows/pages.yml`](.github/workflows/pages.yml) also publishes it as a web page on GitHub Pages on every push to `main` or `master`. To turn this on, go to the repository's **Settings → Pages** and set **Source** to **GitHub Actions**.

The workflow only converts the notebook to HTML. It does not re-run the notebook, so after changing a cell, run all cells and save before committing.

**Run it online:** the Binder badge at the top opens the notebook in a live Jupyter session, with nothing to install. The first launch after a new commit takes a few minutes while Binder builds the image. This happens once per commit: Binder caches the image, so later launches start in seconds. Occasionally a launch is slow again because Binder sends it to a server that hasn't cached the image yet. Everything Binder needs is in [`.binder/`](.binder/); see [docs/binder.md](docs/binder.md) for how Binder works.

To preview the page locally:

```sh
jupyter nbconvert --to html petgraph_dag.ipynb --output-dir site --output index
```
