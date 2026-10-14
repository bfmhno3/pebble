#include <iostream>
#include <memory>

struct Int; struct Add;

struct Visitor {
    virtual void visit(const Int&) = 0;
    virtual void visit(const Add&) = 0;
    virtual ~Visitor() = default;
};

struct Expr {
    virtual void accept(Visitor&) const = 0;
    virtual ~Expr() = default;
};

struct Int : Expr {
    long value;
    explicit Int(long v) : value(v) {}
    void accept(Visitor& v) const override { v.visit(*this); }
};

struct Add : Expr {
    std::unique_ptr<Expr> lhs, rhs;
    Add(std::unique_ptr<Expr> l, std::unique_ptr<Expr> r) : lhs(std::move(l)), rhs(std::move(r)) {}
    void accept(Visitor& v) const override { v.visit(*this); }
};

struct Printer : Visitor {
    void visit(const Int& e) override { std::cout << "int " << e.value << "\n"; }
    void visit(const Add& e) override {
        std::cout << "add\n";
        e.lhs->accept(*this);
        e.rhs->accept(*this);
    }
};

int main() {
    Add tree(std::make_unique<Int>(1), std::make_unique<Int>(2));
    Printer p;
    tree.accept(p);
}
