/// Adds one.
///
/// ```
/// assert_eq!(lab_ch15_tests::bump(1), 2);
/// ```
pub fn bump(x: u32) -> u32 {
    secret(x) + 1
}

fn secret(x: u32) -> u32 {
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_tests_see_private_items() {
        assert_eq!(secret(5), 5);
    }
}
