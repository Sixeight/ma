use std::collections::HashMap;

use crate::er_ast::*;

#[derive(Debug, Clone, PartialEq)]
pub struct ErLayout {
    pub nodes: Vec<ErNodeLayout>,
    pub edges: Vec<ErEdgeLayout>,
    pub width: usize,
    pub height: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ErNodeLayout {
    pub name: String,
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub center_y: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ErEdgeLayout {
    pub from: String,
    pub to: String,
    pub label: String,
}

const BOX_HEIGHT: usize = 3;
const MIN_GAP: usize = 6;

pub fn compute(diagram: &ErDiagram) -> Result<ErLayout, String> {
    if diagram.entities.is_empty() {
        return Err("no entities found".to_string());
    }

    let ranks = assign_ranks(diagram);
    let max_rank = *ranks.values().max().unwrap_or(&0);

    let mut ranks_entities: Vec<Vec<&str>> = vec![Vec::new(); max_rank + 1];
    for entity in &diagram.entities {
        let rank = ranks[entity.as_str()];
        ranks_entities[rank].push(entity);
    }

    let mut nodes = Vec::new();
    let mut x = 0;

    for (rank, rank_entities) in ranks_entities.iter().enumerate() {
        let mut y = 0;
        for entity in rank_entities {
            let w = entity.len() + 4;
            nodes.push(ErNodeLayout {
                name: entity.to_string(),
                x,
                y,
                width: w,
                height: BOX_HEIGHT,
                center_y: y + 1,
            });
            y += BOX_HEIGHT + 1;
        }

        if rank < max_rank {
            let rank_max_width = rank_entities.iter().map(|e| e.len() + 4).max().unwrap_or(0);
            let label_gap = diagram
                .relationships
                .iter()
                .filter(|r| {
                    ranks.get(r.from.as_str()) == Some(&rank)
                        && ranks.get(r.to.as_str()) == Some(&(rank + 1))
                })
                .map(|r| r.label.len() + 4)
                .max()
                .unwrap_or(MIN_GAP)
                .max(MIN_GAP);
            x += rank_max_width + label_gap;
        }
    }

    let width = nodes.iter().map(|n| n.x + n.width).max().unwrap_or(0);
    let height = nodes.iter().map(|n| n.y + n.height).max().unwrap_or(0);

    let edges = diagram
        .relationships
        .iter()
        .map(|r| ErEdgeLayout {
            from: r.from.clone(),
            to: r.to.clone(),
            label: r.label.clone(),
        })
        .collect();

    Ok(ErLayout {
        nodes,
        edges,
        width,
        height,
    })
}

fn assign_ranks(diagram: &ErDiagram) -> HashMap<&str, usize> {
    let mut in_edges: HashMap<&str, Vec<&str>> = HashMap::new();
    for entity in &diagram.entities {
        in_edges.entry(entity).or_default();
    }
    for rel in &diagram.relationships {
        in_edges.entry(&rel.to).or_default().push(&rel.from);
    }

    let mut ranks: HashMap<&str, usize> = HashMap::new();
    for entity in &diagram.entities {
        if !ranks.contains_key(entity.as_str()) {
            compute_rank(entity, &in_edges, &mut ranks);
        }
    }
    ranks
}

fn compute_rank<'a>(
    id: &'a str,
    in_edges: &HashMap<&str, Vec<&'a str>>,
    ranks: &mut HashMap<&'a str, usize>,
) -> usize {
    if let Some(&r) = ranks.get(id) {
        return r;
    }

    let predecessors = in_edges.get(id).cloned().unwrap_or_default();
    if predecessors.is_empty() {
        ranks.insert(id, 0);
        return 0;
    }

    let max_pred = predecessors
        .iter()
        .map(|p| compute_rank(p, in_edges, ranks))
        .max()
        .unwrap_or(0);
    let rank = max_pred + 1;
    ranks.insert(id, rank);
    rank
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::er_ast::*;

    #[test]
    fn rank_single_relationship() {
        let diagram = ErDiagram {
            entities: vec!["A".into(), "B".into()],
            relationships: vec![Relationship {
                from: "A".into(),
                to: "B".into(),
                label: "r1".into(),
            }],
        };
        let layout = compute(&diagram).unwrap();
        assert_eq!(layout.nodes.len(), 2);
        let a = &layout.nodes.iter().find(|n| n.name == "A").unwrap();
        let b = &layout.nodes.iter().find(|n| n.name == "B").unwrap();
        assert!(a.x < b.x, "A should be left of B");
    }

    #[test]
    fn rank_chain() {
        let diagram = ErDiagram {
            entities: vec!["A".into(), "B".into(), "C".into()],
            relationships: vec![
                Relationship { from: "A".into(), to: "B".into(), label: "r1".into() },
                Relationship { from: "B".into(), to: "C".into(), label: "r2".into() },
            ],
        };
        let layout = compute(&diagram).unwrap();
        let a = layout.nodes.iter().find(|n| n.name == "A").unwrap();
        let b = layout.nodes.iter().find(|n| n.name == "B").unwrap();
        let c = layout.nodes.iter().find(|n| n.name == "C").unwrap();
        assert!(a.x < b.x);
        assert!(b.x < c.x);
    }

    #[test]
    fn layout_label_gap() {
        let diagram = ErDiagram {
            entities: vec!["A".into(), "B".into()],
            relationships: vec![Relationship {
                from: "A".into(),
                to: "B".into(),
                label: "long label here".into(),
            }],
        };
        let layout = compute(&diagram).unwrap();
        let a = layout.nodes.iter().find(|n| n.name == "A").unwrap();
        let b = layout.nodes.iter().find(|n| n.name == "B").unwrap();
        let gap = b.x - (a.x + a.width);
        assert!(
            gap >= "long label here".len() + 4,
            "gap ({gap}) should fit label + connectors"
        );
    }
}
