//! Compile-failure tests (trybuild) — verify the proc-macro rejects
//! malformed YAML with `compile_error!`.

#[test]
fn compile_fail_cases() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/trybuild/missing_journey_id.rs");
    t.compile_fail("tests/trybuild/bad_journey_id_case.rs");
    t.compile_fail("tests/trybuild/bad_traces_to.rs");
    t.compile_fail("tests/trybuild/bad_family.rs");
    t.compile_fail("tests/trybuild/yaml_syntax_error.rs");
}
