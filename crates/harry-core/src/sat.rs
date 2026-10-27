//! The sprite list rules of SAT (Sprite Animation Toolkit), as the game
//! configures it (`InitSAT`: `SATConfigure(false, layer sort, collision
//! mode 1, search width 64)`). Sprites are kept in one list:
//!
//! - a new sprite goes to the front (`SATNewSprite`), so it does not run
//!   in the frame it was made in if its maker runs from the list;
//! - `SATRun` (PPC 0x1006290c) runs each sprite's task in list order and
//!   takes its absolute hot rect (`hotRect2` = position + hot rect) right
//!   after it, then does **one** bubble pass forward and one backward by
//!   layer (`sort_pass`), then the collision pass (`collisions`), then
//!   clears kind and hit task of sprites whose task is gone (killed);
//! - `CheckPlayerCollision` (after `SATRun`) tests the player against all
//!   other sprites with a task, using `at` and the current hot rect;
//! - sprites whose task is gone or that are `frozen` leave the list when
//!   the frame is drawn (`EraseSprites`).
//!
//! The functions work on a list of sprite indices; the game state keeps
//! the sprites.

use crate::bounce::Rect;

/// Strict rect overlap (SAT's `FUN_10060f70`, also `SectRect`'s answer):
/// empty rects never overlap.
pub fn overlap(a: Rect, b: Rect) -> bool {
    a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3]
}

/// The sort pass (`FUN_10062100`, layer mode): one bubble pass from the
/// front, moving a sprite before its predecessor when its layer is lower,
/// then one from the back. Not a full sort: the forward pass carries the
/// highest layer to the end, the backward pass the lowest to the front,
/// others move a step or two - the list settles over a few frames. (The
/// original starts the backward pass before the sprite that was last in
/// the forward pass; that pair is in order either way.)
pub fn sort_pass(order: &mut [usize], layer: impl Fn(usize) -> i16) {
    let n = order.len();
    for p in 1..n {
        if layer(order[p]) < layer(order[p - 1]) {
            order.swap(p, p - 1);
        }
    }
    for q in (0..n.saturating_sub(1)).rev() {
        if layer(order[q + 1]) < layer(order[q]) {
            order.swap(q, q + 1);
        }
    }
}

/// The collision pass (`FUN_10061644`): each sprite against those after it
/// in the list, stopping once the next one's layer exceeds its own by
/// more than the search width (64). `hit(a, b)` is called for each
/// overlapping pair, a before b; the original then calls a's hit task
/// with b and b's with a (each if it has one).
pub fn collisions(order: &[usize], layer: impl Fn(usize) -> i16, rect: impl Fn(usize) -> Rect, mut hit: impl FnMut(usize, usize)) {
    for (i, &a) in order.iter().enumerate() {
        let limit = layer(a).wrapping_add(64);
        let mut j = i + 1;
        while j < order.len() {
            let b = order[j];
            if overlap(rect(a), rect(b)) {
                hit(a, b);
            }
            j += 1;
            if j < order.len() && limit < layer(order[j]) {
                break;
            }
        }
    }
}

/// `rect` offset by (v, h).
pub fn absolute(rect: Rect, at: (i16, i16)) -> Rect {
    [rect[0].wrapping_add(at.0), rect[1].wrapping_add(at.1), rect[2].wrapping_add(at.0), rect[3].wrapping_add(at.1)]
}
