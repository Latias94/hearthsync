#[derive(Debug, Clone)]
pub(super) struct TextRangeReplacement {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) replacement: String,
}

pub(super) fn apply_range_replacements(
    content: &str,
    mut replacements: Vec<TextRangeReplacement>,
) -> String {
    if replacements.is_empty() {
        return content.to_string();
    }

    replacements.sort_by(|left, right| {
        right
            .start
            .cmp(&left.start)
            .then_with(|| right.end.cmp(&left.end))
            .then_with(|| left.replacement.cmp(&right.replacement))
    });

    let mut filtered = Vec::new();
    for replacement in replacements {
        if filtered.iter().any(|existing: &TextRangeReplacement| {
            existing.start == replacement.start
                && existing.end == replacement.end
                && existing.replacement == replacement.replacement
        }) {
            continue;
        }

        if let Some(previous) = filtered.last()
            && replacement.end > previous.start
        {
            continue;
        }

        filtered.push(replacement);
    }

    let mut rewritten = content.to_string();
    for replacement in filtered {
        rewritten.replace_range(replacement.start..replacement.end, &replacement.replacement);
    }

    rewritten
}
