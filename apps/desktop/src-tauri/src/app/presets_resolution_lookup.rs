fn find_preset_group_index(groups: &[ScorerPresetFileItem], preset_name: &str) -> Option<usize> {
    groups
        .iter()
        .position(|item| item.preset_name.as_str() == preset_name)
}

fn find_resolved_preset<'a>(
    presets: &'a [ScorerPresetResolvedItem],
    preset_name: &str,
) -> Option<&'a ScorerPresetResolvedItem> {
    presets
        .iter()
        .find(|item| item.preset_name.as_str() == preset_name)
}

fn find_resolved_variant<'a>(
    preset: &'a ScorerPresetResolvedItem,
    variant_name: &str,
) -> Option<&'a ScorerPresetResolvedVariantItem> {
    preset
        .variants
        .iter()
        .find(|item| item.variant_name.as_str() == variant_name)
}
