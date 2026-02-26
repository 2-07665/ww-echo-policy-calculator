fn resolve_preset_group_for_scorer(
    scorer_type: &str,
    preset: &ScorerPresetFileItem,
) -> Result<ScorerPresetResolvedItem, String> {
    let preset_name = normalize_preset_name(&preset.preset_name)?;
    let base_raw_variant = preset
        .variants
        .first()
        .ok_or_else(|| format!("Preset '{preset_name}' must contain at least one variant"))?;
    let default_variant_name = if base_raw_variant.variant_name.trim().is_empty() {
        SCORER_PRESET_VARIANT_NAME_DEFAULT.to_string()
    } else {
        base_raw_variant.variant_name.clone()
    };
    let (weights, main_buff_score, normalized_max_score, preset_intro) =
        normalize_preset_variant_values_for_scorer(
            scorer_type,
            &btree_weights_to_hash_map(&base_raw_variant.weights),
            base_raw_variant
                .main_buff_score
                .or(default_main_buff_score_for_scorer(scorer_type)),
            base_raw_variant
                .normalized_max_score
                .or(default_normalized_max_score_for_scorer(scorer_type)),
            base_raw_variant.preset_intro.clone(),
        )?;
    let base_variant = ScorerPresetResolvedVariantItem {
        variant_name: normalize_preset_variant_name(&default_variant_name)?,
        weights,
        main_buff_score,
        normalized_max_score,
        preset_intro,
    };

    let mut variants = vec![base_variant.clone()];
    for raw_variant in preset.variants.iter().skip(1) {
        match resolve_variant_from_base(scorer_type, &base_variant, raw_variant) {
            Ok(variant) => {
                if !variants
                    .iter()
                    .any(|existing| existing.variant_name == variant.variant_name)
                {
                    variants.push(variant);
                }
            }
            Err(err) => {
                eprintln!(
                    "Skipping invalid variant '{}' in preset '{}': {err}",
                    raw_variant.variant_name, preset_name
                );
            }
        }
    }

    Ok(ScorerPresetResolvedItem {
        preset_name,
        variants,
    })
}

fn normalize_loaded_preset_groups(
    scorer_type: &str,
    items: Vec<ScorerPresetFileItem>,
) -> Vec<ScorerPresetFileItem> {
    let mut out = Vec::new();

    for item in items {
        match resolve_preset_group_for_scorer(scorer_type, &item) {
            Ok(resolved) => {
                if out
                    .iter()
                    .any(|existing: &ScorerPresetFileItem| existing.preset_name == resolved.preset_name)
                {
                    continue;
                }

                let Some(base_variant) = resolved.variants.first() else {
                    eprintln!(
                        "Skipping invalid preset '{}': no normalized variants",
                        resolved.preset_name
                    );
                    continue;
                };
                let mut normalized_variants = vec![resolved_variant_to_file_full(base_variant)];
                for variant in resolved.variants.iter().skip(1) {
                    normalized_variants.push(build_variant_override_from_base(base_variant, variant));
                }

                out.push(ScorerPresetFileItem {
                    preset_name: resolved.preset_name,
                    variants: normalized_variants,
                });
            }
            Err(err) => {
                eprintln!("Skipping invalid preset '{}': {err}", item.preset_name);
            }
        }
    }

    out
}

fn resolve_preset_groups_for_scorer(
    scorer_type: &str,
    groups: &[ScorerPresetFileItem],
) -> Vec<ScorerPresetResolvedItem> {
    groups
        .iter()
        .filter_map(|group| match resolve_preset_group_for_scorer(scorer_type, group) {
            Ok(resolved) => Some(resolved),
            Err(err) => {
                eprintln!("Skipping invalid preset '{}': {err}", group.preset_name);
                None
            }
        })
        .collect()
}
