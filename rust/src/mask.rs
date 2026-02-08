use crate::data::{NUM_BUFFS, NUM_ECHO_SLOTS};

// The mask with bit 1 on every slot.
pub const MASK_ALL: u16 = (1u16 << NUM_BUFFS) - 1;

// The range to search for partial masks (up to (NUM_ECHO_SLOTS - 1) bit 1).
pub const PARTIAL_MASK_SPACE: usize =
    ((1 << (NUM_ECHO_SLOTS - 1)) - 1) << (NUM_BUFFS - (NUM_ECHO_SLOTS - 1));
// The number of valid partial masks.
pub const NUM_PARTIAL_MASKS: usize = count_partial_masks();
// The array of valid partial masks.
pub static PARTIAL_MASKS: [u16; NUM_PARTIAL_MASKS] = build_partial_masks();
// The index of a partial mask in PARTIAL_MASKS
static PARTIAL_MASK_TO_INDEX: [usize; PARTIAL_MASK_SPACE + 1] = build_partial_mask_to_index();

pub const FULL_MASK_SPACE: usize = ((1 << NUM_ECHO_SLOTS) - 1) << (NUM_BUFFS - NUM_ECHO_SLOTS);
pub const NUM_FULL_MASKS: usize = count_full_masks();
pub static FULL_MASKS: [u16; NUM_FULL_MASKS] = build_full_masks();
pub static FULL_MASK_TO_INDEX: [usize; FULL_MASK_SPACE + 1] = build_full_mask_to_index();

#[inline(always)]
pub const fn calculate_num_filled_slots(mask: u16) -> usize {
    mask.count_ones() as usize
}

#[inline(always)]
const fn is_valid_partial_mask(mask: u16) -> bool {
    calculate_num_filled_slots(mask) <= (NUM_ECHO_SLOTS - 1)
}

#[inline(always)]
const fn is_mask_in_domain(mask: u16) -> bool {
    (mask & !MASK_ALL) == 0
}

const fn count_partial_masks() -> usize {
    let mut count: usize = 0;
    let mut mask: u16 = 0;
    loop {
        if is_valid_partial_mask(mask) {
            count += 1;
        }
        if mask == PARTIAL_MASK_SPACE as u16 {
            break;
        }
        mask += 1;
    }
    count
}

const fn build_partial_masks() -> [u16; NUM_PARTIAL_MASKS] {
    let mut masks = [0u16; NUM_PARTIAL_MASKS];
    let mut idx: usize = 0;
    let mut mask: u16 = 0;
    loop {
        if is_valid_partial_mask(mask) {
            masks[idx] = mask;
            idx += 1;
        }
        if mask == PARTIAL_MASK_SPACE as u16 {
            break;
        }
        mask += 1;
    }
    masks
}

const fn build_partial_mask_to_index() -> [usize; PARTIAL_MASK_SPACE + 1] {
    let mut map = [0usize; PARTIAL_MASK_SPACE + 1];
    let mut idx: usize = 0;
    let mut mask: u16 = 0;
    loop {
        if is_valid_partial_mask(mask) {
            map[mask as usize] = idx;
            idx += 1;
        }
        if mask == PARTIAL_MASK_SPACE as u16 {
            break;
        }
        mask += 1;
    }
    map
}

#[inline(always)]
pub const fn is_valid_full_mask(mask: u16) -> bool {
    calculate_num_filled_slots(mask) == NUM_ECHO_SLOTS
}

#[inline(always)]
pub const fn is_valid_external_partial_mask(mask: u16) -> bool {
    is_mask_in_domain(mask) && is_valid_partial_mask(mask)
}

#[inline(always)]
pub const fn is_valid_external_full_mask(mask: u16) -> bool {
    is_mask_in_domain(mask) && is_valid_full_mask(mask)
}

const fn count_full_masks() -> usize {
    let mut count: usize = 0;
    let mut mask: u16 = 0;
    loop {
        if is_valid_full_mask(mask) {
            count += 1;
        }
        if mask == FULL_MASK_SPACE as u16 {
            break;
        }
        mask += 1;
    }
    count
}

const fn build_full_masks() -> [u16; NUM_FULL_MASKS] {
    let mut masks = [0u16; NUM_FULL_MASKS];
    let mut idx: usize = 0;
    let mut mask: u16 = 0;
    loop {
        if is_valid_full_mask(mask) {
            masks[idx] = mask;
            idx += 1;
        }
        if mask == FULL_MASK_SPACE as u16 {
            break;
        }
        mask += 1;
    }
    masks
}

const fn build_full_mask_to_index() -> [usize; FULL_MASK_SPACE + 1] {
    let mut map = [0usize; FULL_MASK_SPACE + 1];
    let mut idx: usize = 0;
    let mut mask: u16 = 0;
    loop {
        if is_valid_full_mask(mask) {
            map[mask as usize] = idx;
            idx += 1;
        }
        if mask == FULL_MASK_SPACE as u16 {
            break;
        }
        mask += 1;
    }
    map
}

/// It does not check whether `mask` is a valid partial mask.
#[inline(always)]
pub fn partial_mask_to_index(mask: u16) -> usize {
    PARTIAL_MASK_TO_INDEX[mask as usize]
}

/// It does not check whether `mask` is a valid full mask.
#[inline(always)]
pub fn full_mask_to_index(mask: u16) -> usize {
    FULL_MASK_TO_INDEX[mask as usize]
}

pub fn bits_to_mask(bits: &[u8]) -> u16 {
    let mut mask: u16 = 0;
    for (index, &bit) in bits.iter().enumerate().take(NUM_BUFFS) {
        match bit {
            0 => {}
            1 => mask |= 1u16 << index,
            _ => {}
        }
    }
    mask
}

pub fn mask_to_bits(mask: u16) -> [u8; NUM_BUFFS] {
    let mut bits = [0; NUM_BUFFS];
    for (index, bit) in bits.iter_mut().enumerate().take(NUM_BUFFS) {
        *bit = ((mask >> index) & 1) as u8;
    }
    bits
}
