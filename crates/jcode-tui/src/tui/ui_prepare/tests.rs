use super::*;

#[test]
fn push_user_prompt_lines_truncates_long_prompts() {
    let mut lines = Vec::new();
    let mut raw_plain_lines = Vec::new();
    let mut line_raw_overrides = Vec::new();
    let mut line_copy_offsets = Vec::new();
    let mut user_line_indices = Vec::new();

    // Build content just over 10000 chars with recognizable head and tail.
    let head_part = "HEADSTART_";
    let tail_part = "_TAILEND";
    // 5000 chars of filler isn't enough; need total > 10000.
    let filler = "x".repeat(9975);
    let content = format!("{}{}{}", head_part, filler, tail_part);
    assert!(
        content.len() > 10000,
        "test content must exceed 10000 bytes but was {}",
        content.len()
    );

    push_user_prompt_lines(
        &mut lines,
        &mut raw_plain_lines,
        &mut line_raw_overrides,
        &mut line_copy_offsets,
        &mut user_line_indices,
        1,
        user_color(),
        &content,
        ratatui::layout::Alignment::Left,
    );

    let plain: Vec<String> = lines.iter().map(ui::line_plain_text).collect();

    // Should have head line + ellipsis + tail line = 3 display lines.
    assert_eq!(plain.len(), 3, "truncated prompt should render 3 display lines");

    // 1st line: prompt prefix + head content.
    assert!(plain[0].starts_with("1› HEADSTART_"), "head line should start with HEADSTART_");
    assert!(plain[0].contains("HEADSTART_"), "head line should contain HEADSTART_");

    // 2nd line: dimmed ellipsis with line/mid count.
    assert!(plain[1].contains("… +"), "ellipsis line should contain '… +'");
    assert!(plain[1].contains("lines …"), "ellipsis line should contain 'lines …'");

    // 3rd line: contnuation prefix + tail content.
    assert!(
        plain[2].ends_with("_TAILEND"),
        "tail line should end with _TAILEND, got: {:?}",
        plain[2]
    );

    // raw_plain_lines should still have the complete content (3 lines: head, mid filler, tail).
    assert_eq!(raw_plain_lines.len(), 3, "should have 3 raw lines");
    assert!(raw_plain_lines[0].starts_with("HEADSTART_"), "raw head");
    assert_eq!(
        raw_plain_lines[1],
        String::new(),
        "raw mid (ellipsis) should be empty"
    );
    assert!(raw_plain_lines[2].ends_with("_TAILEND"), "raw tail");

    // user_line_indices should point at the head line.
    assert_eq!(user_line_indices.len(), 1, "one user line index");
    assert_eq!(user_line_indices[0], 0, "user line index points at head line");
}

#[test]
fn push_user_prompt_lines_does_not_truncate_short_prompts() {
    let mut lines = Vec::new();
    let mut raw_plain_lines = Vec::new();
    let mut line_raw_overrides = Vec::new();
    let mut line_copy_offsets = Vec::new();
    let mut user_line_indices = Vec::new();

    let content = "short prompt";
    push_user_prompt_lines(
        &mut lines,
        &mut raw_plain_lines,
        &mut line_raw_overrides,
        &mut line_copy_offsets,
        &mut user_line_indices,
        1,
        user_color(),
        content,
        ratatui::layout::Alignment::Left,
    );

    let plain: Vec<String> = lines.iter().map(ui::line_plain_text).collect();
    assert_eq!(plain.len(), 1, "short prompt has 1 line");
    assert_eq!(plain[0], "1› short prompt", "short prompt rendered whole");
    assert_eq!(raw_plain_lines, vec!["short prompt"], "raw preserves full content");
    assert_eq!(user_line_indices, vec![0], "user line index set");
}

#[test]
fn centered_mode_centers_unstructured_messages_and_preserves_structured_left_blocks() {
    for role in ["user", "assistant", "meta", "usage", "error", "memory"] {
        assert_eq!(
            default_message_alignment(role, true),
            ratatui::layout::Alignment::Center,
            "role {role} should default to centered alignment"
        );
    }
    for role in ["tool", "system", "swarm", "background_task"] {
        assert_eq!(
            default_message_alignment(role, true),
            ratatui::layout::Alignment::Left,
            "role {role} should keep left/default alignment"
        );
    }
}

#[test]
fn prepare_body_preserves_multiline_user_prompt_lines() {
    let mut lines = Vec::new();
    let mut raw_plain_lines = Vec::new();
    let mut line_raw_overrides = Vec::new();
    let mut line_copy_offsets = Vec::new();
    let mut user_line_indices = Vec::new();

    push_user_prompt_lines(
        &mut lines,
        &mut raw_plain_lines,
        &mut line_raw_overrides,
        &mut line_copy_offsets,
        &mut user_line_indices,
        1,
        user_color(),
        "first line\nsecond line\n\nthird line",
        ratatui::layout::Alignment::Left,
    );

    let plain: Vec<String> = lines.iter().map(ui::line_plain_text).collect();

    assert_eq!(plain.len(), 4);
    assert_eq!(plain[0], "1› first line");
    assert_eq!(plain[1], "   second line");
    assert_eq!(plain[2], "   ");
    assert_eq!(plain[3], "   third line");
    assert_eq!(
        raw_plain_lines,
        vec!["first line", "second line", "", "third line"]
    );
    assert_eq!(user_line_indices, vec![0]);
    assert_eq!(line_copy_offsets, vec![3, 3, 3, 3]);
}
