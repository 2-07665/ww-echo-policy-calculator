#[tauri::command]
fn delete_scorer_preset(
    app: tauri::AppHandle,
    payload: DeleteScorerPresetRequest,
) -> Result<DeleteScorerPresetResponse, String> {
    let mut context = load_preset_command_context(&app, &payload.scorer_type)?;
    let scorer_type = context.scorer_type.as_str();
    let preset_name = normalize_preset_name(&payload.preset_name)?;
    let mut user_items = std::mem::take(&mut context.user_items);

    let Some(existing_index) = find_preset_group_index(&user_items, &preset_name) else {
        if context
            .built_in_items
            .iter()
            .any(|item| item.preset_name == preset_name)
        {
            return Err(format!("Bundled preset '{preset_name}' cannot be deleted"));
        }
        return Err(format!("Preset '{preset_name}' does not exist"));
    };

    user_items.remove(existing_index);
    user_items = normalize_loaded_preset_groups(scorer_type, user_items);
    write_user_preset_items(&context.file_path, &user_items)?;
    let presets = merged_preset_response_items(scorer_type, &context.built_in_items, &user_items);
    Ok(DeleteScorerPresetResponse {
        deleted_preset_name: preset_name,
        presets,
    })
}

#[tauri::command]
fn delete_scorer_preset_variant(
    app: tauri::AppHandle,
    payload: DeleteScorerPresetVariantRequest,
) -> Result<DeleteScorerPresetVariantResponse, String> {
    let mut context = load_preset_command_context(&app, &payload.scorer_type)?;
    let scorer_type = context.scorer_type.as_str();
    let preset_name = normalize_preset_name(&payload.preset_name)?;
    let variant_name = normalize_preset_variant_name(&payload.variant_name)?;
    let built_in_resolved = resolve_preset_groups_for_scorer(scorer_type, &context.built_in_items);
    let mut user_items = std::mem::take(&mut context.user_items);

    let user_index = find_preset_group_index(&user_items, &preset_name);
    let bundled_has_variant = find_resolved_preset(&built_in_resolved, &preset_name)
        .and_then(|preset| find_resolved_variant(preset, &variant_name))
        .is_some();

    let Some(user_index) = user_index else {
        if bundled_has_variant {
            return Err(format!(
                "Bundled variant '{} / {}' cannot be deleted",
                preset_name, variant_name
            ));
        }
        return Err(format!(
            "Preset variant '{} / {}' does not exist",
            preset_name, variant_name
        ));
    };

    let resolved_user_preset = resolve_preset_group_for_scorer(scorer_type, &user_items[user_index])?;
    let default_variant_name = resolved_user_preset
        .variants
        .first()
        .map(|variant| variant.variant_name.clone())
        .unwrap_or_else(|| SCORER_PRESET_VARIANT_NAME_DEFAULT.to_string());
    if variant_name == default_variant_name {
        return Err(format!(
            "Default variant '{}' cannot be deleted",
            variant_name
        ));
    }

    let Some(variant_index) = user_items[user_index]
        .variants
        .iter()
        .position(|variant| variant.variant_name == variant_name)
    else {
        if bundled_has_variant {
            return Err(format!(
                "Bundled variant '{} / {}' cannot be deleted",
                preset_name, variant_name
            ));
        }
        return Err(format!(
            "Preset variant '{} / {}' does not exist",
            preset_name, variant_name
        ));
    };

    if variant_index == 0 {
        return Err(format!(
            "Default variant '{}' cannot be deleted",
            variant_name
        ));
    }

    user_items[user_index].variants.remove(variant_index);
    user_items = normalize_loaded_preset_groups(scorer_type, user_items);

    if let Some(bundled_preset) = find_resolved_preset(&built_in_resolved, &preset_name)
        && let Some(new_user_index) = find_preset_group_index(&user_items, &preset_name)
    {
        let resolved_user_after =
            resolve_preset_group_for_scorer(scorer_type, &user_items[new_user_index])?;
        if resolved_presets_equal(&resolved_user_after, bundled_preset) {
            user_items.remove(new_user_index);
        }
    }

    write_user_preset_items(&context.file_path, &user_items)?;
    let presets = merged_preset_response_items(scorer_type, &context.built_in_items, &user_items);
    Ok(DeleteScorerPresetVariantResponse {
        deleted_preset_name: preset_name,
        deleted_variant_name: variant_name,
        presets,
    })
}
