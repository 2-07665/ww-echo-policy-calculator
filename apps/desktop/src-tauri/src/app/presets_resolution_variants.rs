fn option_f64_bits_equal(left: Option<f64>, right: Option<f64>) -> bool {
    match (left, right) {
        (Some(lhs), Some(rhs)) => f64_bits_equal(lhs, rhs),
        (None, None) => true,
        _ => false,
    }
}

fn resolve_variant_from_base(
    scorer_type: &str,
    base_variant: &ScorerPresetResolvedVariantItem,
    raw_variant: &ScorerPresetVariantFileItem,
) -> Result<ScorerPresetResolvedVariantItem, String> {
    let variant_name = normalize_preset_variant_name(&raw_variant.variant_name)?;
    let mut merged_weights = btree_weights_to_hash_map(&base_variant.weights);
    for (buff_name, value) in &raw_variant.weights {
        merged_weights.insert(buff_name.clone(), *value);
    }
    let raw_main_buff_score = raw_variant.main_buff_score.or(base_variant.main_buff_score);
    let raw_normalized_max_score = raw_variant
        .normalized_max_score
        .or(base_variant.normalized_max_score);
    let raw_preset_intro = raw_variant
        .preset_intro
        .clone()
        .or(base_variant.preset_intro.clone());
    let (weights, main_buff_score, normalized_max_score, preset_intro) =
        normalize_preset_variant_values_for_scorer(
            scorer_type,
            &merged_weights,
            raw_main_buff_score,
            raw_normalized_max_score,
            raw_preset_intro,
        )?;
    Ok(ScorerPresetResolvedVariantItem {
        variant_name,
        weights,
        main_buff_score,
        normalized_max_score,
        preset_intro,
    })
}

fn build_variant_override_from_base(
    base_variant: &ScorerPresetResolvedVariantItem,
    variant: &ScorerPresetResolvedVariantItem,
) -> ScorerPresetVariantFileItem {
    let mut weights = BTreeMap::new();
    for buff_name in BUFF_TYPES {
        let base_value = *base_variant.weights.get(buff_name).unwrap_or(&0.0);
        let variant_value = *variant.weights.get(buff_name).unwrap_or(&0.0);
        if !f64_bits_equal(base_value, variant_value) {
            weights.insert(buff_name.to_string(), variant_value);
        }
    }

    let main_buff_score = if option_f64_bits_equal(base_variant.main_buff_score, variant.main_buff_score)
    {
        None
    } else {
        variant.main_buff_score
    };
    let normalized_max_score = if option_f64_bits_equal(
        base_variant.normalized_max_score,
        variant.normalized_max_score,
    ) {
        None
    } else {
        variant.normalized_max_score
    };
    let preset_intro = if base_variant.preset_intro == variant.preset_intro {
        None
    } else {
        variant.preset_intro.clone()
    };

    ScorerPresetVariantFileItem {
        variant_name: variant.variant_name.clone(),
        weights,
        main_buff_score,
        normalized_max_score,
        preset_intro,
    }
}

fn resolved_variant_to_file_full(variant: &ScorerPresetResolvedVariantItem) -> ScorerPresetVariantFileItem {
    ScorerPresetVariantFileItem {
        variant_name: variant.variant_name.clone(),
        weights: variant.weights.clone(),
        main_buff_score: variant.main_buff_score,
        normalized_max_score: variant.normalized_max_score,
        preset_intro: variant.preset_intro.clone(),
    }
}

fn build_resolved_variant_from_payload(
    scorer_type: &str,
    variant_name: &str,
    weights: &HashMap<String, f64>,
    main_buff_score: Option<f64>,
    normalized_max_score: Option<f64>,
    preset_intro: Option<String>,
) -> Result<ScorerPresetResolvedVariantItem, String> {
    let variant_name = normalize_preset_variant_name(variant_name)?;
    let (weights, main_buff_score, normalized_max_score, preset_intro) =
        normalize_preset_variant_values_for_scorer(
            scorer_type,
            weights,
            main_buff_score,
            normalized_max_score,
            preset_intro,
        )?;
    Ok(ScorerPresetResolvedVariantItem {
        variant_name,
        weights,
        main_buff_score,
        normalized_max_score,
        preset_intro,
    })
}
