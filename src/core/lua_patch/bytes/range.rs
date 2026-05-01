#[derive(Debug, Clone)]
pub(super) struct ByteRangeReplacement {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) replacement: Vec<u8>,
}

pub(super) fn apply_range_replacements(
    content: &[u8],
    mut replacements: Vec<ByteRangeReplacement>,
) -> Vec<u8> {
    if replacements.is_empty() {
        return content.to_vec();
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
        if filtered.iter().any(|existing: &ByteRangeReplacement| {
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

    let mut rewritten = content.to_vec();
    for replacement in filtered {
        rewritten.splice(replacement.start..replacement.end, replacement.replacement);
    }

    rewritten
}
