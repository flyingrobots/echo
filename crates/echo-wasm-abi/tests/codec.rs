use echo_wasm_abi::codec::{Reader, Writer, fx_from_i64, fx_from_f32, vec3_fx_from_i64, vec3_fx_from_f32};

#[test]
fn codec_round_trip_scalars_and_string() {
    let mut w = Writer::with_capacity(64);
    w.write_u32_le(42);
    w.write_i64_le(-123);
    w.write_string("hello", 1024).unwrap();
    let bytes = w.into_vec();

    let mut r = Reader::new(&bytes);
    let a = r.read_u32_le().unwrap();
    let b = r.read_i64_le().unwrap();
    let c = r.read_string(1024).unwrap();

    assert_eq!(a, 42);
    assert_eq!(b, -123);
    assert_eq!(c, "hello");
}

#[test]
fn codec_fx_helpers() {
    assert_eq!(fx_from_i64(1), 1i64 << 32);
    assert_eq!(vec3_fx_from_i64(1, -2, 3), [1i64 << 32, -2i64 << 32, 3i64 << 32]);
    assert_eq!(fx_from_f32(1.5), (1i64 << 32) + (1i64 << 31));
    assert_eq!(vec3_fx_from_f32(1.0, -2.5, 3.0), [1i64 << 32, -2i64 << 32 - (1i64 << 31), 3i64 << 32]);
}
