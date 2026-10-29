//! The save file's pieces (update 3, faster saves): a list read back in
//! pieces lands whole and in order whatever order the pieces come in, and a
//! damaged piece is an error that drops every item it had read.

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::save::Fill;

static DROPS: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
struct Item(u32, String);

impl Drop for Item {
    fn drop(&mut self) {
        DROPS.fetch_add(1, Ordering::SeqCst);
    }
}

fn bytes(items: &[Item]) -> Vec<u8> {
    rmp_serde::to_vec_named(items).unwrap()
}

#[test]
fn pieces_fill_one_list() {
    let all: Vec<Item> = (0..10).map(|i| Item(i, format!("p{i}"))).collect();
    let pieces = [&all[0..4], &all[4..7], &all[7..10]];

    // Whole, the last piece first.
    let mut v: Vec<Item> = Vec::with_capacity(10);
    {
        let fills = Fill::split(&mut v, &[4, 3, 3]);
        for (k, p) in pieces.iter().enumerate().rev() {
            fills[k].lock().unwrap().read(&bytes(p)[..]).unwrap();
        }
        Fill::full(fills).unwrap().into_iter().for_each(std::mem::forget);
    }
    // Every slot holds an item (`Fill::full`).
    unsafe { v.set_len(10) };
    assert_eq!(v, all);

    // A piece shorter than the table says: an error, and the six items read
    // are dropped.
    let before = DROPS.load(Ordering::SeqCst);
    let mut v: Vec<Item> = Vec::with_capacity(10);
    {
        let fills = Fill::split(&mut v, &[4, 3, 3]);
        fills[0].lock().unwrap().read(&bytes(pieces[0])[..]).unwrap();
        fills[1].lock().unwrap().read(&bytes(&all[4..6])[..]).unwrap();
        assert!(Fill::full(fills).is_err());
    }
    assert_eq!(DROPS.load(Ordering::SeqCst) - before, 6);
    assert!(v.is_empty());

    // A piece longer than the table says: an error at once, and the four
    // items read (three placed, one with no room) are dropped.
    let before = DROPS.load(Ordering::SeqCst);
    let mut v: Vec<Item> = Vec::with_capacity(10);
    {
        let fills = Fill::split(&mut v, &[4, 3, 3]);
        assert!(fills[1].lock().unwrap().read(&bytes(&all[4..10])[..]).is_err());
    }
    assert_eq!(DROPS.load(Ordering::SeqCst) - before, 4);
    assert!(v.is_empty());
}
