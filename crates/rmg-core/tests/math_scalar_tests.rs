#![allow(missing_docs)]
pub use rmg_core::math::scalar::F32Scalar;

#[test]
fn test_f32_basics() {
    // constants
    let zero = F32Scalar::zero();
    let one = F32Scalar::one();
    assert_eq!(zero.0, 0.0);
    assert_eq!(one.0, 1.0);

    // basic math
    let a = F32Scalar(5.0);
    let b = F32Scalar(2.0);

    assert_eq!((a + b), 7.0);
    assert_eq!((a - b), 3.0);
    assert_eq!((a * b), 10.0);
    assert_eq!((a / b), 2.5);

    let angle = F32Scalar(std::f32::consts::PI);
    assert_eq!(angle.sin().0, angle.0.sin());
    assert_eq!(angle.cos().0, angle.0.cos());
}
