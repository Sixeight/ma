use ma::display_width::display_width;

#[test]
fn lr_branch_labels_remain_complete_and_distinct() {
    let output = ma::render(
        "flowchart LR\n\
         A[Route] -->|process small items immediately| B[Quick]\n\
         A -->|hold large items for manual review| C[Careful]\n",
    )
    .unwrap();
    for label in [
        "process small items immediately",
        "hold large items for manual review",
    ] {
        assert_eq!(output.matches(label).count(), 1, "{output}");
    }
    assert_eq!(output.matches('>').count(), 2, "{output}");
}

#[test]
fn lr_multiline_branch_labels_preserve_text_and_node_borders() {
    let output = ma::render(
        "flowchart LR\n\
         entry[Input] --> choice[Route]\n\
         choice -->|\"fast_lane<br>(軽量な項目をまとめて処理)\"| a[\"Worker A<br>まとめて処理\"]\n\
         choice -->|\"slow_lane<br>(追加の確認が必要な項目を順番に処理)\"| b[\"Worker B<br>個別に処理\"]\n\
         a --> archive[(Archive)]\n\
         b --> archive\n",
    )
    .unwrap();
    for text in [
        "fast_lane",
        "(軽量な項目をまとめて処理)",
        "slow_lane",
        "(追加の確認が必要な項目を順番に処理)",
        "│ Input │",
        "│ Route │",
        "│ Archive │",
    ] {
        assert_eq!(output.matches(text).count(), 1, "{text}:\n{output}");
    }
    for border in ["┌──────────────┐", "└──────────────┘"]
    {
        assert!(output.contains(border), "{border}:\n{output}");
    }
    assert!(!output.contains("<br>"), "{output}");
    assert!(!output.contains('"'), "{output}");
    assert_eq!(output.matches('>').count(), 4, "{output}");
}

#[test]
fn lr_multiline_labels_reserve_height_and_use_visible_width() {
    for marker in ["<br>", "<br/>", "<BR />", r"\n"] {
        let input = format!("flowchart LR\nA -->|one{marker}two{marker}three{marker}四| B\n");
        let output = ma::render_with_options(&input, Some(17)).unwrap();
        for text in ["one", "two", "three", "四", "│ A │", "│ B │"] {
            assert_eq!(output.matches(text).count(), 1, "{text}:\n{output}");
        }
        assert!(!output.contains(marker), "{output}");
        assert!(output.contains('>'), "LR layout should fit:\n{output}");
        assert!(output.lines().all(|line| display_width(line) <= 17));
    }
}

#[test]
fn narrow_lr_graph_preserves_branch_labels_when_reflowing() {
    let input = "flowchart LR\n\
                 A[Select] -->|\"first<br>確認する\"| B[Accept]\n\
                 A -->|\"second<br>保留する\"| C[Defer]\n\
                 B --> D[Finish]\n\
                 C --> D\n";
    let output = ma::render_with_options(input, Some(24)).unwrap();
    for label in ["first", "確認する", "second", "保留する"] {
        assert_eq!(output.matches(label).count(), 1, "{label}:\n{output}");
    }
    assert!(output.contains('▼'), "{output}");
    assert!(output.lines().all(|line| display_width(line) <= 24));
}

#[test]
fn width_limit_does_not_silently_clip_long_labels() {
    let input = "flowchart LR\nA -->|a label longer than the entire canvas| B\n";
    assert!(ma::render_with_options(input, Some(12)).is_err());
}

#[test]
fn later_edges_do_not_erase_multiline_labels() {
    let input = "flowchart LR\n\
                 F[Finish]\n\
                 A[Alpha] -->|upper_label<br>second_line| C[Charlie]\n\
                 B[Bravo] --> F\n";
    let output = ma::render(input).unwrap();
    for text in [
        "upper_label",
        "second_line",
        "│ Alpha │",
        "│ Bravo │",
        "│ Charlie │",
        "│ Finish │",
    ] {
        assert_eq!(output.matches(text).count(), 1, "{text}:\n{output}");
    }
}

#[test]
fn merging_edges_keep_their_multiline_labels_separate() {
    for direction in ["LR", "TD"] {
        let input = format!(
            "flowchart {direction}\nA -->|first<br>確認する| C\nB -->|second<br>保留する| C\n"
        );
        let output = ma::render(&input).unwrap();
        for label in ["first", "確認する", "second", "保留する"] {
            assert_eq!(output.matches(label).count(), 1, "{label}:\n{output}");
        }
    }
}

#[test]
fn mixed_branching_and_merging_preserves_both_labels() {
    let input = "flowchart LR\n\
                 D[Delta]\n\
                 A[Alpha] -->|first_route| C[Charlie]\n\
                 B[Bravo] --> C\n\
                 B -->|second_route| D\n";
    let output = ma::render(input).unwrap();
    for text in [
        "first_route",
        "second_route",
        "│ Alpha │",
        "│ Bravo │",
        "│ Charlie │",
        "│ Delta │",
    ] {
        assert_eq!(output.matches(text).count(), 1, "{text}:\n{output}");
    }
    assert_eq!(output.matches('>').count(), 2, "{output}");
}

#[test]
fn labels_on_edges_that_skip_ranks_do_not_erase_intermediate_nodes() {
    let input = "flowchart LR\n\
                 D[Destination]\n\
                 C[Intermediate]\n\
                 B -->|direct| D\n\
                 A -->|inspect| C\n\
                 S --> A\n\
                 S -->|bypass| Z\n\
                 A -->|forward| D\n\
                 D --> Z\n";
    let output = ma::render(input).unwrap();
    for text in ["│ Intermediate │", "direct", "inspect", "bypass", "forward"] {
        assert_eq!(output.matches(text).count(), 1, "{text}:\n{output}");
    }
}
