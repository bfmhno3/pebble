use lab_ch15_tests::bump;

#[test]
fn integration_tests_use_the_public_api() {
    assert_eq!(bump(1), 2);
}
