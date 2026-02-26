pub(crate) fn run() {
    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            get_ocr_udp_listener_status,
            start_ocr_udp_listener,
            stop_ocr_udp_listener,
            load_scorer_presets,
            save_scorer_preset,
            save_scorer_preset_variant,
            delete_scorer_preset,
            delete_scorer_preset_variant,
            preview_upgrade_score,
            compute_policy,
            policy_suggestion,
            compute_reroll_policy,
            query_reroll_recommendation
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
