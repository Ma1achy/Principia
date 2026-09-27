//! Fixture: the `discriminating` fixture with one doctest added. `negative_control!` cannot name a doctest, so the
//! command fails naming it (REQ-VAL-147; applied per R-204).

/// Doubles `x`.
///
/// ```
/// assert_eq!(controls_doctest::double(3), 6);
/// ```
pub fn double(x: u32) -> u32 {
    x * 2
}
