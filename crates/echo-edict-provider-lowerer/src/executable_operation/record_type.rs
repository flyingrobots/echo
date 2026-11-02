// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Lowerer-owned grammar for structural record coordinates. Callers charge bytes
//! before scanning and recursively resolve fields under their existing work bound.

pub(super) fn fields(spec: &str) -> Result<Vec<(&str, &str)>, ()> {
    if spec.is_empty() {
        return Ok(Vec::new());
    }
    let mut result = Vec::new();
    let mut start = 0;
    let mut nesting = 0_usize;
    for (index, byte) in spec.bytes().chain(std::iter::once(b',')).enumerate() {
        match byte {
            b'<' => {
                nesting = nesting.checked_add(1).ok_or(())?;
                if nesting > 64 {
                    return Err(());
                }
            }
            b'>' => nesting = nesting.checked_sub(1).ok_or(())?,
            b',' if nesting == 0 => {
                let (key, coordinate) = spec[start..index].split_once(':').ok_or(())?;
                let mut letters = key.bytes();
                if !letters
                    .next()
                    .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
                    || !letters.all(|b| b.is_ascii_alphanumeric() || b == b'_')
                    || coordinate.is_empty()
                    || result.last().is_some_and(|(previous, _)| *previous >= key)
                {
                    return Err(());
                }
                result.push((key, coordinate));
                start = index + 1;
            }
            _ => {}
        }
    }
    if nesting != 0 {
        return Err(());
    }
    Ok(result)
}
