#[derive(Debug, Clone, PartialEq)]
pub enum Direction {
    TopDown,
    LeftRight,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphDiagram {
    pub direction: Direction,
    pub nodes: Vec<NodeDecl>,
    pub edges: Vec<Edge>,
    pub subgraphs: Vec<Subgraph>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Subgraph {
    pub id: String,
    pub label: String,
    pub node_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NodeDecl {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub edge_type: EdgeType,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EdgeType {
    Arrow,
    OpenLink,
}
