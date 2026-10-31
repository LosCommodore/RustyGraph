//! Drawing helpers: turn a petgraph `DiGraph` into a Graphviz picture.
//!
//! Not what the examples are about, so it lives here and gets imported.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;
use std::fs;
use std::io::{self, Write};
use std::process::{Command, Stdio};

use petgraph::dot::{Config, Dot, RankDir};
use petgraph::graph::{DiGraph, EdgeIndex, EdgeReference, NodeIndex};
use petgraph::visit::EdgeRef;

/// How a node weight appears in the picture.
pub trait NodeStyle {
    /// Text inside the box; `\n` starts a new line.
    fn label(&self) -> String;
    /// Empty nodes are drawn white with a dashed border.
    fn is_empty(&self) -> bool {
        false
    }
    /// If true, the node's incoming edges are labelled with their weight
    /// (e.g. the operand position for a subtraction).
    fn ordered_inputs(&self) -> bool {
        false
    }
}

const COLORS: [&str; 6] = ["#cfe8ff", "#d6f5d6", "#fff2b3", "#ffd9b3", "#f2c6de", "#e0d4f7"];

/// An SVG image. Shown as a picture when it is the last value of a notebook cell.
pub struct Svg(pub String);

impl Svg {
    /// Called by the evcxr Jupyter kernel to display the value.
    pub fn evcxr_display(&self) {
        println!("EVCXR_BEGIN_CONTENT image/svg+xml\n{}\nEVCXR_END_CONTENT", self.0);
    }

    pub fn save(&self, path: &str) -> io::Result<()> {
        fs::write(path, &self.0)
    }
}

/// Draws `g` top-down. With `layer`, each layer gets its own row and colour.
/// Edges listed in `highlight` are drawn in red.
pub fn draw<N: NodeStyle + Debug, E: Debug>(
    g: &DiGraph<N, E>,
    layer: Option<&HashMap<NodeIndex, usize>>,
    highlight: &[EdgeIndex],
) -> Svg {
    render(&to_dot(g, layer, highlight))
}

/// The DOT text behind [`draw`].
pub fn to_dot<N: NodeStyle + Debug, E: Debug>(
    g: &DiGraph<N, E>,
    layer: Option<&HashMap<NodeIndex, usize>>,
    highlight: &[EdgeIndex],
) -> String {
    let node_attrs = |_: &DiGraph<N, E>, (n, node): (NodeIndex, &N)| {
        let mut label = node.label();
        if let Some(layer) = layer {
            label += &format!("\nlayer {}", layer[&n]);
        }
        let (fill, style) = if node.is_empty() {
            ("white", "rounded,filled,dashed")
        } else {
            (COLORS[layer.map_or(0, |l| l[&n]) % COLORS.len()], "rounded,filled")
        };
        let label = label.replace('"', "\\\"").replace('\n', "\\n");
        format!("label=\"{label}\" shape=box style=\"{style}\" fillcolor=\"{fill}\"")
    };
    let edge_attrs = |g: &DiGraph<N, E>, e: EdgeReference<E>| {
        let mut attrs = if g[e.target()].ordered_inputs() {
            format!("label=\"{:?}\"", e.weight())
        } else {
            String::new()
        };
        if highlight.contains(&e.id()) {
            attrs += " color=red fontcolor=red penwidth=2";
        }
        attrs
    };

    let dot = Dot::with_attr_getters(
        g,
        &[Config::NodeNoLabel, Config::EdgeNoLabel, Config::RankDir(RankDir::TB)],
        &edge_attrs,
        &node_attrs,
    );
    let mut out = format!("{dot:?}");

    // Dot can't emit subgraphs, so add one `rank=same` group per layer before
    // the closing brace to make Graphviz draw each layer in one row.
    // Node ids in petgraph's DOT output are the node indices.
    if let Some(layer) = layer {
        let mut rows: BTreeMap<usize, Vec<String>> = BTreeMap::new();
        for n in g.node_indices() {
            rows.entry(layer[&n]).or_default().push(n.index().to_string());
        }
        let ranks: String = rows
            .values()
            .map(|ids| format!("    {{ rank=same; {} }}\n", ids.join("; ")))
            .collect();
        let closing = out.rfind('}').expect("dot output ends with }");
        out.insert_str(closing, &ranks);
    }
    out
}

/// Renders DOT text to SVG with Graphviz's `dot` command.
pub fn render(dot: &str) -> Svg {
    let mut child = Command::new("dot")
        .arg("-Tsvg")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Graphviz `dot` must be installed");
    child.stdin.take().unwrap().write_all(dot.as_bytes()).unwrap();
    let svg = String::from_utf8(child.wait_with_output().unwrap().stdout).unwrap();
    // Drop the XML prolog so the SVG can sit inside HTML.
    Svg(svg[svg.find("<svg").unwrap_or(0)..].to_string())
}
