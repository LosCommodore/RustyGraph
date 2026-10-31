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
    /// Empty nodes are drawn with a dashed border (white if there are no layers).
    fn is_empty(&self) -> bool {
        false
    }
    /// Label for the incoming edge with this weight (e.g. its input number).
    /// `None` draws the edge without a label.
    fn input_label(&self, _input: usize) -> Option<String> {
        None
    }
    /// Input nodes are drawn together in the top row, even without layers.
    fn is_input(&self) -> bool {
        false
    }
}

/// Per layer: node fill and the colour of the edges leaving the layer.
/// The last layer (the outputs) is always drawn black on white.
const LAYER_COLORS: [(&str, &str); 3] = [
    ("#cfe8ff", "black"),   // inputs: blue
    ("#d6f5d6", "#2e7d32"), // green
    ("#ffd6d6", "#c62828"), // red
];

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

/// Draws `g` top-down. With `layer`, each layer gets its own row and colour,
/// and edges take the colour of the layer they leave. Edges listed in
/// `highlight` are drawn thick, and in red when there are no layers.
pub fn draw<N: NodeStyle + Debug, E: Debug + Copy + Into<usize>>(
    g: &DiGraph<N, E>,
    layer: Option<&HashMap<NodeIndex, usize>>,
    highlight: &[EdgeIndex],
) -> Svg {
    render(&to_dot(g, layer, highlight))
}

/// The DOT text behind [`draw`].
pub fn to_dot<N: NodeStyle + Debug, E: Debug + Copy + Into<usize>>(
    g: &DiGraph<N, E>,
    layer: Option<&HashMap<NodeIndex, usize>>,
    highlight: &[EdgeIndex],
) -> String {
    let last = layer.and_then(|l| l.values().max().copied());
    let colors = |n: NodeIndex| match layer.map(|l| l[&n]) {
        None => LAYER_COLORS[0],
        Some(l) if Some(l) == last => ("white", "black"),
        Some(l) => LAYER_COLORS[l % LAYER_COLORS.len()],
    };
    let node_attrs = |_: &DiGraph<N, E>, (n, node): (NodeIndex, &N)| {
        let mut label = node.label();
        if let Some(layer) = layer {
            label += &format!("\nlayer {}", layer[&n]);
        }
        let fill = if layer.is_none() && node.is_empty() { "white" } else { colors(n).0 };
        let style = if node.is_empty() { "rounded,filled,dashed" } else { "rounded,filled" };
        let label = label.replace('"', "\\\"").replace('\n', "\\n");
        format!("label=\"{label}\" shape=box style=\"{style}\" fillcolor=\"{fill}\"")
    };
    let edge_attrs = |g: &DiGraph<N, E>, e: EdgeReference<E>| {
        let mut attrs = match g[e.target()].input_label((*e.weight()).into()) {
            Some(label) => format!("label=\"{label}\" fontsize=18"),
            None => String::new(),
        };
        let color = match (layer, highlight.contains(&e.id())) {
            (None, true) => "red",
            (None, false) => "black",
            (Some(_), _) => colors(e.source()).1,
        };
        attrs += &format!(" color=\"{color}\" fontcolor=\"{color}\"");
        if highlight.contains(&e.id()) {
            attrs += " penwidth=3";
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

    // Dot can't emit subgraphs, so add rank groups before the closing brace:
    // one row per layer if layers are given, otherwise one top row for the
    // inputs. Node ids in petgraph's DOT output are the node indices.
    let ranks: String = match layer {
        Some(layer) => {
            let mut rows: BTreeMap<usize, Vec<String>> = BTreeMap::new();
            for n in g.node_indices() {
                rows.entry(layer[&n]).or_default().push(n.index().to_string());
            }
            rows.values()
                .map(|ids| format!("    {{ rank=same; {} }}\n", ids.join("; ")))
                .collect()
        }
        None => {
            let inputs: Vec<_> = g
                .node_indices()
                .filter(|&n| g[n].is_input())
                .map(|n| n.index().to_string())
                .collect();
            if inputs.is_empty() {
                String::new()
            } else {
                format!("    {{ rank=min; {} }}\n", inputs.join("; "))
            }
        }
    };
    let closing = out.rfind('}').expect("dot output ends with }");
    out.insert_str(closing, &ranks);
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
