// slate-lint/src/lib.rs

pub struct Lint;

impl Lint {
    /// Run all registered checks over a program.
    pub fn check(program: &Program) -> Vec<Warning>;

    /// Run only specific checks.
    pub fn check_with(program: &Program, checks: &[CheckKind]) -> Vec<Warning>;
}

#[derive(Debug, Clone)]
pub struct Warning {
    pub code: WarningCode,
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub source_line: String,
    pub suggestion: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity { Hint, Warning, Error }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarningCode {
    // Bracket-mismatch — the star of the show
    ParenInFuncParams,      // W0001: func add(5, 3) — should be <5, 3>
    AngleInBuiltinCall,     // W0002: write<x> — should be (x)
    AngleInCondition,       // W0003: if (x > 5) — should be <x > 5>
    ParenInCondition,       // W0004: if (x) — never valid, use <x>
    UnclosedAngle,          // W0005: func add<a,b { ... } — missing '>'
    UnclosedParen,          // W0006: write("hi" — missing ')'

    // Optimization hints
    UnusedVariable,         // W0100: make x = 5 but x never referenced
    UnusedFunction,         // W0101: func foo<> never called
    ConstantCondition,      // W0102: if <true> { ... } else { ... }
    RedundantElse,          // W0103: elif <true>
    UnreachableCode,        // W0104: statements after `return`
    EmptyBlock,             // W0105: if <x> { } — probably a mistake
    ShadowedVariable,       // W0106: make x = ... after make x = ... earlier
}