/* pebble_shim.c - implementation of the C half of `pebble-sys`. Portable C99. */

#include "pebble_shim.h"

#include <stdlib.h>

/* --- FNV-1a ----------------------------------------------------------------- */

#define PEBBLE_FNV_OFFSET_BASIS UINT64_C(0xcbf29ce484222325)
#define PEBBLE_FNV_PRIME UINT64_C(0x100000001b3)

uint64_t pebble_fnv1a(const uint8_t *data, size_t len) {
    uint64_t hash = PEBBLE_FNV_OFFSET_BASIS;
    size_t i;

    for (i = 0; i < len; i++) {
        hash ^= (uint64_t)data[i];
        hash *= PEBBLE_FNV_PRIME;
    }
    return hash;
}

/* --- Arena ------------------------------------------------------------------ */

/* C99 has no max_align_t, so probe the platform's widest scalar alignment with
 * a union instead. Its size is a multiple of that alignment (16 on x86-64). */
typedef union pebble_align_probe {
    long double ld;
    void *ptr;
    int64_t i64;
    size_t sz;
} pebble_align_probe;

#define PEBBLE_ALIGNMENT ((size_t)sizeof(pebble_align_probe))

struct PebbleArena {
    unsigned char *base;
    size_t capacity;
    size_t used;
};

/* Rounds `n` up to the next multiple of PEBBLE_ALIGNMENT. Overflow is handled
 * by the capacity check in pebble_arena_alloc. */
static size_t pebble_align_up(size_t n) {
    size_t rem = n % PEBBLE_ALIGNMENT;
    return rem == 0 ? n : n + (PEBBLE_ALIGNMENT - rem);
}

PebbleArena *pebble_arena_create(size_t capacity) {
    PebbleArena *arena = (PebbleArena *)malloc(sizeof(PebbleArena));

    if (arena == NULL) {
        return NULL;
    }
    arena->base = NULL;
    arena->capacity = capacity;
    arena->used = 0;

    if (capacity > 0) {
        arena->base = (unsigned char *)malloc(capacity);
        if (arena->base == NULL) {
            free(arena);
            return NULL;
        }
    }
    return arena;
}

void *pebble_arena_alloc(PebbleArena *arena, size_t len) {
    size_t start;

    if (arena == NULL || arena->base == NULL) {
        return NULL;
    }
    start = pebble_align_up(arena->used);
    /* Both comparisons are overflow-safe: `start <= capacity` is established
     * before `capacity - start` is computed. */
    if (start > arena->capacity || len > arena->capacity - start) {
        return NULL;
    }
    arena->used = start + len;
    return arena->base + start;
}

size_t pebble_arena_used(const PebbleArena *arena) {
    return arena == NULL ? 0 : arena->used;
}

size_t pebble_arena_capacity(const PebbleArena *arena) {
    return arena == NULL ? 0 : arena->capacity;
}

void pebble_arena_reset(PebbleArena *arena) {
    if (arena != NULL) {
        arena->used = 0;
    }
}

void pebble_arena_destroy(PebbleArena *arena) {
    if (arena != NULL) {
        free(arena->base);
        free(arena);
    }
}

/* --- Fold ------------------------------------------------------------------- */

int64_t pebble_fold(int64_t init, int64_t lo, int64_t hi, pebble_fold_fn cb, void *ctx) {
    int64_t acc = init;
    int64_t value;

    /* Walking hi upwards would be pointless: the range is inclusive and `lo >
     * hi` means "empty". */
    if (lo > hi) {
        return acc;
    }
    /* Structured so that the loop variable never overflows, even for hi ==
     * INT64_MAX: the increment is skipped on the final iteration. */
    for (value = lo;; value++) {
        acc = cb(acc, value, ctx);
        if (value == hi) {
            break;
        }
    }
    return acc;
}
