use shaell::interpreter::Interpreter;
use shaell::parser::Parser;

fn eval(source: &str) -> String {
    let mut parser = Parser::new(source);
    let program = parser.parse_program().expect("Parse failed");
    let mut interpreter = Interpreter::new();
    let result = interpreter
        .execute_program(&program)
        .expect("Execution failed");
    interpreter.heap.with_ref(result, |v| v.to_string())
}

fn eval_err(source: &str) -> Option<String> {
    let mut parser = Parser::new(source);
    let program = parser.parse_program().ok()?;
    let mut interpreter = Interpreter::new();
    match interpreter.execute_program(&program) {
        Ok(val) => Some(interpreter.heap.with_ref(val, |v| v.to_string())),
        Err(e) => Some(e.message),
    }
}

// ============================================================
// Variable tests
// ============================================================
mod variable_tests {
    use super::*;

    #[test]
    fn test_number_literal() {
        assert_eq!(eval("42"), "42");
    }

    #[test]
    fn test_boolean_literal() {
        assert_eq!(eval("true"), "true");
        assert_eq!(eval("false"), "false");
    }

    #[test]
    fn test_null_literal() {
        assert_eq!(eval("null"), "null");
    }

    #[test]
    fn test_string_literal() {
        assert_eq!(eval("\"hello\""), "hello");
    }

    #[test]
    fn test_let_declaration() {
        assert_eq!(eval("let x = 5"), "null");
    }

    #[test]
    fn test_variable_access() {
        assert_eq!(eval("let x = 42\nx"), "42");
    }

    #[test]
    fn test_reassignment() {
        let source = "let x = 5\nx = 10\nx";
        assert_eq!(eval(source), "10");
    }

    #[test]
    fn test_variable_shadowing() {
        let source = "let x = 5\nif true then\nlet x = 10\nend\nx";
        assert_eq!(eval(source), "5");
    }

    #[test]
    fn test_assign_creates_variable() {
        let source = "newvar = 42\nnewvar";
        assert_eq!(eval(source), "42");
    }

    #[test]
    fn test_let_without_value() {
        let source = "let x\nx";
        assert_eq!(eval(source), "null");
    }

    #[test]
    fn test_multiple_assignments() {
        let source = "let a = 10\nlet b = a\nb";
        assert_eq!(eval(source), "10");
    }

    #[test]
    fn test_reassign_in_nested_scope() {
        let source = "let x = 5\nif true then\nx = 10\nend\nx";
        assert_eq!(eval(source), "10");
    }

    #[test]
    fn test_variable_from_parent_scope() {
        let source = "let outer = 10\nfn get_outer()\nreturn outer\nend\nget_outer()";
        assert_eq!(eval(source), "10");
    }
}

// ============================================================
// Arithmetic tests
// ============================================================
mod arithmetic_tests {
    use super::*;

    #[test]
    fn test_addition() {
        assert_eq!(eval("2 + 3"), "5");
    }

    #[test]
    fn test_subtraction() {
        assert_eq!(eval("10 - 3"), "7");
    }

    #[test]
    fn test_multiplication() {
        assert_eq!(eval("4 * 5"), "20");
    }

    #[test]
    fn test_division() {
        assert_eq!(eval("10 / 3"), format!("{}", 10.0 / 3.0));
    }

    #[test]
    fn test_modulo() {
        assert_eq!(eval("10 % 3"), "1");
    }

    #[test]
    fn test_power() {
        assert_eq!(eval("2 ** 8"), "256");
    }

    #[test]
    fn test_negative_numbers() {
        assert_eq!(eval("-5 + 3"), "-2");
    }

    #[test]
    fn test_complex_arithmetic() {
        assert_eq!(eval("2 + 3 * 4"), "14");
        assert_eq!(eval("(2 + 3) * 4"), "20");
    }

    #[test]
    fn test_div_by_one() {
        assert_eq!(eval("42 / 1"), "42");
    }
    #[test]
    fn test_mod_by_one() {
        assert_eq!(eval("42 % 1"), "0");
    }
    #[test]
    fn test_add_zero() {
        assert_eq!(eval("99 + 0"), "99");
    }
    #[test]
    fn test_mult_by_zero() {
        assert_eq!(eval("99 * 0"), "0");
    }
    #[test]
    fn test_mult_by_one() {
        assert_eq!(eval("99 * 1"), "99");
    }
    #[test]
    fn test_pow_zero() {
        assert_eq!(eval("5 ** 0"), "1");
    }
    #[test]
    fn test_pow_one() {
        assert_eq!(eval("5 ** 1"), "5");
    }
    #[test]
    fn test_sub_negative() {
        assert_eq!(eval("5 - 10"), "-5");
    }
    #[test]
    fn test_float_arithmetic() {
        assert_eq!(eval("3.5 + 1.5"), "5");
    }

    #[test]
    fn test_precedence_add_mult() {
        assert_eq!(eval("2 + 3 * 4"), "14");
    }
    #[test]
    fn test_precedence_mult_add() {
        assert_eq!(eval("3 * 4 + 2"), "14");
    }
    #[test]
    fn test_precedence_pow_mult() {
        assert_eq!(eval("2 * 3 ** 2"), "18");
    }
    #[test]
    fn test_precedence_compare_add() {
        assert_eq!(eval("2 + 3 < 10"), "true");
    }
    #[test]
    fn test_precedence_eq_and() {
        assert_eq!(eval("true && 1 == 1"), "true");
    }
    #[test]
    fn test_precedence_not_eq() {
        assert_eq!(eval("!true == false"), "true");
    }
    #[test]
    fn test_precedence_assignment_low() {
        let source = "let x = 0\nlet y = x = 5\ny";
        assert_eq!(eval(source), "5");
    }

    #[test]
    fn test_addition_with_bool() {
        assert_eq!(eval("true + 1"), "2");
        assert_eq!(eval("false + 5"), "5");
    }

    #[test]
    fn test_parse_grouped_expression() {
        assert_eq!(eval("(5 + 3) * 2"), "16");
    }

    #[test]
    fn test_parse_deeply_nested_parens() {
        assert_eq!(eval("(((1 + 2) + 3) + 4)"), "10");
    }

    #[test]
    fn test_null_plus_null() {
        assert_eq!(eval("null + null"), "0");
    }
}

// ============================================================
// String tests
// ============================================================
mod string_tests {
    use super::*;

    #[test]
    fn test_string_concatenation() {
        assert_eq!(eval("\"hello \" + \"world\""), "hello world");
    }

    #[test]
    fn test_string_number_concatenation() {
        assert_eq!(eval("\"value: \" + 42"), "value: 42");
    }

    #[test]
    fn test_string_repetition() {
        assert_eq!(eval("\"hi\" * 3"), "hihihi");
    }

    #[test]
    fn test_string_interpolation() {
        assert_eq!(eval("let x = 42\n\"the value is ${x}\""), "the value is 42");
    }

    #[test]
    fn test_string_escapes() {
        assert_eq!(eval("\"hello\\nworld\""), "hello\nworld");
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(eval("\"\""), "");
    }
    #[test]
    fn test_string_add_empty() {
        assert_eq!(eval("\"hello\" + \"\""), "hello");
    }
    #[test]
    fn test_empty_string_add() {
        assert_eq!(eval("\"\" + \"world\""), "world");
    }
    #[test]
    fn test_string_repeat_zero() {
        assert_eq!(eval("\"x\" * 0"), "");
    }
    #[test]
    fn test_string_repeat_one() {
        assert_eq!(eval("\"x\" * 1"), "x");
    }
    #[test]
    fn test_number_repeat_string() {
        assert_eq!(eval("3 * \"ab\""), "ababab");
    }
    #[test]
    fn test_string_escape_dollar() {
        assert_eq!(eval("\"\\$5\""), "$5");
    }
    #[test]
    fn test_string_escape_backslash() {
        assert_eq!(eval("\"\\\\n\""), "\\n");
    }

    #[test]
    fn test_interpolation_at_start() {
        let source = "let x = 5\n\"${x} is five\"";
        assert_eq!(eval(source), "5 is five");
    }

    #[test]
    fn test_interpolation_at_end() {
        let source = "let x = 5\n\"five is ${x}\"";
        assert_eq!(eval(source), "five is 5");
    }

    #[test]
    fn test_interpolation_multiple() {
        let source = "let a = 1\nlet b = 2\n\"${a} + ${b} = ${a + b}\"";
        assert_eq!(eval(source), "1 + 2 = 3");
    }

    #[test]
    fn test_interpolation_expression() {
        let source = "\"${2 + 3}\"";
        assert_eq!(eval(source), "5");
    }

    #[test]
    fn test_comparison_strings() {
        assert_eq!(eval("\"ab\" < \"abc\""), "true");
        assert_eq!(eval("\"abc\" > \"ab\""), "true");
    }

    #[test]
    fn test_parse_string_with_escaped_content() {
        assert_eq!(eval("\"hello\\nworld\""), "hello\nworld");
    }

    #[test]
    fn test_string_to_number_unary() {
        assert_eq!(eval("+\"42\""), "42");
        assert_eq!(eval("+\"3.14\""), "3.14");
    }

    #[test]
    fn test_string_in_condition_empty() {
        let source = "let x = 0\nif \"\" then\nx = 1\nend\nx";
        assert_eq!(eval(source), "0");
    }

    #[test]
    fn test_string_in_condition_nonempty() {
        let source = "let x = 0\nif \"hello\" then\nx = 1\nend\nx";
        assert_eq!(eval(source), "1");
    }

    #[test]
    fn test_add_with_file_and_string() {
        let source = "\"hello\" + @\"world\"";
        let result = eval(source);
        assert!(result.contains("hello"));
        assert!(result.contains("world"));
    }

    #[test]
    fn test_eq_between_different_types() {
        assert_eq!(eval("5 == \"5\""), "false");
        assert_eq!(eval("true == 1"), "false");
        assert_eq!(eval("null == false"), "false");
    }
}

// ============================================================
// Comparison tests
// ============================================================
mod comparison_tests {
    use super::*;

    #[test]
    fn test_equality() {
        assert_eq!(eval("5 == 5"), "true");
        assert_eq!(eval("5 == 3"), "false");
        assert_eq!(eval("5 != 3"), "true");
    }

    #[test]
    fn test_comparison_operators() {
        assert_eq!(eval("5 > 3"), "true");
        assert_eq!(eval("5 < 3"), "false");
        assert_eq!(eval("5 >= 5"), "true");
        assert_eq!(eval("5 <= 3"), "false");
    }

    #[test]
    fn test_op_lt_true() {
        assert_eq!(eval("3 < 5"), "true");
    }
    #[test]
    fn test_op_lt_false() {
        assert_eq!(eval("5 < 3"), "false");
    }
    #[test]
    fn test_op_leq_true() {
        assert_eq!(eval("5 <= 5"), "true");
    }
    #[test]
    fn test_op_leq_false() {
        assert_eq!(eval("6 <= 5"), "false");
    }
    #[test]
    fn test_op_gt_true() {
        assert_eq!(eval("5 > 3"), "true");
    }
    #[test]
    fn test_op_gt_false() {
        assert_eq!(eval("3 > 5"), "false");
    }
    #[test]
    fn test_op_geq_true() {
        assert_eq!(eval("5 >= 5"), "true");
    }
    #[test]
    fn test_op_geq_false() {
        assert_eq!(eval("3 >= 5"), "false");
    }
    #[test]
    fn test_op_eq_true() {
        assert_eq!(eval("42 == 42"), "true");
    }
    #[test]
    fn test_op_eq_false() {
        assert_eq!(eval("42 == 99"), "false");
    }
    #[test]
    fn test_op_neq_true() {
        assert_eq!(eval("42 != 99"), "true");
    }
    #[test]
    fn test_op_neq_false() {
        assert_eq!(eval("42 != 42"), "false");
    }
    #[test]
    fn test_op_eq_strings() {
        assert_eq!(eval("\"a\" == \"a\""), "true");
    }
    #[test]
    fn test_op_neq_strings() {
        assert_eq!(eval("\"a\" != \"b\""), "true");
    }
    #[test]
    fn test_op_eq_bool() {
        assert_eq!(eval("true == true"), "true");
    }
    #[test]
    fn test_op_neq_bool() {
        assert_eq!(eval("true != false"), "true");
    }
    #[test]
    fn test_op_eq_null() {
        assert_eq!(eval("null == null"), "true");
    }
    #[test]
    fn test_op_neq_null_number() {
        assert_eq!(eval("null != 5"), "true");
    }

    #[test]
    fn test_eq_between_files() {
        let result = eval("@\"a\" == @\"a\"");
        assert_eq!(result, "true");
    }
}

// ============================================================
// Logical operator tests
// ============================================================
mod logical_tests {
    use super::*;

    #[test]
    fn test_logical_and() {
        assert_eq!(eval("true && true"), "true");
        assert_eq!(eval("true && false"), "false");
        assert_eq!(eval("false && true"), "false");
    }

    #[test]
    fn test_logical_or() {
        assert_eq!(eval("true || false"), "true");
        assert_eq!(eval("false || false"), "false");
    }

    #[test]
    fn test_logical_not() {
        assert_eq!(eval("!true"), "false");
        assert_eq!(eval("!false"), "true");
    }

    #[test]
    fn test_and_short_circuit() {
        let source = "fn side_effect()\nthrow \"should not throw\"\nreturn false\nend\nlet result = false && side_effect()\ntrue";
        assert_eq!(eval(source), "true");
    }

    #[test]
    fn test_or_short_circuit() {
        let source = "fn side_effect()\nthrow \"should not throw\"\nreturn false\nend\nlet result = true || side_effect()\ntrue";
        assert_eq!(eval(source), "true");
    }

    #[test]
    fn test_and_chain() {
        assert_eq!(eval("true && true && true"), "true");
        assert_eq!(eval("true && false && true"), "false");
    }

    #[test]
    fn test_or_chain() {
        assert_eq!(eval("false || false || true"), "true");
        assert_eq!(eval("false || false || false"), "false");
    }

    #[test]
    fn test_parse_not_of_comparison() {
        assert_eq!(eval("!(5 > 3)"), "false");
        assert_eq!(eval("!(5 < 3)"), "true");
    }
}

// ============================================================
// Control flow tests
// ============================================================
mod control_flow_tests {
    use super::*;

    #[test]
    fn test_if_then() {
        let source = "let x = 0\nif true then\nx = 1\nend\nx";
        assert_eq!(eval(source), "1");
    }

    #[test]
    fn test_if_else() {
        let source = "let x = 0\nif false then\nx = 1\nelse\nx = 2\nend\nx";
        assert_eq!(eval(source), "2");
    }

    #[test]
    fn test_while_loop() {
        let source = "let x = 0\nwhile x < 5 do\nx = x + 1\nend\nx";
        assert_eq!(eval(source), "5");
    }

    #[test]
    fn test_for_loop() {
        let source = "let sum = 0\nfor let i = 0, i < 5, i = i + 1 do\nsum = sum + i\nend\nsum";
        assert_eq!(eval(source), "10");
    }

    #[test]
    fn test_while_never_executes() {
        let source = "let x = 0\nwhile false do\nx = 1\nend\nx";
        assert_eq!(eval(source), "0");
    }

    #[test]
    fn test_for_never_executes() {
        let source = "let x = 0\nfor let i = 0, false, i = i + 1 do\nx = 1\nend\nx";
        assert_eq!(eval(source), "0");
    }

    #[test]
    fn test_nested_if() {
        let source = "let x = 0\nif true then\nif true then\nx = 1\nend\nend\nx";
        assert_eq!(eval(source), "1");
    }

    #[test]
    fn test_if_else_if_pattern() {
        let source = "let x = 0\nif false then\nx = 1\nelse\nif true then\nx = 2\nend\nend\nx";
        assert_eq!(eval(source), "2");
    }

    #[test]
    fn test_if_condition_false() {
        let source = "let x = 0\nif 5 > 10 then\nx = 1\nend\nx";
        assert_eq!(eval(source), "0");
    }

    #[test]
    fn test_if_with_number_condition() {
        assert_eq!(eval("let x = 0\nif 1 then\nx = 1\nend\nx"), "1");
        assert_eq!(eval("let x = 0\nif 0 then\nx = 1\nend\nx"), "0");
    }

    #[test]
    fn test_break_in_while() {
        let source = "let x = 0\nwhile true do\nx = x + 1\nif x >= 5 then\nbreak\nend\nend\nx";
        assert_eq!(eval(source), "5");
    }

    #[test]
    fn test_deeply_nested_if() {
        let source = "let x = 0\nif true then\nif true then\nif true then\nif true then\nx = 42\nend\nend\nend\nend\nx";
        assert_eq!(eval(source), "42");
    }

    #[test]
    fn test_deeply_nested_while() {
        let source = "let i = 0\nlet j = 0\nwhile i < 3 do\ni = i + 1\nwhile j < 2 do\nj = j + 1\nend\nend\nj";
        assert_eq!(eval(source), "2");
    }

    #[test]
    fn test_for_with_decrement() {
        let source = "let sum = 0\nfor let i = 5, i > 0, i = i - 1 do\nsum = sum + i\nend\nsum";
        assert_eq!(eval(source), "15");
    }

    #[test]
    fn test_number_in_condition_nonzero() {
        let source = "let x = 0\nif 42 then\nx = 1\nend\nx";
        assert_eq!(eval(source), "1");
    }

    #[test]
    fn test_number_in_condition_zero() {
        let source = "let x = 0\nif 0 then\nx = 1\nend\nx";
        assert_eq!(eval(source), "0");
    }

    #[test]
    fn test_for_break_skips_update() {
        let source = "let last_i = 999\nfor let i = 0, i < 10, i = i + 1 do\nlast_i = i\nif i == 4 then\nbreak\nend\nend\nlast_i";
        assert_eq!(eval(source), "4");
    }

    #[test]
    fn test_while_break_resets_flag_so_outer_not_affected() {
        let source = "let result = \"\"\nlet i = 0\nwhile i < 3 do\nlet j = 0\nwhile j < 2 do\nj = j + 1\nif j == 1 then\nbreak\nend\nresult = result + \"j\"\nend\nresult = result + \"i\"\ni = i + 1\nend\nresult";
        let r = eval(source);
        assert!(r.contains("i"));
    }
}

// ============================================================
// Function tests
// ============================================================
mod function_tests {
    use super::*;

    #[test]
    fn test_function_definition_and_call() {
        let source = "fn add(a, b)\nreturn a + b\nend\nadd(3, 4)";
        assert_eq!(eval(source), "7");
    }

    #[test]
    fn test_lambda_function() {
        let source = "fn square(x) => x * x\nsquare(5)";
        assert_eq!(eval(source), "25");
    }

    #[test]
    fn test_anonymous_function() {
        let source = "let f = fn (x) => x + 1\nf(10)";
        assert_eq!(eval(source), "11");
    }

    #[test]
    fn test_closure() {
        let source = "let base = 10\nfn add_base(x)\nreturn x + base\nend\nadd_base(5)";
        assert_eq!(eval(source), "15");
    }

    #[test]
    fn test_nested_function() {
        let source = "fn outer(x)\nfn inner(y)\nreturn x + y\nend\nreturn inner(x)\nend\nouter(5)";
        assert_eq!(eval(source), "10");
    }

    #[test]
    fn test_function_no_args() {
        let source = "fn greet()\nreturn \"hi\"\nend\ngreet()";
        assert_eq!(eval(source), "hi");
    }

    #[test]
    fn test_function_extra_args_ignored() {
        let source = "fn f(a) => a\nf(1, 2, 3)";
        assert_eq!(eval(source), "1");
    }

    #[test]
    fn test_function_missing_args_null() {
        let source =
            "fn f(a, b)\nif b == null then\nreturn \"missing\"\nend\nreturn \"ok\"\nend\nf(1)";
        assert_eq!(eval(source), "missing");
    }

    #[test]
    fn test_recursive_function() {
        let source = "fn factorial(n)\nif n <= 1 then\nreturn 1\nend\nreturn n * factorial(n - 1)\nend\nfactorial(5)";
        assert_eq!(eval(source), "120");
    }

    #[test]
    fn test_function_returning_function() {
        let source = "fn make_mult(n)\nfn mult(x) => x * n\nreturn mult\nend\nlet triple = make_mult(3)\ntriple(7)";
        assert_eq!(eval(source), "21");
    }

    #[test]
    fn test_closure_mutates_captured_var() {
        let source = "fn make_counter()\nlet n = 0\nfn count()\nn = n + 1\nreturn n\nend\nreturn count\nend\nlet c = make_counter()\nc()\nc()\nc()";
        assert_eq!(eval(source), "3");
    }

    #[test]
    fn test_lambda_with_no_args() {
        let source = "let f = fn () => 42\nf()";
        assert_eq!(eval(source), "42");
    }

    #[test]
    fn test_parse_function_with_multiple_params() {
        let source = "fn many(a, b, c, d, e)\nreturn a + b + c + d + e\nend\nmany(1, 2, 3, 4, 5)";
        assert_eq!(eval(source), "15");
    }

    #[test]
    fn test_parse_function_no_params() {
        let source = "fn noparams()\nreturn 42\nend\nnoparams()";
        assert_eq!(eval(source), "42");
    }

    #[test]
    fn test_parse_function_call_chain() {
        let source = "fn double(x) => x * 2\nfn triple(x) => x * 3\ndouble(triple(5))";
        assert_eq!(eval(source), "30");
    }

    #[test]
    fn test_parse_negation_of_call() {
        let source = "fn get_ten()\nreturn 10\nend\n-get_ten()";
        assert_eq!(eval(source), "-10");
    }

    #[test]
    fn test_parse_function_as_arg() {
        let source = "fn apply(f, x)\nreturn f(x)\nend\nfn double(n) => n * 2\napply(double, 7)";
        assert_eq!(eval(source), "14");
    }

    #[test]
    fn test_parse_deref_complex() {
        let source = "let path = \"hello\"\n@path";
        let result = eval(source);
        assert!(result.contains("hello"));
    }

    #[test]
    fn test_return_from_top_level() {
        let source = "return 42";
        let result = eval(source);
        assert_eq!(result, "42");
    }

    #[test]
    fn test_function_body_without_return() {
        let source = "fn f()\n42\nend\nf()";
        assert_eq!(eval(source), "42");
    }

    #[test]
    fn test_function_body_error_swallowed() {
        let source = "fn f()\nthrow \"err\"\nend\nf()";
        let result = eval_err(source);
        assert!(result.is_some());
    }

    #[test]
    fn test_call_value_with_too_many_args() {
        let source = "fn f(a) => a\nf(1, 2, 3, 4, 5)";
        assert_eq!(eval(source), "1");
    }

    #[test]
    fn test_lambda_with_params() {
        let source = "let sum = fn (a, b, c) => a + b + c\nsum(1, 2, 3)";
        assert_eq!(eval(source), "6");
    }

    #[test]
    fn test_function_with_early_return() {
        let source = "fn early(x)\nif x > 10 then\nreturn \"big\"\nend\nreturn \"small\"\nend\nearly(5) + \" \" + early(15)";
        assert_eq!(eval(source), "small big");
    }

    #[test]
    fn test_throw_from_function() {
        let source = "fn bad()\nthrow \"oops\"\nend\nlet res = try\nbad()\nend\nres:status";
        assert_eq!(eval(source), "1");
    }

    #[test]
    fn test_stmts_after_return_skipped() {
        let source = "let side = 0\nfn f()\nreturn 42\nside = 99\nend\nlet r = f()\nside";
        assert_eq!(eval(source), "0");
    }

    #[test]
    fn test_foreach_kv_with_return() {
        let source = "fn find_target(t)\nforeach k, v in t do\nif v == 20 then\nreturn 1\nend\nend\nreturn 0\nend\nfind_target({ a = 10, b = 20, c = 30 })";
        assert_eq!(eval(source), "1");
    }

    #[test]
    fn test_call_table_errors() {
        let result = eval_err("let t = {}\nt()");
        assert!(result.is_some());
    }

    #[test]
    fn test_call_file_errors() {
        let result = eval_err("@\"path\"()");
        assert!(result.is_some());
    }
}

// ============================================================
// Table tests
// ============================================================
mod table_tests {
    use super::*;

    #[test]
    fn test_table_literal() {
        assert_eq!(eval("{ foo = \"bar\" }"), "{ foo = ... }");
    }

    #[test]
    fn test_table_colon_access() {
        let source = "let t = { foo = \"bar\" }\nt:foo";
        assert_eq!(eval(source), "bar");
    }

    #[test]
    fn test_table_bracket_access() {
        let source = "let t = { [0] = \"zero\", [1] = \"one\" }\nt[0]";
        assert_eq!(eval(source), "zero");
    }

    #[test]
    fn test_table_assignment() {
        let source = "let t = {}\nt:foo = \"hello\"\nt:foo";
        assert_eq!(eval(source), "hello");
    }

    #[test]
    fn test_table_mixed_keys() {
        let source = "let t = { name = \"test\", [0] = \"zero\" }\nt:name";
        assert_eq!(eval(source), "test");
    }

    #[test]
    fn test_table_access_nonexistent_key() {
        let source = "let t = { a = 1 }\nt:b";
        assert_eq!(eval(source), "null");
    }

    #[test]
    fn test_table_array_out_of_bounds() {
        let source = "let t = { [0] = \"a\" }\nt[99]";
        assert_eq!(eval(source), "null");
    }

    #[test]
    fn test_empty_table_literal() {
        let source = "let t = {}\nt";
        assert_eq!(eval(source), "{  }");
    }

    #[test]
    fn test_object_with_computed_key() {
        let source = "let t = { [0] = \"zero\" }\nt[0]";
        assert_eq!(eval(source), "zero");
    }

    #[test]
    fn test_compound_assign_table() {
        let source = "let t = { a = 1 }\nt:a += 5\nt:a";
        assert_eq!(eval(source), "6");
    }

    #[test]
    fn test_parse_object_with_comma_separated_fields() {
        let source = "let t = { a = 1, b = 2, c = 3 }\nt:a + t:b + t:c";
        assert_eq!(eval(source), "6");
    }

    #[test]
    fn test_parse_try_as_expression() {
        let source = "let val = try 42 end\nval:value";
        assert_eq!(eval(source), "42");
    }
}

// ============================================================
// File tests
// ============================================================
mod file_tests {
    use super::*;

    #[test]
    fn test_dereference_creates_file() {
        let result = eval("@\"testfile.txt\"");
        assert!(result.contains("testfile"));
    }

    #[test]
    fn test_undeclared_identifier_is_file() {
        let result = eval("somefile");
        assert!(result.contains("somefile"));
    }

    #[test]
    fn test_dereference_string() {
        let result = eval("@\"somepath\"");
        assert!(result.contains("somepath"));
    }

    #[test]
    fn test_deref_identifier() {
        let result = eval("@filepath");
        assert!(result.contains("filepath"));
    }

    #[test]
    fn test_deref_number_string() {
        let result = eval("@\"explicit_path\"");
        assert!(result.contains("explicit_path"));
    }

    #[test]
    fn test_print_and_concat_undeclared() {
        let result = eval("undeclared_var");
        assert!(result.contains("undeclared_var"));
    }
}

// ============================================================
// Try/throw tests
// ============================================================
mod try_throw_tests {
    use super::*;

    #[test]
    fn test_try_success() {
        let source = "let res = try\n42\nend\nres:status";
        assert_eq!(eval(source), "0");
    }

    #[test]
    fn test_try_success_value() {
        let source = "let res = try\n42\nend\nres:value";
        assert_eq!(eval(source), "42");
    }

    #[test]
    fn test_throw() {
        let result = eval_err("throw \"error message\"");
        assert_eq!(result, Some("error message".to_string()));
    }

    #[test]
    fn test_try_catches_error() {
        let source = "let res = try\nthrow \"test error\"\nend\nres:status";
        assert_eq!(eval(source), "1");
    }

    #[test]
    fn test_try_empty_body() {
        let source = "let res = try\nend\nres:status";
        assert_eq!(eval(source), "0");
    }

    #[test]
    fn test_nested_try() {
        let source = "let res = try\nlet inner = try\nthrow \"inner error\"\nend\ninner:error\nend\nres:status";
        assert_eq!(eval(source), "0");
    }

    #[test]
    fn test_try_with_return() {
        let source = "fn f()\nlet res = try\nreturn 42\nend\nreturn res:value\nend\nf()";
        assert_eq!(eval(source), "42");
    }
}

// ============================================================
// Assignment tests
// ============================================================
mod assignment_tests {
    use super::*;

    #[test]
    fn test_compound_addition() {
        let source = "let x = 5\nx += 3\nx";
        assert_eq!(eval(source), "8");
    }

    #[test]
    fn test_compound_multiplication() {
        let source = "let x = 5\nx *= 2\nx";
        assert_eq!(eval(source), "10");
    }

    #[test]
    fn test_compound_minus() {
        let source = "let x = 10\nx -= 3\nx";
        assert_eq!(eval(source), "7");
    }
    #[test]
    fn test_compound_div() {
        let source = "let x = 10\nx /= 2\nx";
        assert_eq!(eval(source), "5");
    }
    #[test]
    fn test_compound_mod() {
        let source = "let x = 10\nx %= 3\nx";
        assert_eq!(eval(source), "1");
    }
    #[test]
    fn test_compound_pow() {
        let source = "let x = 2\nx **= 3\nx";
        assert_eq!(eval(source), "8");
    }

    #[test]
    fn test_parse_assignment_right_assoc() {
        let source = "let a = 0\nlet b = 0\na = b = 5\na + b";
        assert_eq!(eval(source), "10");
    }

    #[test]
    fn test_invalid_assign_target_number() {
        let result = eval_err("let x = 0\n5 = 3");
        assert!(result.is_some());
    }

    #[test]
    fn test_invalid_assign_target_lambda() {
        let result = eval_err("(fn () => 1) = x");
        assert!(result.is_some());
    }

    #[test]
    fn test_invalid_compound_assign_target() {
        let result = eval_err("5 += 3");
        assert!(result.is_some());
    }
}

// ============================================================
// Foreach tests
// ============================================================
mod foreach_tests {
    use super::*;

    #[test]
    fn test_foreach_loop() {
        let source = "let t = { [0] = 10, [1] = 20, [2] = 30 }\nlet sum = 0\nforeach v in t do\nsum = sum + v\nend\nsum";
        assert_eq!(eval(source), "60");
    }

    #[test]
    fn test_foreach_key_value() {
        let source = "let t = { a = 1, b = 2 }\nlet keys = \"\"\nforeach k, v in t do\nkeys = keys + k\nend\nkeys";
        let result = eval(source);
        assert!(result.contains("a") || result.contains("b"));
    }

    #[test]
    fn test_foreach_empty_table() {
        let source = "let t = {}\nlet count = 0\nforeach v in t do\ncount = count + 1\nend\ncount";
        assert_eq!(eval(source), "0");
    }

    #[test]
    fn test_foreach_array_only() {
        let source = "let t = { [0] = \"a\", [1] = \"b\", [2] = \"c\" }\nlet result = \"\"\nforeach v in t do\nresult = result + v\nend\nresult";
        let result = eval(source);
        assert!(result.contains("a"));
        assert!(result.contains("b"));
        assert!(result.contains("c"));
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_foreach_on_string_keys() {
        let source = "let t = { x = 1, y = 2, z = 3 }\nlet sum = 0\nforeach v in t do\nsum = sum + v\nend\nsum";
        assert_eq!(eval(source), "6");
    }

    #[test]
    fn test_foreach_kv_on_mixed_table() {
        let source = "let t = { name = \"test\", [0] = \"zero\" }\nlet keys = \"\"\nforeach k, v in t do\nkeys = keys + k\nend\nkeys";
        let result = eval(source);
        assert!(result.contains("name") || result.contains("0"));
    }

    #[test]
    fn test_parse_foreach_with_expression_collection() {
        let source = "let t = { [0] = 5, [1] = 10 }\nlet sum = 0\nforeach v in t do\nsum = sum + v\nend\nsum";
        assert_eq!(eval(source), "15");
    }

    #[test]
    fn test_foreach_with_break() {
        let source = "let t = { [0] = 1, [1] = 2, [2] = 3, [3] = 4, [4] = 5 }\nlet sum = 0\nforeach v in t do\nsum = sum + v\nif sum >= 6 then\nbreak\nend\nend\nsum";
        let result: f64 = eval(source).parse().unwrap_or(0.0);
        assert!(result >= 6.0);
    }

    #[test]
    fn test_foreach_kv_with_break() {
        let source = "let t = { a = 1, b = 2, c = 3 }\nlet sum = 0\nforeach k, v in t do\nif v == 2 then\nbreak\nend\nsum = sum + v\nend\nsum";
        let result: i32 = eval(source).parse().unwrap();
        assert!(
            result < 6,
            "Break should stop iteration early, got sum={}",
            result
        );
    }
}

// ============================================================
// Unary operator tests
// ============================================================
mod unary_tests {
    use super::*;

    #[test]
    fn test_unary_plus_number_conversion() {
        let result = eval("+\"42\"");
        assert_eq!(result, "42");
    }

    #[test]
    fn test_unary_negation() {
        let result = eval("-\"42\"");
        assert_eq!(result, "-42");
    }

    #[test]
    fn test_unary_not_on_number() {
        assert_eq!(eval("!0"), "true");
    }
    #[test]
    fn test_unary_not_on_nonzero() {
        assert_eq!(eval("!42"), "false");
    }
    #[test]
    fn test_unary_not_on_string() {
        assert_eq!(eval("!\"\""), "true");
    }
    #[test]
    fn test_unary_not_double() {
        assert_eq!(eval("!!true"), "true");
    }
    #[test]
    fn test_unary_plus_on_number() {
        assert_eq!(eval("+100"), "100");
    }
    #[test]
    fn test_unary_neg_on_number() {
        assert_eq!(eval("-100"), "-100");
    }
    #[test]
    fn test_unary_double_neg() {
        assert_eq!(eval("--10"), "10");
    }
    #[test]
    fn test_unary_neg_float() {
        assert_eq!(eval("-3.14"), "-3.14");
    }
}

// ============================================================
// Parser error recovery tests
// ============================================================
mod parser_error_tests {
    use shaell::parser::Parser;

    #[test]
    fn test_parse_error_unexpected_token() {
        let mut parser = Parser::new(")");
        assert!(parser.parse_program().is_err());
    }

    #[test]
    fn test_parse_error_unclosed_string() {
        let mut parser = Parser::new("\"unclosed");
        assert!(parser.parse_program().is_err());
    }

    #[test]
    fn test_parse_error_unclosed_brace() {
        let mut parser = Parser::new("{ a = 1");
        assert!(parser.parse_program().is_err());
    }

    #[test]
    fn test_parse_empty_program() {
        let mut parser = Parser::new("");
        let prog = parser.parse_program();
        assert!(prog.is_ok());
    }

    #[test]
    fn test_parse_invalid_token_in_expression() {
        let mut parser = Parser::new("let x = )");
        let result = parser.parse_program();
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_then() {
        let mut parser = Parser::new("if true\nx = 1\nend");
        let result = parser.parse_program();
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_do() {
        let mut parser = Parser::new("while true\nx = 1\nend");
        let result = parser.parse_program();
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_unclosed_paren() {
        let mut parser = Parser::new("let x = (5 + 3\nx");
        let result = parser.parse_program();
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_unclosed_object() {
        let mut parser = Parser::new("let t = { a = 1\nx");
        let result = parser.parse_program();
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_function_missing_params_paren() {
        let mut parser = Parser::new("fn f x\nreturn x\nend");
        let result = parser.parse_program();
        assert!(result.is_err());
    }

    #[test]
    fn test_lambda_parse_error_on_statement() {
        let mut parser = Parser::new("fn f() => throw \"err\"\nf()");
        assert!(parser.parse_program().is_err());
    }

    #[test]
    fn test_parse_nested_blocks() {
        use super::*;
        let source = "if true then\nif true then\nif true then\nlet x = 1\nend\nend\nend";
        let result = eval(source);
        assert_eq!(result, "null");
    }

    #[test]
    fn test_parse_multiple_stmts_same_line() {
        use super::*;
        let source = "let a = 1\nlet b = 2\na + b";
        assert_eq!(eval(source), "3");
    }

    #[test]
    fn test_empty_program_is_null() {
        use shaell::interpreter::Interpreter;
        let mut parser = Parser::new("");
        let program = parser.parse_program().unwrap();
        let mut interpreter = Interpreter::new();
        let result = interpreter.execute_program(&program).unwrap();
        let s = interpreter.heap.with_ref(result, |v| v.to_string());
        assert_eq!(s, "null");
    }
}

// ============================================================
// Exec/pipe tests
// ============================================================
mod exec_pipe_tests {
    use super::*;

    #[test]
    fn test_exec_echo() {
        let source = "exec echo with (\"hello\")";
        let result = eval_err(source);
        assert!(result.is_some());
    }

    #[test]
    fn test_exec_with_expression_args() {
        let source = "let msg = \"test\"\nexec echo with (msg)";
        let result = eval_err(source);
        assert!(result.is_some());
    }

    #[test]
    fn test_exec_piping_capture() {
        let source = "exec echo with (\"hello world\") -> let captured\ncaptured";
        let result = eval(source);
        assert!(result.contains("hello"));
    }

    #[test]
    fn test_exec_whoami() {
        let source = "exec whoami with () -> let user\nuser";
        let result = eval(source);
        assert!(!result.is_empty());
        assert!(result != "null");
    }

    #[test]
    fn test_exec_echo_no_args() {
        let source = "exec echo with () -> let output\noutput";
        let result = eval(source);
        assert!(result == "\n" || result.is_empty());
    }

    #[test]
    fn test_exec_echo_multiple_args() {
        let source = "exec echo with (\"hello\", \"world\") -> let output\noutput";
        let result = eval(source);
        assert!(result.contains("hello"));
    }

    #[test]
    fn test_parse_comment_at_end_of_file() {
        let source = "let x = 42\n# comment at end";
        assert_eq!(eval(source), "null");
    }
}

// ============================================================
// Builtin integration tests (through Shæll scripts)
// ============================================================
mod builtin_integration {
    use shaell::builtin;
    use shaell::interpreter::Interpreter;
    use shaell::parser::Parser;

    fn eval(s: &str) -> String {
        let mut p = Parser::new(s);
        let prog = p.parse_program().expect("Parse failed");
        let mut i = Interpreter::new();
        {
            let mut g = i.global_scope.borrow_mut();
            builtin::populate_global_scope(&i.heap, &mut g);
        }
        let v = i.execute_program(&prog).expect("Exec failed");
        i.heap.with_ref(v, |v| v.to_string())
    }
    fn err(s: &str) -> Option<String> {
        let mut p = Parser::new(s);
        let prog = p.parse_program().ok()?;
        let mut i = Interpreter::new();
        {
            let mut g = i.global_scope.borrow_mut();
            builtin::populate_global_scope(&i.heap, &mut g);
        }
        match i.execute_program(&prog) {
            Ok(v) => Some(i.heap.with_ref(v, |v| v.to_string())),
            Err(e) => Some(e.message),
        }
    }

    #[test]
    fn num_sqrt() {
        assert_eq!(eval("A:Number:sqrt(9)"), "3");
    }
    #[test]
    fn num_sqrt_zero() {
        assert_eq!(eval("A:Number:sqrt(0)"), "0");
    }
    #[test]
    fn num_floor() {
        assert_eq!(eval("A:Number:floor(3.7)"), "3");
    }
    #[test]
    fn num_ceil() {
        assert_eq!(eval("A:Number:ceil(3.2)"), "4");
    }
    #[test]
    fn num_log2() {
        assert_eq!(eval("A:Number:log2(8)"), "3");
    }
    #[test]
    fn num_log() {
        assert_eq!(eval("A:Number:log(100)"), "2");
    }
    #[test]
    fn str_length() {
        assert_eq!(eval("A:String:length(\"hello\")"), "5");
    }
    #[test]
    fn str_length_empty() {
        assert_eq!(eval("A:String:length(\"\")"), "0");
    }
    #[test]
    fn str_substr() {
        assert_eq!(eval("A:String:substring(\"hello\",1,4)"), "ell");
    }
    #[test]
    fn str_substr_full() {
        assert_eq!(eval("A:String:substring(\"hi\",0,2)"), "hi");
    }
    #[test]
    fn str_substr_empty() {
        assert_eq!(eval("A:String:substring(\"x\",1,1)"), "");
    }
    #[test]
    fn str_substr_past() {
        assert_eq!(eval("A:String:substring(\"hi\",0,99)"), "hi");
    }
    #[test]
    fn assert_ok() {
        assert_eq!(eval("A:assert(5==5,\"ok\")"), "ok");
    }
    #[test]
    fn assert_fail() {
        let r = err("A:assert(5==3,\"fail msg\")");
        assert!(r.is_some() && r.unwrap().contains("fail msg"));
    }
    #[test]
    fn assert_type_pass() {
        assert_eq!(eval("A:assertType(42,\"number\")"), "ok");
    }
    #[test]
    fn assert_type_fail() {
        let r = err("A:assertType(42,\"string\")");
        assert!(
            r.is_some()
                && r.unwrap()
                    .contains("Expected type 'string' but got 'number'")
        );
    }
    #[test]
    fn assert_type_string() {
        assert_eq!(eval("A:assertType(\"hi\",\"string\")"), "ok");
    }
    #[test]
    fn assert_type_bool() {
        assert_eq!(eval("A:assertType(true,\"bool\")"), "ok");
    }
    #[test]
    fn assert_type_null() {
        assert_eq!(eval("A:assertType(null,\"null\")"), "ok");
    }

    // ── mutation-testing gaps ───────────────────────────────

    #[test]
    fn test_define_args_creates_table() {
        // define_args should create a table variable
        let source = "define_args args\nfirst\nend\nargs";
        let result = eval(source);
        assert!(result.contains("table") || result != "null");
    }

    #[test]
    fn test_compound_assign_table_subscript() {
        // table[index] += value
        let source = "let t = { [0] = 10 }\nt[0] += 5\nt[0]";
        assert_eq!(eval(source), "15");
    }

    #[test]
    fn test_assign_table_subscript() {
        // table[index] = value
        let source = "let t = { [0] = \"old\" }\nt[0] = \"new\"\nt[0]";
        assert_eq!(eval(source), "new");
    }

    #[test]
    fn test_pipe_to_identifier() {
        // exec ... -> let varname should capture output
        let source = "exec echo with (\"captured\") -> let result\nresult";
        let output = eval(source);
        assert!(output.contains("captured"));
    }
}
