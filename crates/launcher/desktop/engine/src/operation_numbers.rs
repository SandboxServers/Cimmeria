//! Process-local operation numbers for launcher events (C6). The operation
//! UUID is a durable identifier and never leaves the process (R5); every
//! event of one operation carries the same small number instead.
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;
use uuid::Uuid;

const KEEP: usize = 64;
static NEXT: AtomicU64 = AtomicU64::new(1);
static SEEN: Mutex<Vec<(Uuid, u64, Instant)>> = Mutex::new(Vec::new());

fn entry(id: Uuid) -> (u64, Instant) {
    let mut seen = SEEN.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(&(_, number, first)) = seen.iter().find(|(known, _, _)| *known == id) {
        return (number, first);
    }
    if seen.len() == KEEP {
        seen.remove(0);
    }
    let made = (NEXT.fetch_add(1, Ordering::Relaxed), Instant::now());
    seen.push((id, made.0, made.1));
    made
}

/// The number for `id`, assigned the first time this process logs it.
pub fn number(id: Uuid) -> u64 {
    entry(id).0
}

/// Milliseconds since `number` (or this call) first saw `id`.
pub fn elapsed_ms(id: Uuid) -> u64 {
    u64::try_from(entry(id).1.elapsed().as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_id_keeps_its_number() {
        let id = Uuid::new_v4();
        assert_eq!(number(id), number(id));
    }

    #[test]
    fn different_ids_get_different_numbers() {
        assert_ne!(number(Uuid::new_v4()), number(Uuid::new_v4()));
    }

    #[test]
    fn an_old_id_is_forgotten_after_keep_others() {
        let a = Uuid::new_v4();
        let first = number(a);
        for _ in 0..KEEP {
            number(Uuid::new_v4());
        }
        assert_ne!(number(a), first);
    }
}
