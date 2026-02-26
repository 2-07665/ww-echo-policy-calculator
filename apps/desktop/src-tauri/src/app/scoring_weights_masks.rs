fn parse_u16_from_f64(value: f64, field: &str) -> Result<u16, String> {
    if !value.is_finite() || value < 0.0 {
        return Err(format!("{field} must be a non-negative finite number"));
    }
    if value > u16::MAX as f64 {
        return Err(format!("{field} must be <= {}", u16::MAX));
    }
    if value.fract().abs() > f64::EPSILON {
        return Err(format!("{field} must be an integer"));
    }
    Ok(value as u16)
}

fn build_default_weight_map_f64(weights: &[f64; NUM_BUFFS]) -> BTreeMap<String, f64> {
    let mut out = BTreeMap::new();
    for (index, buff_name) in BUFF_TYPES.iter().enumerate() {
        out.insert((*buff_name).to_string(), weights[index]);
    }
    out
}

fn build_default_weight_map_u16(weights: &[u16; NUM_BUFFS]) -> BTreeMap<String, u16> {
    let mut out = BTreeMap::new();
    for (index, buff_name) in BUFF_TYPES.iter().enumerate() {
        out.insert((*buff_name).to_string(), weights[index]);
    }
    out
}

fn build_weight_array_f64(
    input: &HashMap<String, f64>,
    defaults: [f64; NUM_BUFFS],
) -> Result<[f64; NUM_BUFFS], String> {
    let mut weights = defaults;

    for (buff_name, value) in input {
        let index = buff_index(buff_name)
            .ok_or_else(|| format!("Unknown buff name in weights: {buff_name}"))?;
        if !value.is_finite() || *value < 0.0 {
            return Err(format!("Invalid weight for {buff_name}: {value}"));
        }
        weights[index] = *value;
    }

    Ok(weights)
}

fn build_weight_array_u16(
    input: &HashMap<String, u16>,
    defaults: [u16; NUM_BUFFS],
) -> Result<[u16; NUM_BUFFS], String> {
    let mut weights = defaults;

    for (buff_name, value) in input {
        let index = buff_index(buff_name)
            .ok_or_else(|| format!("Unknown buff name in weights: {buff_name}"))?;
        weights[index] = *value;
    }

    Ok(weights)
}

fn build_weight_array_u16_from_f64(
    input: &HashMap<String, f64>,
    defaults: [u16; NUM_BUFFS],
) -> Result<[u16; NUM_BUFFS], String> {
    let mut weights = defaults;

    for (buff_name, value) in input {
        let index = buff_index(buff_name)
            .ok_or_else(|| format!("Unknown buff name in weights: {buff_name}"))?;
        weights[index] = parse_u16_from_f64(*value, &format!("weight[{buff_name}]"))?;
    }

    Ok(weights)
}

fn build_mask(buff_names: &[String]) -> Result<u16, String> {
    if buff_names.len() > MAX_SELECTED_TYPES {
        return Err(format!(
            "Too many selected buffs: {}, max is {MAX_SELECTED_TYPES}",
            buff_names.len()
        ));
    }

    let mut bits = [0u8; NUM_BUFFS];
    for buff_name in buff_names {
        let index = buff_index(buff_name)
            .ok_or_else(|| format!("Unknown buff name in selection: {buff_name}"))?;
        if bits[index] == 1 {
            return Err(format!("Duplicate buff in selection: {buff_name}"));
        }
        bits[index] = 1;
    }

    Ok(bits_to_mask(&bits))
}

fn build_full_mask(buff_names: &[String]) -> Result<u16, String> {
    if buff_names.len() != MAX_SELECTED_TYPES {
        return Err(format!(
            "Exactly {MAX_SELECTED_TYPES} buff types are required, got {}",
            buff_names.len()
        ));
    }
    let mask = build_mask(buff_names)?;
    if mask.count_ones() as usize != MAX_SELECTED_TYPES {
        return Err("Buff selections must be unique and fully filled".to_string());
    }
    Ok(mask)
}

fn fixed_score_from_selected(scorer: &FixedScorer, buff_names: &[String]) -> Result<u16, String> {
    let zero_values = vec![0u16; buff_names.len()];
    let indexed = build_indexed_echo(buff_names, &zero_values)?;
    scorer
        .echo_score_display(&indexed)
        .map_err(|err| format!("Failed to compute fixed display score: {err:?}"))
}

fn lock_slot_indices_from_mask(lock_mask: u16, baseline_buff_names: &[String]) -> Vec<usize> {
    let mut slots = Vec::new();
    for (slot_idx, buff_name) in baseline_buff_names.iter().enumerate() {
        if let Some(buff_idx) = buff_index(buff_name) {
            if (lock_mask & (1u16 << buff_idx)) != 0 {
                slots.push(slot_idx + 1);
            }
        }
    }
    slots
}

