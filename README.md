# mylang

A statically typed programming language with a Pratt parser, implemented in Rust.

## Quick Start

```bash
cargo build
cargo run -- test-files/test.my
```

## Types

| Type                  | Syntax   | Example                           |
| --------------------- | -------- | --------------------------------- |
| `int`                 | Integer  | `42`                              |
| `float`               | Float    | `3.14`                            |
| `bool`                | Boolean  | `true`, `false`                   |
| `string`              | String   | `"hello"`                         |
| `void`                | Void     | `nil`                             |
| `T[]`                 | Array    | `[1, 2, 3]`                       |
| `func(params) -> ret` | Function | `func(a: int) -> int { a + 1; }`  |
| `struct { ... }`      | Struct   | `struct Point { x: int, y: int }` |

## Syntax

### Variables

```mylang
def x = 42;                # immutable, type inferred
mut y = 0;                  # mutable, type inferred
def s: string = "hello";   # immutable, type annotated
mut arr: int[] = [];        # mutable, type annotated
```

### Functions

```mylang
# anonymous function
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

# if without else
if true; then {
    def temp = 1;
}

# while loop (note the ';' after condition and 'do' keyword)
mut i = 0;
while i < 10; do {
    i = i + 1;
};

# break and continue
mut n = 0;
while true; do {
    n = n + 1;
    if n == 5; then {
        break;
    };
    if n % 2 == 0; then {
        continue;
    };
};
```

### Operators

**Arithmetic:**

| Operator | Description              |
| -------- | ------------------------ |
| `+`      | Addition                 |
| `-`      | Subtraction              |
| `*`      | Multiplication           |
| `/`      | Division (returns float) |
| `%`      | Modulo                   |
| `**`     | Power                    |

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
| `\|\|`   | Logical or  |
| `!`      | Logical not |

**Bitwise:**

| Operator | Description |
| -------- | ----------- |
| `&`      | Bitwise and |
| `\|`     | Bitwise or  |
| `^`      | Bitwise xor |
| `<<`     | Shift left  |
| `>>`     | Shift right |

**Other:**

| Operator | Description     |
| -------- | --------------- |
| `=`      | Assignment      |
| `++`     | Increment       |
| `--`     | Decrement       |
| `as`     | Type cast       |
| `.`      | Property access |
| `::`     | Module path     |
| `()`     | Function call   |
| `[]`     | Index access    |

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
| 6          | `\|`                     | Left          |
| 5          | `<`, `<=`, `>`, `>=`     | Left          |
| 4          | `==`, `!=`               | Left          |
| 3          | `&&`                     | Left          |
| 2          | `\|\|`                   | Left          |
| 1          | `=`                      | Right         |

**Prefix operators:** `-`, `!`, `++`, `--`

### Casting

```mylang
def f = 3.14 as int;   # float -> int (truncates)
def g = 42 as float;   # int -> float
def h = 3.14 as float; # same type (identity)
```

### Arrays

```mylang
def nums: int[] = [1, 2, 3, 4, 5];
def first = nums[0];      # index access

# built-in methods
nums.push(6);              # push element
def last = nums.pop();     # pop last element
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
def n = 42;
def s = n.to_str();        # int -> string

def pi = 3.14;
def ps = pi.to_str();      # float -> string

def b = true;
def bs = b.to_str();       # bool -> string ("true"/"false")
```

### Modules

```mylang
# import a module (looks for file.my)
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
# print without newline
std::io::print("hello");

# print with newline
std::io::println("hello");

# read line from stdin with prompt
def name = std::io::readln("What is your name? ");
std::io::println("Hello, " + name);
```

### Comments

```mylang
# this is a single-line comment
```

## Examples

**Fibonacci:**

```mylang
def fib = func(n: int) -> int {
    if n <= 1; then {
        return n;
    };
    fib(n - 1) + fib(n - 2);
};

def result = fib(10);
```

**Structs and arrays:**

```mylang
struct Point {
    x: int,
    y: int,
}

def points: Point[] = [
    Point { x: 1, y: 2 },
    Point { x: 3, y: 4 },
];

def first_x = points[0].x;
```

## Architecture

```
Source (.my) --> Lexer --> Tokens --> Parser --> AST --> TypeChecker --> TypedAST --> Evaluator --> Value
```

| Module          | Files                   | Description                                                    |
| --------------- | ----------------------- | -------------------------------------------------------------- |
| **Lexer**       | `src/lexer/`            | Tokenization with span tracking (file, line, col)              |
| **Parser**      | `src/parser/`           | Pratt parser (expressions) + recursive descent (statements)    |
| **TypeChecker** | `src/typechecker/`      | Bidirectional type inference/checking with scope management    |
| **Evaluator**   | `src/evaluator/`        | Tree-walking evaluator with closures and environment capture   |
| **Properties**  | `src/*/properties.rs`   | Built-in property resolution for String, Array, Struct, Module |
| **Builtins**    | `src/*/builtins/mod.rs` | Standard library (`std::io`) and native function registration  |

### Key Design Decisions

- **Pratt parsing** for expressions: operator precedence is encoded in binding power pairs, making it easy to add new operators
- **RefCell + Rc** for environments: allows shared mutable scope chains without a borrow checker fight
- **TypedAST** as intermediate representation: the typechecker produces a fully typed tree, ready for evaluation
- **`::` for module paths**, `.` for property access: clear syntactic distinction between namespace access and instance field access
- **Functions as first-class values** with environment capture: closures capture the scope at definition time via `Rc<RefCell<EvalEnv>>`

### Type System

- Primitive types: `int`, `float`, `bool`, `string`, `void`
- Compound types: `T[]` (arrays), `func(...) -> T` (functions), `struct { ... }` (structs), `module` (modules)
- Type inference on variable declarations (optional annotation)
- Type checking on function bodies (return type must match)
- `TypeRegistry` for struct definitions (global, per-file)
- `TypeEnv` / `EvalEnv` for variable bindings (scoped, with shadowing)

## Roadmap

- [x] Lexer with span tracking
- [x] Pratt parser for expressions
- [x] Recursive descent for statements
- [x] Type inference and checking
- [x] Struct declarations and literals
- [x] Module imports (`import("file")`)
- [x] Path access (`mod::member`)
- [x] Built-in properties (String, Array)
- [x] Public exports (`pub`)
- [x] Tree-walking evaluator
- [x] Closures with captured environments
- [x] Standard library (`std::io`)
- [ ] Pattern matching
- [ ] Generics
