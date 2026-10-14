#include <stdio.h>

enum Severity { ERROR, WARNING, NOTE, HELP };

static const char *as_str(enum Severity s) {
    switch (s) {
    case ERROR:   return "error";
    case WARNING: return "warning";
    case NOTE:    return "note";
    }
    return "???";
}

int main(void) {
    enum Severity bogus = (enum Severity)42;
    printf("%s %s\n", as_str(HELP), as_str(bogus));
    return 0;
}
