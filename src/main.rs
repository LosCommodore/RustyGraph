//! Exploring petgraph: build a DAG, verify it is acyclic, sort it into layers,
//! evaluate a tiny "computation graph" over it and visualize the result.

use std::collections::HashMap;
use std::fs;
use std::process::Command;

use petgraph::algo::{is_cyclic_directed, toposort};
use petgraph::dot::{Config, Dot, RankDir};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;

/// What a node computes from its inputs.
#[derive(Debug, Clone, Copy)]
enum Op {
    /// Source node holding a constant.
    Input(i64),
    /// Sum of all inputs.
    Add,
    /// First input minus all remaining inputs.
    Sub,
}

#[derive(Debug, Clone)]
struct Node {
    name: &'static str,
    op: Op,
}

/// Edge weight = operand position. Only matters for non-commutative ops (Sub).
type Operand = usize;

type CalcGraph = DiGraph<Node, Operand>;

fn build_graph() -> CalcGraph {
    let mut g = CalcGraph::new();
    let mut node = |name, op| g.add_node(Node { name, op });

    let a = node("a", Op::Input(10));
    let b = node("b", Op::Input(4));
    let c = node("c", Op::Input(3));
    let d = node("d", Op::Input(7));
    let s1 = node("s1", Op::Add);
    let s2 = node("s2", Op::Sub);
    let s3 = node("s3", Op::Sub);
    let s4 = node("s4", Op::Add);
    let out = node("out", Op::Sub);

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

/// Evaluate every node in topological order, so all inputs are ready when needed.
fn evaluate(g: &CalcGraph, topo: &[NodeIndex]) -> HashMap<NodeIndex, i64> {
    let mut value = HashMap::new();
    for &n in topo {
        // Collect inputs sorted by operand position.
        let mut inputs: Vec<(Operand, i64)> = g
            .edges_directed(n, Direction::Incoming)
            .map(|e| (*e.weight(), value[&e.source()]))
            .collect();
        inputs.sort_by_key(|&(pos, _)| pos);
        let mut vals = inputs.into_iter().map(|(_, v)| v);

        let v = match g[n].op {
            Op::Input(x) => x,
            Op::Add => vals.sum(),
            Op::Sub => {
                let first = vals.next().unwrap_or(0);
                first - vals.sum::<i64>()
            }
        };
        value.insert(n, v);
    }
    value
}

fn op_symbol(op: Op) -> String {
    match op {
        Op::Input(x) => format!("input {x}"),
        Op::Add => "+".into(),
        Op::Sub => "−".into(),
    }
}

fn to_dot(
    g: &CalcGraph,
    layer: &HashMap<NodeIndex, usize>,
    value: &HashMap<NodeIndex, i64>,
) -> String {
    const COLORS: [&str; 6] = ["#cfe8ff", "#d6f5d6", "#fff2b3", "#ffd9b3", "#f2c6de", "#e0d4f7"];

    let node_attrs = |_: &CalcGraph, (n, node): (NodeIndex, &Node)| {
        format!(
            "label=\"{}  [{}]\\nlayer {}\\n= {}\" shape=box style=\"rounded,filled\" fillcolor=\"{}\"",
            node.name,
            op_symbol(node.op),
            layer[&n],
            value[&n],
            COLORS[layer[&n] % COLORS.len()],
        )
    };
    let edge_attrs = |g: &CalcGraph, e: petgraph::graph::EdgeReference<Operand>| {
        // Show operand position only where order matters.
        match g[e.target()].op {
            Op::Sub => format!("label=\"{}\"", e.weight()),
            _ => String::new(),
        }
    };

    let dot = Dot::with_attr_getters(
        g,
        &[Config::NodeNoLabel, Config::EdgeNoLabel, Config::RankDir(RankDir::TB)],
        &edge_attrs,
        &node_attrs,
    );
    // petgraph's Dot can't emit subgraphs, so append one `rank=same` group per
    // layer before the closing brace to make Graphviz draw each layer in one row.
    // Node ids in petgraph's DOT output are the node indices.
    let mut out = format!("{dot:?}");
    let closing = out.rfind('}').expect("dot output ends with }");
    let depth = layer.values().max().copied().unwrap_or(0);
    let mut ranks = String::new();
    for l in 0..=depth {
        let ids: Vec<_> = g
            .node_indices()
            .filter(|n| layer[n] == l)
            .map(|n| n.index().to_string())
            .collect();
        ranks += &format!("    {{ rank=same; {} }}\n", ids.join("; "));
    }
    out.insert_str(closing, &ranks);
    out
}

fn main() {
    let g = build_graph();
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
    let depth = layer.values().max().copied().unwrap_or(0);
    println!("\nLayers:");
    for l in 0..=depth {
        let mut in_layer: Vec<_> = topo.iter().filter(|n| layer[n] == l).map(|&n| g[n].name).collect();
        in_layer.sort();
        println!("  layer {l}: {}", in_layer.join(", "));
    }

    // --- 3. Compute ---------------------------------------------------------
    let value = evaluate(&g, &topo);
    println!("\nResults (in evaluation order):");
    for &n in &topo {
        let inputs: Vec<_> = {
            let mut es: Vec<_> = g.edges_directed(n, Direction::Incoming).collect();
            es.sort_by_key(|e| *e.weight());
            es.iter().map(|e| g[e.source()].name).collect()
        };
        let expr = match g[n].op {
            Op::Input(_) => "const".to_string(),
            Op::Add => inputs.join(" + "),
            Op::Sub => inputs.join(" - "),
        };
        println!("  {:>4} = {:<14} = {}", g[n].name, expr, value[&n]);
    }

    // --- 4. Visualize -------------------------------------------------------
    let dot = to_dot(&g, &layer, &value);
    fs::write("graph.dot", &dot).expect("write graph.dot");
    println!("\nWrote graph.dot");
    match Command::new("dot").args(["-Tsvg", "graph.dot", "-o", "graph.svg"]).status() {
        Ok(s) if s.success() => println!("Rendered graph.svg (Graphviz)"),
        _ => println!("Graphviz `dot` not available; render with: dot -Tsvg graph.dot -o graph.svg"),
    }
}
