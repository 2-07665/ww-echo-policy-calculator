fn preset_item_to_response(
    item: ScorerPresetResolvedItem,
    built_in: bool,
    user_defined: bool,
) -> ScorerPresetResponseItem {
    ScorerPresetResponseItem {
        preset_name: item.preset_name,
        variants: item
            .variants
            .into_iter()
            .map(|variant| ScorerPresetResponseVariantItem {
                variant_name: variant.variant_name,
                weights: variant.weights,
                main_buff_score: variant.main_buff_score,
                normalized_max_score: variant.normalized_max_score,
                preset_intro: variant.preset_intro,
            })
            .collect(),
        built_in,
        user_defined,
    }
}

fn build_merged_preset_response(
    scorer_type: &str,
    built_in_items: &[ScorerPresetFileItem],
    user_items: &[ScorerPresetFileItem],
) -> Vec<ScorerPresetResponseItem> {
    let built_in_resolved = resolve_preset_groups_for_scorer(scorer_type, built_in_items);
    let user_resolved = resolve_preset_groups_for_scorer(scorer_type, user_items);

    let mut out = Vec::new();
    for user_preset in &user_resolved {
        if built_in_resolved
            .iter()
            .any(|item| item.preset_name == user_preset.preset_name)
        {
            out.push(preset_item_to_response(user_preset.clone(), true, true));
        } else {
            out.push(preset_item_to_response(user_preset.clone(), false, true));
        }
    }

    for built_in_preset in &built_in_resolved {
        if !user_resolved
            .iter()
            .any(|item| item.preset_name == built_in_preset.preset_name)
        {
            out.push(preset_item_to_response(built_in_preset.clone(), true, false));
        }
    }

    out
}
