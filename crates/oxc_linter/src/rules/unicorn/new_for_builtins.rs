use oxc_ast::{AstKind, ast::Expression};
use oxc_diagnostics::OxcDiagnostic;
use oxc_macros::declare_oxc_lint;
use oxc_span::Span;
use oxc_syntax::operator::BinaryOperator;

use crate::{
    AstNode, config::GlobalValue, context::LintContext, globals::GLOBAL_OBJECT_NAMES, rule::Rule,
};

fn enforce(span: Span, fn_name: &str) -> OxcDiagnostic {
    OxcDiagnostic::warn(format!("Use `new {fn_name}()` instead of `{fn_name}()`")).with_label(span)
}

fn disallow(span: Span, fn_name: &str) -> OxcDiagnostic {
    OxcDiagnostic::warn(format!("Use `{fn_name}()` instead of `new {fn_name}()`")).with_label(span)
}

fn error_date(span: Span) -> OxcDiagnostic {
    OxcDiagnostic::warn("Use `String(new Date())` instead of `Date()`").with_label(span)
}

#[derive(Debug, Default, Clone)]
pub struct NewForBuiltins;

declare_oxc_lint!(
    /// ### What it does
    ///
    /// Enforces the use of `new` for the following builtins: `Object`, `Array`, `ArrayBuffer`, `BigInt64Array`,
    /// `BigUint64Array`, `DataView`, `Date`, `Error`, `Float32Array`, `Float64Array`, `Function`, `Int8Array`,
    /// `Int16Array`, `Int32Array`, `Map`, `WeakMap`, `Set`, `WeakSet`, `Promise`, `RegExp`, `Uint8Array`,
    /// `Uint16Array`, `Uint32Array`, `Uint8ClampedArray`, `SharedArrayBuffer`, `Proxy`, `WeakRef`, `FinalizationRegistry`.
    ///
    /// Disallows the use of `new` for the following builtins: `String`, `Number`, `Boolean`, `Symbol`, `BigInt`.
    ///
    /// ### Why is this bad?
    ///
    /// Using `new` inconsistently can cause confusion. Constructors like `Array` and `RegExp` should always use `new`
    /// to ensure the expected instance type. Meanwhile, `String`, `Number`, `Boolean`, `Symbol`, and `BigInt` should not use `new`,
    /// as they create object wrappers instead of primitive values.
    ///
    /// ### Examples
    ///
    /// Examples of **incorrect** code for this rule:
    /// ```javascript
    /// const foo = new String('hello world');
    /// const bar = Array(1, 2, 3);
    /// const now = Date();
    /// ```
    ///
    /// Examples of **correct** code for this rule:
    /// ```javascript
    /// const foo = String('hello world');
    /// const bar = new Array(1, 2, 3);
    /// const now = String(new Date());
    /// ```
    NewForBuiltins,
    unicorn,
    pedantic,
    pending,
    version = "0.0.16",
    short_description = "Enforce the use of `new` for most builtins.",
);

impl Rule for NewForBuiltins {
    fn run<'a>(&self, node: &AstNode<'a>, ctx: &LintContext<'a>) {
        match node.kind() {
            AstKind::NewExpression(new_expr) => {
                let Some(builtin_name) = is_expr_global_builtin(&new_expr.callee, ctx) else {
                    return;
                };

                if DISALLOW_NEW_FOR_BUILTINS.contains(&builtin_name) {
                    ctx.diagnostic(disallow(new_expr.span, builtin_name));
                }
            }
            AstKind::CallExpression(call_expr) => {
                let Some(builtin_name) = is_expr_global_builtin(&call_expr.callee, ctx) else {
                    return;
                };

                if ENFORCE_NEW_FOR_BUILTINS.contains(builtin_name) {
                    if builtin_name == "Object" {
                        let parent_kind = ctx.nodes().parent_kind(node.id());
                        if let AstKind::BinaryExpression(bin_expr) = parent_kind
                            && (bin_expr.operator == BinaryOperator::StrictEquality
                                || bin_expr.operator == BinaryOperator::StrictInequality)
                        {
                            return;
                        }
                    }

                    // `Date()` returns a string representation of the current date and time, exactly as `new Date().toString()` does.
                    // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Date/Date#return_value
                    if builtin_name == "Date" {
                        ctx.diagnostic(error_date(call_expr.span));
                        return;
                    }

                    ctx.diagnostic(enforce(call_expr.span, builtin_name));
                }
            }
            _ => {}
        }
    }
}

fn is_expr_global_builtin<'a, 'b>(
    expr: &'b Expression<'a>,
    ctx: &'b LintContext<'a>,
) -> Option<&'b str> {
    let expr = expr.without_parentheses();
    if let Expression::Identifier(ident) = expr {
        if !ctx.is_reference_to_global_variable(ident) {
            return None;
        }

        Some(ident.name.as_str())
    } else {
        let member_expr = expr.as_member_expression()?;

        let Expression::Identifier(ident) = member_expr.object().without_parentheses() else {
            return None;
        };

        let name = ident.name.as_str();

        if !GLOBAL_OBJECT_NAMES.contains(&name) {
            return None;
        }

        if ctx.globals().get(name).is_some_and(|value| *value == GlobalValue::Off) {
            return None;
        }

        member_expr.static_property_name()
    }
}

const ENFORCE_NEW_FOR_BUILTINS: phf::Set<&'static str> = phf::phf_set![
    "Array",
    "ArrayBuffer",
    "BigInt64Array",
    "BigUint64Array",
    "DataView",
    "Date",
    "Error",
    "FinalizationRegistry",
    "Float32Array",
    "Float64Array",
    "Function",
    "Int16Array",
    "Int32Array",
    "Int8Array",
    "Map",
    "Object",
    "Promise",
    "Proxy",
    "RegExp",
    "Set",
    "SharedArrayBuffer",
    "Uint16Array",
    "Uint32Array",
    "Uint8Array",
    "Uint8ClampedArray",
    "WeakMap",
    "WeakRef",
    "WeakSet",
];

const DISALLOW_NEW_FOR_BUILTINS: [&str; 5] = ["BigInt", "Boolean", "Number", "Symbol", "String"];

#[test]
fn test() {
    use crate::tester::Tester;

    let pass = vec![
        ("const foo = new Object()", None, None),
        ("const foo = new Array()", None, None),
        ("const foo = new ArrayBuffer()", None, None),
        ("const foo = new BigInt64Array()", None, None),
        ("const foo = new BigUint64Array()", None, None),
        ("const foo = new DataView()", None, None),
        ("const foo = new Error()", None, None),
        ("const foo = new Float16Array()", None, None),
        ("const foo = new Float32Array()", None, None),
        ("const foo = new Float64Array()", None, None),
        ("const foo = new Function()", None, None),
        ("const foo = new Int8Array()", None, None),
        ("const foo = new Int16Array()", None, None),
        ("const foo = new Int32Array()", None, None),
        ("const foo = new Map()", None, None),
        ("const foo = new Map([['foo', 'bar'], ['unicorn', 'rainbow']])", None, None),
        ("const foo = new WeakMap()", None, None),
        ("const foo = new Set()", None, None),
        ("const foo = new WeakSet()", None, None),
        ("const foo = new Promise()", None, None),
        ("const foo = new RegExp()", None, None),
        ("const foo = new UInt8Array()", None, None),
        ("const foo = new UInt16Array()", None, None),
        ("const foo = new UInt32Array()", None, None),
        ("const foo = new Uint8ClampedArray()", None, None),
        ("const foo = BigInt()", None, None),
        ("const foo = Boolean()", None, None),
        ("const foo = Number()", None, None),
        ("const foo = String()", None, None),
        ("const foo = Symbol()", None, None),
        (
            "
                        import { Map } from 'immutable';
                        const m = Map();
                    ",
            None,
            None,
        ),
        (
            "
                        const {Map} = require('immutable');
                        const foo = Map();
                    ",
            None,
            None,
        ),
        (
            "
                        const {String} = require('guitar');
                        const lowE = new String();
                    ",
            None,
            None,
        ),
        (
            "
                        import {String} from 'guitar';
                        const lowE = new String();
                    ",
            None,
            None,
        ),
        ("new Foo();Bar();", None, None),
        ("Foo();new Bar();", None, None),
        ("const isObject = v => Object(v) === v;", None, None),
        ("const isObject = v => globalThis.Object(v) === v;", None, None),
        ("(x) !== Object(x)", None, None),
        (r#"new Symbol("")"#, None, Some(serde_json::json!({ "globals": { "Symbol": "off" } }))),
        ("const foo = new Date();", None, None),
    ];

    let fail = vec![
        ("const object = (Object)();", None, None),
        (r#"const symbol = new (Symbol)("");"#, None, None),
        (r#"const symbol = new /* comment */ Symbol("");"#, None, None),
        ("const symbol = new Symbol;", None, None),
        (
            "() => {
                return new // 1
                    Symbol();
            }",
            None,
            None,
        ),
        (
            "() => {
                return (
                    new // 2
                        Symbol()
                );
            }",
            None,
            None,
        ),
        (
            "() => {
                return new // 3
                    (Symbol);
            }",
            None,
            None,
        ),
        (
            "() => {
                return new // 4
                    Symbol;
            }",
            None,
            None,
        ),
        (
            "() => {
                return (
                    new // 5
                        Symbol
                );
            }",
            None,
            None,
        ),
        (
            "() => {
                return (
                    new // 6
                        (Symbol)
                );
            }",
            None,
            None,
        ),
        (
            "() => {
                throw new // 1
                    Symbol();
            }",
            None,
            None,
        ),
        (
            "() => {
                return new /**/ Symbol;
            }",
            None,
            None,
        ),
        ("new globalThis.String()", None, None),
        ("new global.String()", None, None),
        ("new self.String()", None, None),
        ("new window.String()", None, None),
        // TODO: Fix.
        // (
        //     "const {String} = globalThis;
        //     new String();",
        //     None,
        //     None,
        // ),
        // (
        //     "const {String: RenamedString} = globalThis;
        //     new RenamedString();",
        //     None,
        //     None,
        // ),
        // (
        //     "const RenamedString = globalThis.String;
        //     new RenamedString();",
        //     None,
        //     None,
        // ),
        ("globalThis.Array()", None, None),
        ("global.Array()", None, None),
        ("self.Array()", None, None),
        ("window.Array()", None, None),
        // (
        //     "const {Array: RenamedArray} = globalThis;
        //     RenamedArray();",
        //     None,
        //     None,
        // ),
        ("globalThis.Array()", None, Some(serde_json::json!({ "globals": { "Array": "off" } }))),
        // (
        //     "const {Array} = globalThis;
        //     Array();",
        //     None,
        //     Some(serde_json::json!({ "globals": { "Symbol": "off" } })),
        // ),
        ("const foo = Object()", None, None),
        ("const foo = Array()", None, None),
        ("const foo = ArrayBuffer()", None, None),
        ("const foo = BigInt64Array()", None, None),
        ("const foo = BigUint64Array()", None, None),
        ("const foo = DataView()", None, None),
        ("const foo = Error()", None, None),
        ("const foo = Error('Foo bar')", None, None),
        // ("const foo = Float16Array()", None, None),
        ("const foo = Float32Array()", None, None),
        ("const foo = Float64Array()", None, None),
        ("const foo = Function()", None, None),
        ("const foo = Int8Array()", None, None),
        ("const foo = Int16Array()", None, None),
        ("const foo = Int32Array()", None, None),
        ("const foo = (( Map ))()", None, None),
        ("const foo = Map([['foo', 'bar'], ['unicorn', 'rainbow']])", None, None),
        ("const foo = WeakMap()", None, None),
        ("const foo = Set()", None, None),
        ("const foo = WeakSet()", None, None),
        ("const foo = Promise()", None, None),
        ("const foo = RegExp()", None, None),
        ("const foo = Uint8Array()", None, None),
        ("const foo = Uint16Array()", None, None),
        ("const foo = Uint32Array()", None, None),
        ("const foo = Uint8ClampedArray()", None, None),
        ("const foo = new BigInt(123)", None, None),
        ("const foo = new Boolean()", None, None),
        ("const foo = new Number()", None, None),
        ("const foo = new Number('123')", None, None),
        ("const foo = new String()", None, None),
        ("const foo = new Symbol()", None, None),
        (
            "function varCheck() {
                {
                    var WeakMap = function() {};
                }
                // This should not reported
                return WeakMap()
            }
            function constCheck() {
                {
                    const Array = function() {};
                }
                return Array()
            }
            function letCheck() {
                {
                    let Map = function() {};
                }
                return Map()
            }",
            None,
            None,
        ),
        (
            "function foo() {
                return(globalThis).Map()
            }",
            None,
            None,
        ),
        ("const foo = Date();", None, None),
        ("const foo = globalThis.Date();", None, None),
        (
            "function foo() {
                return(globalThis).Date();
            }",
            None,
            None,
        ),
        ("const foo = Date(/*comment*/);", None, None),
        ("const foo = globalThis/*comment*/.Date();", None, None),
        ("const foo = Date(bar);", None, None),
    ];

    Tester::new(NewForBuiltins::NAME, NewForBuiltins::PLUGIN, pass, fail).test_and_snapshot();
}
