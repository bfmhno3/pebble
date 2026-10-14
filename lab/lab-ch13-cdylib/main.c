#include <stdint.h>
#include <stdio.h>

int64_t pebble_add(int64_t a, int64_t b);

int main(void) {
    printf("pebble_add(40, 2) = %lld\n", (long long)pebble_add(40, 2));
    return 0;
}
