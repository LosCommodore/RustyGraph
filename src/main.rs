//! Exploring petgraph: build a DAG whose input layer holds values, verify it is
//! acyclic, sort it into layers, then fill in the other nodes layer by layer
//! and visualize the result.

use std::collections::HashMap;

use petgraph::algo::{is_cyclic_directed, toposort};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use rusty_graph::viz::{self, NodeStyle};

/// What a node computes from its inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Op {
    /// Source node; its value is given, not computed.
    Input,
    /// Sum of all inputs.
    Add,
    /// First input minus all remaining inputs.
    Sub,
}

#[derive(Debug, Clone)]
struct Node {
    name: &'static str,
    op: Op,
    /// Given for inputs; `None` for every other node until it is computed.
    value: Option<i64>,
}

/// Edge weight = operand position. Only matters for non-commutative ops (Sub).
type Operand = usize;

type CalcGraph = DiGraph<Node, Operand>;

/// Builds the graph with only the input layer filled in.
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
    let out = node("out", Op::Sub, None);

    let edges = [
        (a, s1, 0), (b, s1, 1),              // s1  = a + b
        (b, s2, 0), (c, s2, 1),              // s2  = b - c
        (s1, s3, 0), (s2, s3, 1),            // s3  = s1 - s2
        (s2, s4, 0), (d, s4, 1), (a, s4, 2), // s4  = s2 + d + a
        (s3, out, 0), (s4, out, 1),          // out = s3 - s4
    ];
    for (from, to, pos) in edges {
        g.add_edge(from, to, pos);
    }
    g
}

/// Longest-path layering: sources are layer 0, every other node sits one layer
/// below its deepest predecessor. Requires a topological order.
fn layers(g: &CalcGraph, topo: &[NodeIndex]) -> HashMap<NodeIndex, usize> {
    let mut layer = HashMap::new();
    for &n in topo {
        let l = g
            .neighbors_directed(n, Direction::Incoming)
            .map(|p| layer[&p] + 1)
            .max()
            .unwrap_or(0);
        layer.insert(n, l);
    }
    layer
}

/// Groups nodes by layer: `result[l]` holds the nodes of layer `l`.
fn group_by_layer(g: &CalcGraph, layer: &HashMap<NodeIndex, usize>) -> Vec<Vec<NodeIndex>> {
    let depth = layer.values().max().map_or(0, |d| d + 1);
    let mut groups = vec![Vec::new(); depth];
    for n in g.node_indices() {
        groups[layer[&n]].push(n);
    }
    groups
}

/// Incoming edges of `n`, sorted by operand position.
fn operands(g: &CalcGraph, n: NodeIndex) -> Vec<petgraph::graph::EdgeReference<'_, Operand>> {
    let mut es: Vec<_> = g.edges_directed(n, Direction::Incoming).collect();
    es.sort_by_key(|e| *e.weight());
    es
}

/// Computes a node from the values of its inputs, which must already be filled in.
fn compute_node(g: &CalcGraph, n: NodeIndex) -> i64 {
    let mut vals = operands(g, n)
        .into_iter()
        .map(|e| g[e.source()].value.expect("inputs are computed in an earlier layer"));
    match g[n].op {
        Op::Input => g[n].value.expect("input nodes have a value"),
        Op::Add => vals.sum(),
        Op::Sub => {
            let first = vals.next().unwrap_or(0);
            first - vals.sum::<i64>()
        }
    }
}

/// Fills in every node of one layer. All their inputs sit in earlier layers.
fn compute_layer(g: &mut CalcGraph, nodes: &[NodeIndex]) {
    for &n in nodes {
        if g[n].op != Op::Input {
            g[n].value = Some(compute_node(g, n));
        }
    }
}

fn fmt_value(v: Option<i64>) -> String {
    v.map_or("?".into(), |v| v.to_string())
}

/// The node's formula, e.g. `s1 - s2`.
fn expr(g: &CalcGraph, n: NodeIndex) -> String {
    let inputs: Vec<_> = operands(g, n).iter().map(|e| g[e.source()].name).collect();
    match g[n].op {
        Op::Input => "input".into(),
        Op::Add => inputs.join(" + "),
        Op::Sub => inputs.join(" - "),
    }
}

fn op_symbol(op: Op) -> &'static str {
    match op {
        Op::Input => "input",
        Op::Add => "+",
        Op::Sub => "−",
    }
}

/// How a node is drawn (see `viz`).
impl NodeStyle for Node {
    fn label(&self) -> String {
        format!("{}  [{}]\n= {}", self.name, op_symbol(self.op), fmt_value(self.value))
    }
    fn is_empty(&self) -> bool {
        self.value.is_none()
    }
    fn ordered_inputs(&self) -> bool {
        self.op == Op::Sub
    }
    fn is_input(&self) -> bool {
        self.op == Op::Input
    }
}

fn save_picture(g: &CalcGraph, layer: &HashMap<NodeIndex, usize>, path: &str) {
    viz::draw(g, Some(layer), &[]).save(path).expect("write svg");
    println!("Wrote {path}");
}

fn main() {
    let mut g = build_graph();
    println!("Graph: {} nodes, {} edges", g.node_count(), g.edge_count());

    // --- 1. Is it really a DAG? -------------------------------------------
    println!("is_cyclic_directed: {}", is_cyclic_directed(&g));
    let topo = match toposort(&g, None) {
        Ok(order) => order,
        Err(cycle) => panic!("not a DAG, cycle at node {:?}", g[cycle.node_id()].name),
    };
    let names: Vec<_> = topo.iter().map(|&n| g[n].name).collect();
    println!("topological order: {names:?}");

    // Counter-example: one back edge turns it into a cyclic graph.
    let mut cyclic = g.clone();
    let out = NodeIndex::new(g.node_count() - 1);
    cyclic.add_edge(out, NodeIndex::new(0), 0); // out -> a
    match toposort(&cyclic, None) {
        Ok(_) => println!("unexpected: still acyclic"),
        Err(c) => println!(
            "with extra edge out->a: is_cyclic_directed = {}, toposort error at {:?}",
            is_cyclic_directed(&cyclic),
            cyclic[c.node_id()].name
        ),
    }

    // --- 2. Layers ----------------------------------------------------------
    let layer = layers(&g, &topo);
    let groups = group_by_layer(&g, &layer);
    println!("\nLayers:");
    for (l, nodes) in groups.iter().enumerate() {
        let names: Vec<_> = nodes.iter().map(|&n| g[n].name).collect();
        println!("  layer {l}: {}", names.join(", "));
    }

    // --- 3. Compute layer by layer -------------------------------------------
    println!("\nBefore computing:");
    for (l, nodes) in groups.iter().enumerate() {
        let vals: Vec<_> = nodes.iter().map(|&n| format!("{} = {}", g[n].name, fmt_value(g[n].value))).collect();
        println!("  layer {l}: {}", vals.join(", "));
    }
    save_picture(&g, &layer, "graph_before.svg");

    println!("\nComputing:");
    for (l, nodes) in groups.iter().enumerate().skip(1) {
        compute_layer(&mut g, nodes);
        for &n in nodes {
            println!("  layer {l}: {:>4} = {:<14} = {}", g[n].name, expr(&g, n), fmt_value(g[n].value));
        }
    }

    // --- 4. Visualize -------------------------------------------------------
    println!();
    save_picture(&g, &layer, "graph.svg");
}
