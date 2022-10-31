//====================//
// Hasher & allocator //
//====================//

// FxHash appears to be the winner.
// Although AHash is a lot faster than the default hasher, I've found FxHash to be even faster.
use fxhash::{FxHashMap, FxHasher64};
pub fn new_hasher() -> FxHasher64 { FxHasher64::default() }
pub type HMap<K,V> = FxHashMap<K,V>;

// use ahash::{AHasher, AHashMap};
// fn new_hasher() -> AHasher { AHasher::default() }
// type HMap<K,V> = AHashMap<K,V>;


// Using a different allocator also makes a huge difference.
// I've found jemalloc to be better than mimalloc, both in terms of speed and memory use.
#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;