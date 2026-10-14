#include <iostream>
#include <string>
#include <string_view>

struct Token {
    std::string_view text;
};

Token first_word(const std::string& src) {
    return Token{std::string_view(src).substr(0, src.find(' '))};
}

int main() {
    Token token;
    {
        std::string source = "let answer = 42";
        token = first_word(source);
    }
    std::cout << token.text << "\n";
}
