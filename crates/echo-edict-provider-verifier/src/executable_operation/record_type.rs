// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Independent verifier parsing of canonical structural record references.
//! The caller debits the coordinate's bytes before this bounded scan.

pub(super) fn fields(mut input: &str) -> Result<Vec<(&str, &str)>, ()> {
    let mut fields = Vec::new();
    while !input.is_empty() {
        let mut level = 0_i32;
        let mut end = input.len();
        for (offset, character) in input.char_indices() {
            match character {
                '<' => level += 1,
                '>' => level -= 1,
                ',' if level == 0 => {
                    end = offset;
                    break;
                }
                _ => {}
            }
            if !(0..=64).contains(&level) {
                return Err(());
            }
        }
        if level != 0 {
            return Err(());
        }
        let (name, ty) = input[..end].split_once(':').ok_or(())?;
        if name.is_empty()
            || ty.is_empty()
            || !name.bytes().enumerate().all(|(i, byte)| {
                byte == b'_' || byte.is_ascii_alphabetic() || (i != 0 && byte.is_ascii_digit())
            })
            || fields.last().is_some_and(|(last, _)| *last >= name)
        {
            return Err(());
        }
        fields.push((name, ty));
        if end == input.len() {
            break;
        }
        input = &input[end + 1..];
        if input.is_empty() {
            return Err(());
        }
    }
    Ok(fields)
}
