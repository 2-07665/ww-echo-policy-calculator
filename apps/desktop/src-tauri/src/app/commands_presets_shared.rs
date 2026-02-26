struct PresetCommandContext {
    scorer_type: String,
    file_path: PathBuf,
    built_in_items: Vec<ScorerPresetFileItem>,
    user_items: Vec<ScorerPresetFileItem>,
}

fn load_preset_command_context(
    app: &tauri::AppHandle,
    scorer_type_raw: &str,
) -> Result<PresetCommandContext, String> {
    let scorer_type = parse_scorer_type(scorer_type_raw)?;
    let file_path = scorer_preset_file_path(app, scorer_type)?;
    let built_in_items = read_built_in_scorer_presets(scorer_type)?;
    let file = read_scorer_preset_file(&file_path)?;
    let user_items = normalize_loaded_preset_groups(scorer_type, file.presets);
    Ok(PresetCommandContext {
        scorer_type: scorer_type.to_string(),
        file_path,
        built_in_items,
        user_items,
    })
}

fn write_user_preset_items(
    file_path: &Path,
    user_items: &[ScorerPresetFileItem],
) -> Result<(), String> {
    write_scorer_preset_file(
        file_path,
        &ScorerPresetFile {
            presets: user_items.to_vec(),
        },
    )
}

fn merged_preset_response_items(
    scorer_type: &str,
    built_in_items: &[ScorerPresetFileItem],
    user_items: &[ScorerPresetFileItem],
) -> Vec<ScorerPresetResponseItem> {
    build_merged_preset_response(scorer_type, built_in_items, user_items)
}
