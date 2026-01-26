use crate::data::{MAX_SELECTED_TYPES, TOTAL_BUFF_TYPES};

// The mask with bit 1 on every slot.
pub(super) const MASK_ALL: u16 = (1u16 << TOTAL_BUFF_TYPES) - 1;
// The range to search for masks with up to (MAX_SELECTED_TYPES - 1) bit 1.
const MASK_SPACE: usize =
    ((1 << (MAX_SELECTED_TYPES - 1)) - 1) << (TOTAL_BUFF_TYPES - (MAX_SELECTED_TYPES - 1));
// The number of valid masks.
pub(super) const NUM_OF_MASKS: usize = count_masks();
// The array of valid masks.
pub(super) const MASKS: [u16; NUM_OF_MASKS] = build_masks();
// The index of a mask in MASKS
const MASK_TO_INDEX: [usize; MASK_SPACE + 1] = build_mask_to_index();

#[inline(always)]
pub(super) const fn used_slots(mask: u16) -> usize {
    mask.count_ones() as usize
}

#[inline(always)]
const fn is_valid_mask(mask: u16) -> bool {
    used_slots(mask) <= (MAX_SELECTED_TYPES - 1)
}

const fn count_masks() -> usize {
    let mut count: usize = 0;
    let mut mask: u16 = 0;
    loop {
        if is_valid_mask(mask) {
            count += 1;
        }
        if mask == MASK_SPACE as u16 {
            break;
        }
        mask += 1;
    }
    count
}

const fn build_masks() -> [u16; NUM_OF_MASKS] {
    let mut masks = [0u16; NUM_OF_MASKS];
    let mut idx: usize = 0;
    let mut mask: u16 = 0;
    loop {
        if is_valid_mask(mask) {
            masks[idx] = mask;
            idx += 1;
        }
        if mask == MASK_SPACE as u16 {
            break;
        }
        mask += 1;
    }
    masks
}

const fn build_mask_to_index() -> [usize; MASK_SPACE + 1] {
    let mut map = [0usize; MASK_SPACE + 1];
    let mut idx: usize = 0;
    let mut mask: u16 = 0;
    loop {
        if is_valid_mask(mask) {
            map[mask as usize] = idx;
            idx += 1;
        }
        if mask == MASK_SPACE as u16 {
            break;
        }
        mask += 1;
    }
    map
}

/// Must ensure mask is valid.
#[inline(always)]
pub(super) fn mask_to_cache_index(mask: u16) -> usize {
    MASK_TO_INDEX[mask as usize]
}

pub(super) fn best_case_remaining_score(
    mask: u16,
    buff_max_score: &[u16; TOTAL_BUFF_TYPES],
) -> u16 {
    let used_slots = used_slots(mask);
    if used_slots >= MAX_SELECTED_TYPES {
        return 0;
    }

    let remaining_slots = MAX_SELECTED_TYPES - used_slots;
    let mut top = [0u16; MAX_SELECTED_TYPES];
    for buff_type_index in 0..TOTAL_BUFF_TYPES {
        if (mask & (1u16 << buff_type_index)) != 0 {
            continue;
        }
        let s = buff_max_score[buff_type_index];
        if s <= top[remaining_slots - 1] {
            continue;
        }
        let mut j = remaining_slots - 1;
        while j > 0 && s > top[j - 1] {
            top[j] = top[j - 1];
            j -= 1;
        }
        top[j] = s;
    }
    top[..remaining_slots].iter().sum()
}

pub(super) struct MaskCache {
    continue_values: Vec<f64>,
    pub(super) touched: Vec<usize>,

    min_score: u16,
    pub(super) best_case_remaining_score: u16,
    cut_off_score: Option<u16>,
}

impl MaskCache {
    pub(super) fn new(min_score: u16, max_score: u16, best_case_remaining_score: u16) -> Self {
        let size = (max_score - min_score + 1) as usize;

        Self {
            continue_values: vec![f64::NAN; size],
            touched: Vec::with_capacity(size),

            min_score,
            best_case_remaining_score,
            cut_off_score: None,
        }
    }

    pub(super) fn min_score(&self) -> u16 {
        self.min_score
    }

    /// Get the continue value for a score.
    ///
    /// Output is NAN if the continue value has not been set.
    pub(super) fn get_value(&self, score: u16) -> f64 {
        let idx = (score - self.min_score) as usize;
        return self.continue_values[idx];
    }

    pub(super) fn set(&mut self, score: u16, continue_value: f64, decision: bool) {
        let idx = (score - self.min_score) as usize;
        if self.continue_values[idx].is_nan() {
            self.touched.push(idx);
        }
        self.continue_values[idx] = continue_value;

        if decision {
            self.cut_off_score = Some(self.cut_off_score.map_or(score, |s| s.min(score)));
        }
    }

    pub(super) fn cut_off_score(&self) -> Option<u16> {
        self.cut_off_score
    }

    pub(super) fn get_decision(&self, score: u16) -> Option<bool> {
        self.cut_off_score.map(|s| score >= s)
    }

    pub(super) fn clear_touched(&mut self) {
        for &idx in self.touched.iter() {
            self.continue_values[idx] = f64::NAN;
        }
        self.touched.clear();
        self.cut_off_score = None;
    }
}
