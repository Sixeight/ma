use std::collections::{HashMap, HashSet};

use crate::display_width::{display_width, multiline_width};
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
    pub alias: Option<String>,
    pub attributes: Vec<EntityAttribute>,
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
    pub left_card: Cardinality,
    pub right_card: Cardinality,
    pub line_style: RelationshipLineStyle,
    pub label: String,
}

const BOX_HEIGHT: usize = 3;
const MIN_GAP: usize = 6;

fn entity_width(entity: &Entity) -> usize {
    let attr_width = entity
        .attributes
        .iter()
        .map(|attribute| display_width(&attribute.display_text()))
        .max()
        .unwrap_or(0);
    display_width(entity.alias.as_deref().unwrap_or(&entity.name)).max(attr_width) + 4
}

pub fn compute(diagram: &ErDiagram) -> Result<ErLayout, String> {
    compute_with_gap(diagram, MIN_GAP)
}

pub fn compute_with_max_width(diagram: &ErDiagram, max_width: usize) -> Result<ErLayout, String> {
    let layout = compute(diagram)?;
    if layout.width <= max_width {
        return Ok(layout);
    }

    for gap in (1..MIN_GAP).rev() {
        let layout = compute_with_gap(diagram, gap)?;
        if layout.width <= max_width {
            return Ok(layout);
        }
    }

    Err(format!("ER diagram too wide for {max_width} columns"))
}

fn group_entities_by_target<'a>(
    entities: &[&'a Entity],
    diagram: &ErDiagram,
    ranks: &HashMap<&str, usize>,
    current_rank: usize,
) -> Vec<&'a Entity> {
    let next_rank = current_rank + 1;
    let mut groups: HashMap<&str, Vec<&'a Entity>> = HashMap::new();
    
    for &entity in entities {
        let target = diagram
            .relationships
            .iter()
            .find(|r| r.from == entity.name && ranks.get(r.to.as_str()) == Some(&next_rank))
            .map(|r| r.to.as_str())
            .unwrap_or("");
        groups.entry(target).or_default().push(entity);
    }
    
    let mut result = Vec::new();
    let mut seen = std::collections::HashSet::new();
    
    for &entity in entities {
        if seen.contains(&entity.name) {
            continue;
        }
        let target = diagram
            .relationships
            .iter()
            .find(|r| r.from == entity.name && ranks.get(r.to.as_str()) == Some(&next_rank))
            .map(|r| r.to.as_str())
            .unwrap_or("");
        
        for &e in &groups[target] {
            if seen.insert(&e.name) {
                result.push(e);
            }
        }
    }
    result
}

fn center_align_targets(
    nodes: &mut [ErNodeLayout],
    diagram: &ErDiagram,
    ranks: &HashMap<&str, usize>,
) {
    let node_map: HashMap<String, usize> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.name.clone(), i))
        .collect();
    
    let mut adjustments: Vec<(usize, usize)> = Vec::new();
    
    for target_name in nodes.iter().map(|n| n.name.clone()).collect::<Vec<_>>() {
        let sources: Vec<&str> = diagram
            .relationships
            .iter()
            .filter(|r| r.to == target_name)
            .map(|r| r.from.as_str())
            .collect();
        
        if sources.is_empty() {
            continue;
        }
        
        let target_rank = match ranks.get(target_name.as_str()) {
            Some(&r) => r,
            None => continue,
        };
        
        let source_centers: Vec<usize> = sources
            .iter()
            .filter_map(|&src| {
                let src_rank = ranks.get(src)?;
                if *src_rank + 1 == target_rank {
                    let idx = node_map.get(src)?;
                    Some(nodes[*idx].center_y)
                } else {
                    None
                }
            })
            .collect();
        
        if source_centers.is_empty() {
            continue;
        }
        
        let min_cy = *source_centers.iter().min().unwrap();
        let max_cy = *source_centers.iter().max().unwrap();
        let avg_cy = (min_cy + max_cy) / 2;
        
        let target_idx = node_map[&target_name];
        let target_height = nodes[target_idx].height;
        let new_y = avg_cy.saturating_sub(target_height / 2);
        adjustments.push((target_idx, new_y));
    }
    
    for (idx, new_y) in adjustments {
        nodes[idx].y = new_y;
        nodes[idx].center_y = new_y + nodes[idx].height / 2;
    }
}

fn compute_with_gap(diagram: &ErDiagram, min_gap: usize) -> Result<ErLayout, String> {
    if diagram.entities.is_empty() {
        return Err("no entities found".to_string());
    }

    let ranks = assign_ranks(diagram);
    let max_rank = *ranks.values().max().unwrap_or(&0);

    let mut ranks_entities: Vec<Vec<&Entity>> = vec![Vec::new(); max_rank + 1];
    for entity in &diagram.entities {
        let rank = ranks[entity.name.as_str()];
        ranks_entities[rank].push(entity);
    }

    let mut nodes = Vec::new();
    let mut x = 0;

    for (rank, rank_entities) in ranks_entities.iter().enumerate() {
        let grouped_entities = group_entities_by_target(rank_entities, diagram, &ranks, rank);
        
        let mut y = 0;
        for entity in grouped_entities {
            let w = entity_width(entity);
            let h = if entity.attributes.is_empty() {
                BOX_HEIGHT
            } else {
                BOX_HEIGHT + 1 + entity.attributes.len()
            };
            nodes.push(ErNodeLayout {
                name: entity.name.to_string(),
                alias: entity.alias.clone(),
                attributes: entity.attributes.clone(),
                x,
                y,
                width: w,
                height: h,
                center_y: y + h / 2,
            });
            y += h + 1;
        }

        if rank < max_rank {
            let rank_max_width = rank_entities
                .iter()
                .map(|entity| entity_width(entity))
                .max()
                .unwrap_or(0);
            let label_gap = diagram
                .relationships
                .iter()
                .filter(|r| {
                    ranks.get(r.from.as_str()) == Some(&rank)
                        && ranks.get(r.to.as_str()) == Some(&(rank + 1))
                })
                .map(|r| multiline_width(&r.label) + 8)
                .max()
                .unwrap_or(min_gap)
                .max(min_gap);
            x += rank_max_width + label_gap;
        }
    }

    center_align_targets(&mut nodes, diagram, &ranks);

    let width = nodes.iter().map(|n| n.x + n.width).max().unwrap_or(0);
    let height = nodes.iter().map(|n| n.y + n.height).max().unwrap_or(0);

    let edges = diagram
        .relationships
        .iter()
        .map(|r| ErEdgeLayout {
            from: r.from.clone(),
            to: r.to.clone(),
            left_card: r.left_card,
            right_card: r.right_card,
            line_style: r.line_style,
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
        in_edges.entry(&entity.name).or_default();
    }
    for rel in &diagram.relationships {
        in_edges.entry(&rel.to).or_default().push(&rel.from);
    }

    let mut ranks: HashMap<&str, usize> = HashMap::new();
    let mut visiting: HashSet<&str> = HashSet::new();
    for entity in &diagram.entities {
        if !ranks.contains_key(entity.name.as_str()) {
            compute_rank(&entity.name, &in_edges, &mut ranks, &mut visiting);
        }
    }
    ranks
}

fn compute_rank<'a>(
    id: &'a str,
    in_edges: &HashMap<&str, Vec<&'a str>>,
    ranks: &mut HashMap<&'a str, usize>,
    visiting: &mut HashSet<&'a str>,
) -> usize {
    if let Some(&r) = ranks.get(id) {
        return r;
    }

    if !visiting.insert(id) {
        return 0;
    }

    let predecessors = in_edges.get(id).cloned().unwrap_or_default();
    if predecessors.is_empty() {
        visiting.remove(id);
        ranks.insert(id, 0);
        return 0;
    }

    let max_pred = predecessors
        .iter()
        .map(|p| compute_rank(p, in_edges, ranks, visiting))
        .max()
        .unwrap_or(0);
    let rank = max_pred + 1;
    visiting.remove(id);
    ranks.insert(id, rank);
    rank
}


#[cfg(test)]
mod tests {
    use super::*;

    fn entity(name: &str) -> Entity {
        Entity {
            name: name.to_string(),
            alias: None,
            attributes: Vec::new(),
        }
    }

    fn rel(from: &str, to: &str) -> Relationship {
        Relationship {
            from: from.into(),
            to: to.into(),
            left_card: Cardinality::ExactlyOne,
            right_card: Cardinality::ZeroOrMany,
            line_style: RelationshipLineStyle::Identifying,
            label: String::new(),
        }
    }

    #[test]
    fn rank_single_relationship() {
        let diagram = ErDiagram {
            entities: vec![entity("A"), entity("B")],
            relationships: vec![Relationship {
                from: "A".into(),
                to: "B".into(),
                left_card: Cardinality::ExactlyOne,
                right_card: Cardinality::ExactlyOne,
                line_style: RelationshipLineStyle::Identifying,
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
            entities: vec![entity("A"), entity("B"), entity("C")],
            relationships: vec![
                Relationship {
                    from: "A".into(),
                    to: "B".into(),
                    left_card: Cardinality::ExactlyOne,
                    right_card: Cardinality::ExactlyOne,
                    line_style: RelationshipLineStyle::Identifying,
                    label: "r1".into(),
                },
                Relationship {
                    from: "B".into(),
                    to: "C".into(),
                    left_card: Cardinality::ExactlyOne,
                    right_card: Cardinality::ExactlyOne,
                    line_style: RelationshipLineStyle::Identifying,
                    label: "r2".into(),
                },
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
            entities: vec![entity("A"), entity("B")],
            relationships: vec![Relationship {
                from: "A".into(),
                to: "B".into(),
                left_card: Cardinality::ExactlyOne,
                right_card: Cardinality::ExactlyOne,
                line_style: RelationshipLineStyle::Identifying,
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

    #[test]
    fn entities_targeting_same_node_grouped() {
        let diagram = ErDiagram {
            entities: vec![
                entity("A"),
                entity("UNRELATED"),
                entity("B"),
                entity("TARGET"),
                entity("OTHER"),
            ],
            relationships: vec![
                rel("A", "TARGET"),
                rel("B", "TARGET"),
                rel("UNRELATED", "OTHER"),
            ],
        };
        let layout = compute(&diagram).unwrap();
        let indices: Vec<_> = ["A", "B", "UNRELATED"]
            .iter()
            .map(|name| layout.nodes.iter().position(|n| &n.name == name).unwrap())
            .collect();
        assert!(
            indices[0].abs_diff(indices[1]) == 1,
            "A and B (both→TARGET) should be consecutive, got indices {:?}",
            indices
        );
    }

    #[test]
    fn edges_connect_to_target_boxes_in_render() {
        let diagram = ErDiagram {
            entities: vec![entity("A"), entity("B"), entity("TARGET")],
            relationships: vec![rel("A", "TARGET"), rel("B", "TARGET")],
        };
        let layout = compute(&diagram).unwrap();
        let rendered = crate::er_renderer::render(&layout);
        
        let mut edges_connect_to_target = false;
        
        for line in rendered.lines() {
            if line.contains("TARGET") {
                if line.contains("o{│") || line.contains("|{│") {
                    edges_connect_to_target = true;
                    break;
                }
            }
        }
        
        assert!(
            edges_connect_to_target,
            "No edge connects to TARGET box. Render:\n{}",
            rendered
        );
    }

    #[test]
    fn cardinality_markers_preserved_per_relationship() {
        let diagram = ErDiagram {
            entities: vec![entity("ORDER"), entity("PRODUCT"), entity("LINE_ITEM")],
            relationships: vec![
                Relationship {
                    from: "ORDER".into(),
                    to: "LINE_ITEM".into(),
                    left_card: Cardinality::ExactlyOne,
                    right_card: Cardinality::OneOrMany,
                    line_style: RelationshipLineStyle::Identifying,
                    label: "contains".into(),
                },
                Relationship {
                    from: "PRODUCT".into(),
                    to: "LINE_ITEM".into(),
                    left_card: Cardinality::ExactlyOne,
                    right_card: Cardinality::ZeroOrMany,
                    line_style: RelationshipLineStyle::Identifying,
                    label: "ordered in".into(),
                },
            ],
        };
        let layout = compute(&diagram).unwrap();
        let rendered = crate::er_renderer::render(&layout);
        
        let has_one_or_many = rendered.contains("|{");
        let has_zero_or_many = rendered.contains("o{");
        
        assert!(
            has_one_or_many,
            "ORDER→LINE_ITEM (one-or-many |{{) marker not found in render:\n{}",
            rendered
        );
        assert!(
            has_zero_or_many,
            "PRODUCT→LINE_ITEM (zero-or-many o{{) marker not found in render:\n{}",
            rendered
        );
        
        let one_many_count = rendered.matches("|{").count();
        let zero_many_count = rendered.matches("o{").count();
        assert_eq!(
            one_many_count, 1,
            "Expected exactly 1 |{{ marker, found {}",
            one_many_count
        );
        assert_eq!(
            zero_many_count, 1,
            "Expected exactly 1 o{{ marker, found {}",
            zero_many_count
        );
    }

    #[test]
    fn label_not_adjacent_to_left_marker() {
        let diagram = ErDiagram {
            entities: vec![entity("A"), entity("B")],
            relationships: vec![Relationship {
                from: "A".into(),
                to: "B".into(),
                left_card: Cardinality::ExactlyOne,
                right_card: Cardinality::ZeroOrMany,
                line_style: RelationshipLineStyle::Identifying,
                label: "label".into(),
            }],
        };
        let layout = compute(&diagram).unwrap();
        let rendered = crate::er_renderer::render(&layout);
        
        assert!(
            !rendered.contains("||label"),
            "Label should not be adjacent to || marker. Render:\n{}",
            rendered
        );
    }
}
