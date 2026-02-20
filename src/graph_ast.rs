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
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeShape {
    Box,
    Round,
    Diamond,
    Circle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NodeDecl {
    pub id: String,
    pub label: String,
    pub shape: NodeShape,
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
    DottedArrow,
    DottedLink,
    ThickArrow,
    ThickLink,
}
