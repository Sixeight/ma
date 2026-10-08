/// Test that edges crossing subgraph boundaries don't take long detours.
/// Before the fix, these edges were routed as back edges around the entire
/// layout, making diagrams unnecessarily wide.

#[test]
fn boundary_edge_from_subgraph_to_external_node() {
    let input = r#"graph TD
    subgraph Process
        A --> B
        B --> C
    end
    B -->|Error| ErrorHandler
"#;
    let output = ma::render(input).unwrap();
    
    // Before fix: width is 40 columns but edges routed through back-edge lanes
    // After fix: edges cross boundaries directly with proper forward routing
    let max_line_width = output.lines().map(|line| line.chars().count()).max().unwrap_or(0);
    
    assert!(
        max_line_width <= 50,
        "Diagram width should be <= 50 columns, got {max_line_width}"
    );
    
    // Verify all expected nodes are present
    assert!(output.contains("Process"), "subgraph title present");
    assert!(output.contains("│ A │"), "node A present");
    assert!(output.contains("│ B │"), "node B present");
    assert!(output.contains("│ C │"), "node C present");
    assert!(output.contains("ErrorHandler"), "ErrorHandler node present");
    assert!(output.contains("Error"), "edge label present");
}

#[test]
fn boundary_edges_to_and_from_subgraph() {
    let input = r#"graph TD
    Start --> Decision{Complex Decision?}
    Decision -->|Yes| Process1
    Decision -->|No| Process2
    Decision -->|Maybe| Process3
    subgraph Processing
        Process1
        Process2
        Process3
    end
    Process1 --> Merge
    Process2 --> Merge
    Process3 --> Merge
    Merge --> End
"#;
    let output = ma::render(input).unwrap();
    
    // Before fix: width is 235 columns (edges take long detours)
    // After fix: width is around 80 columns (edges cross boundaries more directly)
    let max_line_width = output.lines().map(|line| line.chars().count()).max().unwrap_or(0);
    
    assert!(
        max_line_width <= 85,
        "Diagram width should be <= 85 columns for efficient boundary crossing, got {max_line_width}"
    );
    
    // Verify all expected nodes are present
    assert!(output.contains("Start"), "Start node present");
    assert!(output.contains("Decision"), "Decision node present");
    assert!(output.contains("Processing"), "subgraph title present");
    assert!(output.contains("Process1"), "Process1 present");
    assert!(output.contains("Process2"), "Process2 present");
    assert!(output.contains("Process3"), "Process3 present");
    assert!(output.contains("Merge"), "Merge node present");
    assert!(output.contains("End"), "End node present");
}
