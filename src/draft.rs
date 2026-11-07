#![allow(unused)]

use std::{any::Any, cell::OnceCell, ops::IndexMut};

use petgraph::{Directed, stable_graph::StableGraph};

type FlowGraph = StableGraph<DataNode, Edge, Directed>;

#[derive(Debug)]
struct InputData {
    pos: (f32, f32),
    vel: (f32, f32),
}

enum InputData2 {
    Seek,
    Destroy,
}

#[derive(Debug)]
struct DataNode {
    name: String,
    data: OnceCell<Box<dyn Any>>,
}

struct Edge {
    idx: usize,
}

impl DataNode {
    fn new(name: &str) -> Self {
        DataNode {
            name: name.to_owned(),
            data: OnceCell::new(),
        }
    }
}

fn create_empty_graph() -> FlowGraph {
    let mut g = StableGraph::<DataNode, Edge, Directed>::new();
    let idx_a = g.add_node(DataNode::new("A"));
    g
}

fn main() {
    let g_template = create_empty_graph();
    let g = g_template.clone();

    let data1 = InputData {
        pos: (0., 0.),
        vel: (10., 10.),
    };

    g_template
        .index_mut(idx_a)
        .data
        .set(Box::new(data1))
        .expect("data already set ??");

    let a = g_template[idx_a]
        .data
        .get()
        .expect("no data in once ???")
        .downcast_ref::<InputData>()
        .expect("downcast failed");

    println!("{a:?}");
}
