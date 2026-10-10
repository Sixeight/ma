#[derive(Debug, Clone, PartialEq)]
pub enum Direction {
    TopDown,
    LeftRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubgraphIndex(u32);

impl SubgraphIndex {
    pub(crate) fn new(index: usize) -> Self {
        Self(index as u32)
    }

    pub(crate) fn get(self) -> usize {
        self.0 as usize
    }
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
    pub(crate) parent: Option<SubgraphIndex>,
}

impl GraphDiagram {
    pub(crate) fn is_nested(&self) -> bool {
        self.subgraphs.iter().any(|sg| sg.parent.is_some())
    }

    pub(crate) fn parent(&self, index: SubgraphIndex) -> Option<SubgraphIndex> {
        self.subgraphs.get(index.get()).and_then(|sg| sg.parent)
    }

    pub(crate) fn children(&self, index: SubgraphIndex) -> Vec<SubgraphIndex> {
        self.subgraphs
            .iter()
            .enumerate()
            .filter_map(|(i, sg)| (sg.parent == Some(index)).then_some(SubgraphIndex::new(i)))
            .collect()
    }

    pub(crate) fn roots(&self) -> Vec<SubgraphIndex> {
        self.subgraphs
            .iter()
            .enumerate()
            .filter_map(|(i, sg)| sg.parent.is_none().then_some(SubgraphIndex::new(i)))
            .collect()
    }

    pub(crate) fn innermost(&self, id: &str) -> Option<SubgraphIndex> {
        let owners: Vec<SubgraphIndex> = self
            .subgraphs
            .iter()
            .enumerate()
            .filter(|(_, sg)| sg.node_ids.iter().any(|node| node == id))
            .map(|(i, _)| SubgraphIndex::new(i))
            .collect();
        owners.iter().copied().find(|&idx| {
            !self
                .children(idx)
                .iter()
                .any(|child| owners.contains(child))
        })
    }

    pub(crate) fn exclusive_members(&self, index: SubgraphIndex) -> Vec<&str> {
        let Some(sg) = self.subgraphs.get(index.get()) else {
            return Vec::new();
        };
        sg.node_ids
            .iter()
            .filter(|id| self.innermost(id) == Some(index))
            .map(String::as_str)
            .collect()
    }

    pub(crate) fn contains_frame(&self, ancestor: SubgraphIndex, index: SubgraphIndex) -> bool {
        let mut current = Some(index);
        let mut steps = 0;
        while let Some(at) = current {
            if at == ancestor {
                return true;
            }
            if steps > self.subgraphs.len() {
                return false;
            }
            steps += 1;
            current = self.parent(at);
        }
        false
    }
}

pub(crate) fn validate_subgraph_forest(diagram: &GraphDiagram) -> Result<(), String> {
    let node_ids: std::collections::HashSet<&str> =
        diagram.nodes.iter().map(|node| node.id.as_str()).collect();
    let len = diagram.subgraphs.len();

    for (index, sg) in diagram.subgraphs.iter().enumerate() {
        for id in &sg.node_ids {
            if !node_ids.contains(id.as_str()) {
                return Err(format!("subgraph {} lists unknown node {id}", sg.id));
            }
        }
        let Some(parent) = sg.parent else {
            continue;
        };
        let parent_index = parent.get();
        if parent_index >= len || parent_index <= index {
            return Err(format!("subgraph {} parent must be a later frame", sg.id));
        }
        let parent_sg = &diagram.subgraphs[parent_index];
        if sg
            .node_ids
            .iter()
            .any(|id| !parent_sg.node_ids.contains(id))
        {
            return Err(format!(
                "subgraph {} is not contained by {}",
                sg.id, parent_sg.id
            ));
        }
        let mut cursor = Some(parent);
        let mut steps = 0;
        while let Some(at) = cursor {
            if steps > len {
                return Err(format!("subgraph {} parent chain cycles", sg.id));
            }
            steps += 1;
            cursor = diagram.parent(at);
        }
    }

    let mut owners: std::collections::HashMap<&str, Vec<usize>> = std::collections::HashMap::new();
    for (index, sg) in diagram.subgraphs.iter().enumerate() {
        for id in &sg.node_ids {
            owners.entry(id.as_str()).or_default().push(index);
        }
    }
    for (id, frames) in owners {
        for left in 0..frames.len() {
            for right in (left + 1)..frames.len() {
                let a = SubgraphIndex::new(frames[left]);
                let b = SubgraphIndex::new(frames[right]);
                if !diagram.contains_frame(a, b) && !diagram.contains_frame(b, a) {
                    return Err(format!(
                        "node {id} is in subgraphs {} and {} but neither contains the other",
                        diagram.subgraphs[frames[left]].id, diagram.subgraphs[frames[right]].id
                    ));
                }
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeShape {
    Box,
    Round,
    Diamond,
    Circle,
    Stadium,
    Subroutine,
    Cylinder,
    Hexagon,
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
    Invisible,
    Arrow,
    OpenLink,
    DottedArrow,
    DottedLink,
    ThickArrow,
    ThickLink,
}
