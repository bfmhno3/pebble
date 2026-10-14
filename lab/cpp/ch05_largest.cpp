struct Pebble {};

template <typename T>
T largest(T a, T b) {
    return a > b ? a : b;
}

int main() {
    largest(Pebble{}, Pebble{});
}
