/* pebble_shim.h - hand-written C99 shims linked into the Pebble runtime.
 *
 * Everything in this header has a safe Rust wrapper in `pebble-sys`'s lib.rs.
 * The ABI is plain C: integers by value, pointers by address.
 */
#ifndef PEBBLE_SHIM_H
#define PEBBLE_SHIM_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* FNV-1a, 64-bit variant (offset basis 0xcbf29ce484222325, prime 0x100000001b3).
 *
 * Reads `len` bytes through `data` and returns the resulting digest. `data` may
 * be NULL when `len` is 0. The function keeps no reference to `data`.
 */
uint64_t pebble_fnv1a(const uint8_t *data, size_t len);

/* A bump (arena) allocator over a single upfront allocation.
 *
 * The struct is opaque: callers only ever hold a `PebbleArena *`. Every
 * function tolerates a NULL arena. Arenas are not thread-safe.
 */
typedef struct PebbleArena PebbleArena;

/* Creates an arena with room for `capacity` bytes.
 *
 * Returns NULL if memory cannot be obtained. A capacity of 0 is allowed: the
 * arena exists, `pebble_arena_used` stays 0 and every `pebble_arena_alloc`
 * fails. Release the result with `pebble_arena_destroy`.
 */
PebbleArena *pebble_arena_create(size_t capacity);

/* Bump-allocates `len` uninitialised bytes.
 *
 * The returned region is aligned to the widest scalar alignment the platform
 * requires (16 bytes on x86-64) and
 * never overlaps a region handed out by a previous call that has not been
 * invalidated by `pebble_arena_reset`. Contents are unspecified.
 *
 * Returns NULL when the arena is NULL, when the arena has no backing storage
 * (capacity 0) or when `len` bytes plus alignment padding do not fit in the
 * remaining capacity.
 */
void *pebble_arena_alloc(PebbleArena *arena, size_t len);

/* Bytes handed out so far, including alignment padding between allocations.
 * Returns 0 for a NULL arena. */
size_t pebble_arena_used(const PebbleArena *arena);

/* Total number of bytes the arena can hand out. Returns 0 for a NULL arena. */
size_t pebble_arena_capacity(const PebbleArena *arena);

/* Rewinds the arena so all of its capacity becomes available again.
 *
 * Every pointer previously returned by `pebble_arena_alloc` is invalidated by
 * this call: using one afterwards is undefined behaviour. No-op for NULL.
 */
void pebble_arena_reset(PebbleArena *arena);

/* Releases the arena and its backing storage. No-op for NULL. Afterwards the
 * arena pointer and every pointer it handed out are dangling. */
void pebble_arena_destroy(PebbleArena *arena);

/* Callback signature used by `pebble_fold`. `ctx` is the opaque pointer the
 * caller passed to `pebble_fold`; it may be NULL. */
typedef int64_t (*pebble_fold_fn)(int64_t acc, int64_t value, void *ctx);

/* Folds the integers in the inclusive range [lo, hi].
 *
 * Calls `cb(acc, value, ctx)` once per integer, in ascending order, threading
 * `init` as the initial accumulator and the callback's result as the next
 * accumulator. Returns the final accumulator, or `init` when `lo > hi`.
 *
 * `cb` must not be NULL. `pebble_fold` does not catch anything: how a callback
 * failure is handled is up to the callback.
 */
int64_t pebble_fold(int64_t init, int64_t lo, int64_t hi, pebble_fold_fn cb, void *ctx);

#ifdef __cplusplus
}
#endif

#endif /* PEBBLE_SHIM_H */
