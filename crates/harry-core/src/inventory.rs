//! The inventory: 12 slots of {object id, count}, kept compact (empty
//! slots at the end). The original's routines also draw the inventory
//! panel; only their effect on the state is ported here.

use crate::frame::{PlayerGlobals, SLOTS};

/// Object ids of the items (the level editor's).
pub mod item {
    pub const STAPLE_GUN: i16 = 11299;
    /// 11397: an item only the debug keys give (F4).
    pub const DEBUG_ITEM: i16 = 11397;
    pub const CAFFEINE_PILL: i16 = 11499;
    pub const BADASS: i16 = 11599;
    pub const MYSTERY_VIAL: i16 = 11699;
    pub const SHRAPNEL_GUN: i16 = 11799;
    pub const SODA_CAN: i16 = 11899;
    pub const POWER_OF_THE_SWIVEL: i16 = 11999;
}

/// No object in the slot.
pub const EMPTY: i16 = -1;

/// The id of the selected item. Slot −1 (nothing selected) reads the word
/// before the table, the chatter timer (BUGS.md) - never an item id.
pub fn current_item(g: &PlayerGlobals) -> i16 {
    match g.cur_slot {
        -1 => g.chatter,
        s => g.inventory[s as usize].0,
    }
}

/// `GetInvItem` (PPC 0x10026148): one more of `id` - in its slot if Harry
/// has one already, else in the first empty slot (selected if nothing
/// is). False when the inventory is full.
pub fn get_item(g: &mut PlayerGlobals, id: i16) -> bool {
    if let Some(s) = g.inventory.iter().position(|e| e.0 == id) {
        g.inventory[s].1 = g.inventory[s].1.wrapping_add(1);
        return true;
    }
    let Some(s) = g.inventory.iter().position(|e| e.0 == EMPTY) else { return false };
    g.inventory[s] = (id, 1);
    if g.cur_slot == -1 {
        g.cur_slot = s as i16;
    }
    true
}

/// `SwitchInvItem` (PPC 0x10025eb8): select `slot`, or with −1 the next
/// item (wrapping to the first). Nothing happens with an empty inventory.
pub fn switch_item(g: &mut PlayerGlobals, slot: i16) {
    if g.inventory[0].0 == EMPTY {
        return;
    }
    g.cur_slot = if slot != -1 {
        slot
    } else if g.cur_slot == SLOTS as i16 - 1 || g.inventory[(g.cur_slot + 1) as usize].0 == EMPTY {
        0
    } else {
        g.cur_slot + 1
    };
}

/// `RemoveInvItem` (PPC 0x10025b5c): use up one of the item in `slot`.
/// The last one empties the slot, the items behind it move up, and the
/// selection moves to the slot before (or stays on slot 0 while it holds
/// something).
pub fn remove_item(g: &mut PlayerGlobals, slot: i16) {
    let s = slot as usize;
    g.inventory[s].1 = g.inventory[s].1.wrapping_sub(1);
    if g.inventory[s].1 >= 1 {
        return;
    }
    g.inventory[s] = (EMPTY, 0);
    for k in 0..SLOTS - 1 {
        if g.inventory[k].0 == EMPTY && g.inventory[k + 1].0 != EMPTY {
            g.inventory.copy_within(k + 1.., k);
            g.inventory[SLOTS - 1] = (EMPTY, 0);
        }
    }
    if g.cur_slot == slot {
        if slot != 0 {
            g.cur_slot -= 1;
        } else if g.inventory[0].0 == EMPTY {
            g.cur_slot = -1;
        }
    }
}
