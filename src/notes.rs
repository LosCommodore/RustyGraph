use petgraph::stable_graph::{NodeIndex, StableGraph};
use petgraph::{Directed, Direction};
use std::any::Any;
use std::cell::OnceCell;

// 1. Das Knoten-Gewicht: Hält die OnceCell für beliebige Typen (dyn Any)
pub struct DataNode {
    pub storage: OnceCell<Box<dyn Any>>,
}

// 2. Das Kanten-Gewicht: Hält nur die Nummerierung für das spätere Zielfeld
pub struct NumberedEdge {
    pub number: usize,
}

// Beispiel-Datentypen für die Knoten
#[derive(Debug, Clone)]
pub struct Flight {
    pub duration: u32,
}
#[derive(Debug, Clone)]
pub struct Road {
    pub congestion: f32,
}

// 3. Das finale, statisch typisierte Ziel-Struct
#[derive(Debug)]
pub struct CollectedData {
    pub field_0: String, // Erwartet aus Kante 0
    pub field_1: Road,   // Erwartet aus Kante 1
    pub field_2: Flight, // Erwartet aus Kante 2
}

fn main() {
    // Graphen initialisieren (StableGraph, gerichtet)
    let mut graph = StableGraph::<DataNode, NumberedEdge, Directed>::new();

    // Zentral-Knoten und leere Daten-Knoten erstellen
    let center = graph.add_node(DataNode {
        storage: OnceCell::new(),
    });

    let n0 = graph.add_node(DataNode {
        storage: OnceCell::new(),
    });
    let n1 = graph.add_node(DataNode {
        storage: OnceCell::new(),
    });
    let n2 = graph.add_node(DataNode {
        storage: OnceCell::new(),
    });

    // Kanten mit ihrer Nummerierung anlegen (Reihenfolge beim Einfügen ist egal)
    graph.add_edge(center, n2, NumberedEdge { number: 2 });
    graph.add_edge(center, n0, NumberedEdge { number: 0 });
    graph.add_edge(center, n1, NumberedEdge { number: 1 });

    // --- SCHREIBEN (Genau einmal) ---
    // Wir holen uns die Knoten und befüllen die OnceCell
    graph
        .node_weight(n0)
        .unwrap()
        .storage
        .set(Box::new("München".to_string()))
        .unwrap();
    graph
        .node_weight(n1)
        .unwrap()
        .storage
        .set(Box::new(Road { congestion: 1.5 }))
        .unwrap();
    graph
        .node_weight(n2)
        .unwrap()
        .storage
        .set(Box::new(Flight { duration: 450 }))
        .unwrap();

    // Ein zweiter Schreibversuch würde hier hart fehlschlagen (Invariante geschützt!):
    // graph.node_weight(n0).unwrap().storage.set(Box::new("Berlin".to_string())).unwrap_err();

    // --- LESEN, SORTIEREN & DOWNCAST ---
    // 1. Nachbarn und Kanten-Nummern als Paare einsammeln
    let mut pairs: Vec<(usize, &DataNode)> = graph
        .edges_directed(center, Direction::Outgoing)
        .map(|edge| {
            let num = edge.weight().number;
            let target_node = graph.node_weight(edge.target()).unwrap();
            (num, target_node)
        })
        .collect();

    // 2. Nach Kanten-Nummer sortieren (O(1) Overhead bei kleinen Graphen)
    pairs.sort_by_key(|(num, _)| *num);

    // 3. Typsicher auslesen und in das feste Struct gießen
    let final_struct = CollectedData {
        field_0: pairs[0]
            .1
            .storage
            .get()
            .and_then(|b| b.downcast_ref::<String>())
            .expect("Fehler bei Feld 0: Leer oder falscher Typ!")
            .clone(),

        field_1: pairs[1]
            .1
            .storage
            .get()
            .and_then(|b| b.downcast_ref::<Road>())
            .expect("Fehler bei Feld 1: Leer oder falscher Typ!")
            .clone(),

        field_2: pairs[2]
            .1
            .storage
            .get()
            .and_then(|b| b.downcast_ref::<Flight>())
            .expect("Fehler bei Feld 2: Leer oder falscher Typ!")
            .clone(),
    };

    println!("Erfolgreich extrahiert:\n{:#?}", final_struct);
}
