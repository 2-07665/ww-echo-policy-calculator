#[tauri::command]
fn load_scorer_presets(
    app: tauri::AppHandle,
    payload: LoadScorerPresetsRequest,
) -> Result<LoadScorerPresetsResponse, String> {
    let context = load_preset_command_context(&app, &payload.scorer_type)?;
    let presets = merged_preset_response_items(
        &context.scorer_type,
        &context.built_in_items,
        &context.user_items,
    );
    Ok(LoadScorerPresetsResponse { presets })
}

#[tauri::command]
fn save_scorer_preset(
    app: tauri::AppHandle,
    payload: SaveScorerPresetRequest,
) -> Result<SaveScorerPresetResponse, String> {
    let mut context = load_preset_command_context(&app, &payload.scorer_type)?;
    let scorer_type = context.scorer_type.as_str();
    let preset_name = normalize_preset_name(&payload.preset_name)?;
    let mut user_items = std::mem::take(&mut context.user_items);

    let bundled_exists = find_preset_group_index(&context.built_in_items, &preset_name).is_some();
    let user_index = find_preset_group_index(&user_items, &preset_name);
    if bundled_exists && user_index.is_none() {
        return Err(format!(
            "Bundled preset '{}' is read-only. Save it using a new preset name.",
            preset_name
        ));
    }

    let user_resolved = resolve_preset_groups_for_scorer(scorer_type, &user_items);
    let fallback_intro = find_resolved_preset(&user_resolved, &preset_name)
        .and_then(|preset| preset.variants.first())
        .and_then(|variant| variant.preset_intro.clone());

    let requested_variant_name = payload
        .variant_name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(normalize_preset_variant_name)
        .transpose()?;

    let base_variant_name = if let Some(index) = user_index {
        user_items[index]
            .variants
            .first()
            .map(|variant| variant.variant_name.clone())
            .unwrap_or_else(|| SCORER_PRESET_VARIANT_NAME_DEFAULT.to_string())
    } else {
        requested_variant_name.unwrap_or_else(|| SCORER_PRESET_VARIANT_NAME_DEFAULT.to_string())
    };

    let resolved_base_variant = build_resolved_variant_from_payload(
        scorer_type,
        &base_variant_name,
        &payload.weights,
        payload.main_buff_score,
        payload.normalized_max_score,
        payload.preset_intro.or(fallback_intro),
    )?;
    let serialized_base_variant = resolved_variant_to_file_full(&resolved_base_variant);

    if let Some(existing_index) = user_index {
        if user_items[existing_index].variants.is_empty() {
            user_items[existing_index]
                .variants
                .push(serialized_base_variant.clone());
        } else {
            user_items[existing_index].variants[0] = serialized_base_variant.clone();
        }
    } else {
        user_items.push(ScorerPresetFileItem {
            preset_name: preset_name.clone(),
            variants: vec![serialized_base_variant],
        });
    }

    user_items = normalize_loaded_preset_groups(scorer_type, user_items);

    write_user_preset_items(&context.file_path, &user_items)?;
    let presets = merged_preset_response_items(scorer_type, &context.built_in_items, &user_items);
    Ok(SaveScorerPresetResponse {
        saved_preset_name: preset_name,
        saved_variant_name: resolved_base_variant.variant_name,
        presets,
    })
}

#[tauri::command]
fn save_scorer_preset_variant(
    app: tauri::AppHandle,
    payload: SaveScorerPresetVariantRequest,
) -> Result<SaveScorerPresetVariantResponse, String> {
    let mut context = load_preset_command_context(&app, &payload.scorer_type)?;
    let scorer_type = context.scorer_type.as_str();
    let preset_name = normalize_preset_name(&payload.preset_name)?;
    let variant_name = normalize_preset_variant_name(&payload.variant_name)?;
    let mut user_items = std::mem::take(&mut context.user_items);
    let user_index = find_preset_group_index(&user_items, &preset_name).ok_or_else(|| {
        if find_preset_group_index(&context.built_in_items, &preset_name).is_some() {
            format!(
                "Bundled preset '{}' is read-only. Save as a new preset first.",
                preset_name
            )
        } else {
            format!("Preset '{preset_name}' does not exist")
        }
    })?;
    let user_resolved = resolve_preset_groups_for_scorer(scorer_type, &user_items);
    let source_preset = find_resolved_preset(&user_resolved, &preset_name)
        .ok_or_else(|| format!("Preset '{preset_name}' does not exist"))?;
    let default_variant_name = source_preset
        .variants
        .first()
        .map(|variant| variant.variant_name.clone())
        .unwrap_or_else(|| SCORER_PRESET_VARIANT_NAME_DEFAULT.to_string());

    if variant_name == default_variant_name {
        return Err(format!(
            "Variant '{}' is the default variant. Use preset save to update it.",
            variant_name
        ));
    }

    let current_user_preset = find_resolved_preset(&user_resolved, &preset_name)
        .ok_or_else(|| format!("Preset '{preset_name}' failed to load"))?;
    let base_variant = current_user_preset
        .variants
        .first()
        .ok_or_else(|| format!("Preset '{preset_name}' has no base variant"))?;
    let fallback_intro = find_resolved_variant(current_user_preset, &variant_name)
        .and_then(|variant| variant.preset_intro.clone());

    let resolved_variant = build_resolved_variant_from_payload(
        scorer_type,
        &variant_name,
        &payload.weights,
        payload.main_buff_score,
        payload.normalized_max_score,
        payload.preset_intro.or(fallback_intro),
    )?;
    let serialized_variant = build_variant_override_from_base(base_variant, &resolved_variant);

    if let Some(existing_index) = user_items[user_index]
        .variants
        .iter()
        .position(|variant| variant.variant_name == variant_name)
    {
        if existing_index == 0 {
            return Err(format!(
                "Variant '{}' is the default variant. Use preset save to update it.",
                variant_name
            ));
        }
        user_items[user_index].variants[existing_index] = serialized_variant;
    } else {
        user_items[user_index].variants.push(serialized_variant);
    }

    user_items = normalize_loaded_preset_groups(scorer_type, user_items);
    write_user_preset_items(&context.file_path, &user_items)?;
    let presets = merged_preset_response_items(scorer_type, &context.built_in_items, &user_items);
    Ok(SaveScorerPresetVariantResponse {
        saved_preset_name: preset_name,
        saved_variant_name: variant_name,
        presets,
    })
}
