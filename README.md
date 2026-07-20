# mylang

A statically typed programming language with a bytecode compiler and stack-based VM, implemented in Rust (~5.6k LOC).

## Quick Start

```bash
cargo build
```

Create a `hello.my` file:

```mylang
def std = import("std");
std::io::println("hello world");
```

Run:

```bash
cargo run -- hello.my
```

## Examples

### Counter with closure

```mylang
def make_counter = func() -> func() -> int {
    mut count = 0;
    func() -> int { ++count; };
};
def ctr = make_counter();
std::io::println(ctr().to_str());  # 1
std::io::println(ctr().to_str());  # 2
std::io::println(ctr().to_str());  # 3
```

### Recursive factorial

```mylang
def fact = func(n: int) -> int {
    if n <= 1; then { 1; } else { n * fact(n - 1); };
};
std::io::println(fact(5).to_str());  # 120
```

## Language Reference

### Types

| Type                    | Syntax   | Example                           |
| ----------------------- | -------- | --------------------------------- |
| `int`                   | Integer  | `42`                              |
| `float`                 | Float    | `3.14`                            |
| `bool`                  | Boolean  | `true`, `false`                   |
| `string`                | String   | `"hello"`                         |
| `void`                  | Void     | `nil`                             |
| `T[]`                   | Array    | `[1, 2, 3]`                       |
| `func(T1, T2) -> Ret`   | Function | `func(a: int) -> int { a + 1; }`  |
| `struct { name: T, .. }`| Struct   | `struct Point { x: int, y: int }` |

### Variables

```mylang
def x = 42;                # immutable, type inferred
mut y = 0;                  # mutable, type inferred
def s: string = "hello";   # immutable, type annotated
mut arr: int[] = [];        # mutable, type annotated
```

### Functions

```mylang
# anonymous function bound to a name
def add = func(a: int, b: int) -> int {
    a + b;
};

# recursive function (no keyword needed)
def factorial = func(n: int) -> int {
    if n == 0 || n == 1; then {
        return 1;
    };
    n * factorial(n - 1);
};

# calling
def result = add(3, 4);
def fact = factorial(5);
```

### Structs

```mylang
# declaration
struct Point {
    x: int,
    y: int,
}

# literal
def p = Point { x: 10, y: 20 };

# property access
def px = p.x;

# public struct (exported from modules)
pub struct Point {
    x: int,
    y: int,
}
```

### Control Flow

```mylang
# if/then/else (expression, note the ';' after condition and 'then' keyword)
def max = if a > b; then { a; } else { b; };

# if without else returns the then-branch type
if true; then {
    def temp = 1;
}

# while loop (note the ';' after condition and 'do' keyword)
mut i = 0;
while i < 10; do {
    i = i + 1;
}

# break and continue
mut n = 0;
while true; do {
    n = n + 1;
    if n == 5; then { break; };
    if n % 2 == 0; then { continue; };
};
```

### Operators

**Arithmetic:**

| Operator | Description              |
| -------- | ------------------------ |
| `+`      | Addition                 |
| `-`      | Subtraction              |
| `*`      | Multiplication           |
| `/`      | Division (int/int → float) |
| `%`      | Modulo                   |
| `**`     | Power (neg. exponent → float) |

**Comparison:**

| Operator | Description      |
| -------- | ---------------- |
| `==`     | Equal            |
| `!=`     | Not equal        |
| `<`      | Less than        |
| `<=`     | Less or equal    |
| `>`      | Greater than     |
| `>=`     | Greater or equal |

**Logical:**

| Operator | Description |
| -------- | ----------- |
| `&&`     | Logical and |
| `||`     | Logical or  |
| `!`      | Logical not |

**Bitwise:**

| Operator | Description |
| -------- | ----------- |
| `&`      | Bitwise and |
| `|`      | Bitwise or  |
| `^`      | Bitwise xor |
| `<<`     | Shift left  |
| `>>`     | Shift right |

**Other:**

| Operator | Description       |
| -------- | ----------------- |
| `=`      | Assignment        |
| `++`     | Increment (`mut` only) |
| `--`     | Decrement (`mut` only) |
| `as`     | Type cast         |
| `.`      | Property access   |
| `::`     | Module path       |
| `()`     | Function call     |
| `[]`     | Index access      |

**Precedence (highest to lowest):**

| Precedence | Operator                 | Associativity |
| ---------- | ------------------------ | ------------- |
| 14         | `.`, `::`, `(`, `[`, `{` | Left          |
| 13         | `**`                     | Right         |
| 12         | `*`, `/`, `%`            | Left          |
| 11         | `+`, `-`                 | Left          |
| 10         | `<<`, `>>`               | Left          |
| 9          | `as`                     | Left          |
| 8          | `&`                      | Left          |
| 7          | `^`                      | Left          |
| 6          | `|`                      | Left          |
| 5          | `<`, `<=`, `>`, `>=`     | Left          |
| 4          | `==`, `!=`               | Left          |
| 3          | `&&`                     | Left          |
| 2          | `||`                     | Left          |
| 1          | `=`                      | Right         |

Prefix operators: `-`, `!`, `++`, `--`

### Casting

```mylang
def f = 3.14 as int;   # float → int (truncates)
def g = 42 as float;   # int → float
def h = 3.14 as float; # same type (identity)
```

### Arrays

```mylang
def nums: int[] = [1, 2, 3, 4, 5];
def first = nums[0];      # index access (positive and negative)
def last = nums[-1];      # negative index: from end

# built-in methods
nums.push(6);              # push element
def popped = nums.pop();   # pop last element
def len = nums.len();      # length
nums.clear();              # clear array
```

### Strings

```mylang
def s = "hello";
def len = s.len();          # length
def upper = s.upcase();     # uppercase
def lower = s.lowcase();    # lowercase
def chars = s.chars();      # split into string[]
def trimmed = "  hi  ".trim(); # trim whitespace
```

### Type Conversions

```mylang
# to_str() on primitives
def s1 = (42).to_str();       # int → "42"
def s2 = (3.14).to_str();     # float → "3.14"
def s3 = true.to_str();       # bool → "true"
```

### Modules and Imports

```mylang
# import a module (looks for <file>.my)
def math = import("math");

# access with ::
def result = math::add(1, 2);
def p = math::Point { x: 0, y: 0 };

# public exports (in math.my)
pub def add = func(a: int, b: int) -> int {
    a + b;
};
```

### Standard Library

```mylang
# io module
std::io::print("hello");                    # print without newline
std::io::println("hello");                  # print with newline
def name = std::io::readln("Name: ");       # read line with prompt

# math module
def pi = std::math::PI;                     # 3.141592653589793
def absv = std::math::abs(-5);              # 5
```

### Comments

```mylang
# this is a single-line comment
```

### pub

```mylang
pub def exported = 42;       # exported from module
pub struct Point { ... }     # exported struct
```

## Architecture

```
Source (.my) → Lexer → Tokens → Parser → AST → TypeChecker → TypedAST → Compiler → Chunk → VM → Value
```

| Module          | Files                        | LOC    | Description                                                  |
| --------------- | ---------------------------- | ------ | ------------------------------------------------------------ |
| **Lexer**       | `src/lexer/`                 | ~560   | Tokenization with `Span` (file, line, col, byte_offset), 16 keywords, 27 operators |
| **Parser**      | `src/parser/`                | ~1050  | Pratt parser (expressions) + recursive descent (statements) |
| **TypeChecker** | `src/typechecker/`           | ~1650  | Bidirectional type inference/checking, `TypeEnv`, `TypeRegistry` |
| **Compiler**    | `src/compiler/`              | ~860   | AST → bytecode, local slot allocation, upvalue resolution, 42 opcodes |
| **VM**          | `src/vm/`                    | ~1200  | Stack-based VM, `CallFrame`, close-upvalue, property dispatch |

### Lexer

Produces tokens with `Span { file, start: Pos, end: Pos }`. Tracks line, column, and byte offset for precise error reporting. Keywords include `def`, `mut`, `if`/`then`/`else`, `while`/`do`/`break`/`continue`, `func`/`return`, `struct`, `nil`, `true`/`false`, `as`, `pub`.

### Parser

Pratt parser for expressions (binding power encodes precedence). Recursive descent for statements. Produces `Ast` (list of `Stmt`, each containing `Expr` trees). Supports blocks, if/then/else (including else-if chains), while/break/continue, struct declarations, function literals.

### TypeChecker

Bidirectional inference: `infer_expr` determines type from context, `check_expr` verifies against expected type. `TypeEnv` manages scoped bindings with shadowing. `TypeRegistry` holds struct definitions. `TypedAst` is the fully-annotated output consumed by the compiler.

### Compiler

Recursive AST → bytecode compilation. Key mechanisms:

| Concept | Implementation |
|---------|---------------|
| **Local slots** | Flat index into stack frame, allocated by `add_local` |
| **Upvalues** | Captured outer-scope variables, resolved by parent chain |
| **Self-recursion** | Pre-allocate slot via `add_local`, emit `Nil` placeholder, `Closure` captures it, `SetLocal` updates via shared `Rc` |
| **Scope cleanup** | `end_scope` emits `Rotate(N)` + `N × Pop` to remove locals while preserving the block's result |
| **Control flow** | Absolute jumps with patchable offsets (`Jump`, `JumpIfFalse`) |

#### Opcodes (42)

| Category | Opcodes |
|----------|---------|
| **Constants** | `Const(idx)`, `Nil`, `True`, `False` |
| **Locals** | `GetLocal(slot)`, `SetLocal(slot)`, `GetUpvalue(idx)`, `SetUpvalue(idx)` |
| **Arithmetic** | `Add`, `Sub`, `Mul`, `Div`, `Mod`, `Pow`, `Neg` |
| **Comparison** | `Eq`, `Neq`, `Lt`, `Le`, `Gt`, `Ge` |
| **Logical** | `And`, `Or`, `Not` |
| **Bitwise** | `BitAnd`, `BitOr`, `BitXor`, `Shl`, `Shr` |
| **Cast** | `AsInt`, `AsFloat` |
| **Control flow** | `Jump(addr)`, `JumpIfFalse(addr)` |
| **Functions** | `Call(argc)`, `Closure(idx, n_upv)`, `Return` |
| **Objects** | `Array(n)`, `Struct(name, n_fields)`, `GetProperty(name)`, `SetProperty(name)`, `IndexGet`, `IndexSet` |
| **Stack** | `Pop`, `Rotate(n)` |
| **Mutation** | `Increment(slot)`, `Decrement(slot)` |

### VM

Stack-based machine with explicit `CallFrame` management:

```
struct CallFrame {
    chunk: Chunk,        // function bytecode
    ip: usize,           // instruction pointer
    base: usize,         // stack base (args + locals)
    closure: Closure,    // executing closure
}
```

**Value types:**

| Variant | Runtime Representation |
|---------|----------------------|
| `Int` | `i64` |
| `Float` | `f64` |
| `String` | `String` |
| `Bool` | `bool` |
| `Nil` | unit |
| `Array` | `Rc<RefCell<Vec<Value>>>` |
| `Struct` | `{ type_name: String, fields: Rc<RefCell<Vec<(String, Value)>>> }` |
| `Closure` | `{ chunk: Chunk, upvalues: Vec<Rc<RefCell<Value>>>, upvalues_specs: Vec<Upvalue> }` |
| `NativeFunc` | `Rc<dyn Fn(&[Value]) -> Result<Value, VmError>>` |
| `Module` | `Vec<(String, Value)>` |
| `Upvalue` | `Rc<RefCell<Value>>` (only on stack slots, transparently dereferenced) |

**Close-upvalue**: When a closure captures a local variable, the stack slot is replaced with `Value::Upvalue(Rc)`. All closures capturing the same variable share the same `Rc<RefCell<Value>>`. `GetLocal`, `SetLocal`, `Increment`, and `Decrement` all dereference `Upvalue` transparently, so mutations to captured variables are visible to all closures.

### Error Handling

| Error | Source | Format |
|-------|--------|--------|
| `LexError` | Lexer | `file:line:col: lexer error: ...` |
| Parse error | Parser | `[span] Expected ...` |
| `TypeError` | TypeChecker | `[span] type mismatch: ...` |
| `CompileError` | Compiler | `[span] internal compiler invariant: ...` |
| `VmError` | VM | 9 variants: `ImmutableMutation`, `IndexError`, `PopEmpty`, `NotCallable`, `UnknownProperty`, `NoProperties`, `DivisionByZero`, `ImportError`, plus wrapped `LexError`/`ParseError`/`TypeError`/`CompileError` |

## Key Design Decisions

- **Bytecode compiler + stack VM** over tree-walking evaluator: better performance and clearer separation
- **Close-upvalue**: captured variables are shared via `Rc<RefCell<Value>>`, not copied. Mutations post-capture (including `++`/`--`) are visible to all closures
- **Self-recursion**: the compiler pre-allocates a local slot, emits a `Nil` placeholder, and the `Closure` instruction captures it via the close-upvalue mechanism. A subsequent `SetLocal` updates the shared `Rc` with the actual closure
- **`::` for module paths**, `.` for property access: clear syntactic distinction between namespace access and instance field access
- **`;` after condition in `if`/`while`**: required syntax — `if cond; then { ... }`, `while cond; do { ... }`
- **Division `int/int` → float**: follows the principle of least surprise
- **Power with negative exponent**: promotes to float (`2 ** (-1)` → `0.5`)
- **Functions as first-class values** with full closure support: closures capture their environment at definition time and can mutate captured variables

## Project Status

### Implemented

- [x] Lexer with span tracking
- [x] Pratt parser for expressions
- [x] Recursive descent for statements
- [x] Type inference and checking (bidirectional)
- [x] Struct declarations and literals
- [x] Module imports (`import("file")`)
- [x] Path access (`mod::member`)
- [x] Built-in properties (`String::len/upcase/lowcase/chars/trim`, `Array::len/push/pop/clear`, `to_str` on primitives)
- [x] Public exports (`pub`)
- [x] Bytecode compiler (42 opcodes)
- [x] Stack-based VM with `CallFrame` management
- [x] Closures with close-upvalue (mutable capture)
- [x] Standard library (`std::io::print/println/readln`, `std::math::PI/abs`)
- [x] `while`/`break`/`continue`
- [x] `++`/`--` operators on `mut` locals
- [x] `as` type casts
- [x] Negative array indexing

### Not Yet Implemented

- [ ] Pattern matching
- [ ] Generics
- [ ] Range-based for loop
