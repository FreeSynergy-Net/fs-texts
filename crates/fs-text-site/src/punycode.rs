//! Punycode (RFC 3492) decoding, only to know whether the part after `xn--` is real Punycode.
//!
//! Pattern: none (one pure function). fs-release checks every `xn--` label with the Python
//! codec; a label that does not decode is a name nobody resolves. The same rule here keeps
//! the two sides equal. Mirrors the strict mode of that codec.

const BASE: u32 = 36;
const T_MIN: u32 = 1;
const T_MAX: u32 = 26;
const SKEW: u32 = 38;
const DAMP: u32 = 700;
const LAST: u64 = 0x0010_FFFF;

fn adapt(mut delta: u32, first: bool, count: u32) -> u32 {
    delta = if first { delta / DAMP } else { delta / 2 };
    delta += delta / count;
    let mut k = 0;
    while delta > ((BASE - T_MIN) * T_MAX) / 2 {
        delta /= BASE - T_MIN;
        k += BASE;
    }
    k + (BASE - T_MIN + 1) * delta / (delta + SKEW)
}

fn digit(b: u8) -> Option<u32> {
    match b {
        b'a'..=b'z' => Some(u32::from(b - b'a')),
        b'A'..=b'Z' => Some(u32::from(b - b'A')),
        b'0'..=b'9' => Some(u32::from(b - b'0') + 26),
        _ => None,
    }
}

/// One generalized variable-length number at `at` in `ext`: `(value, next position)`.
fn number(ext: &[u8], mut at: usize, bias: u32) -> Option<(u64, usize)> {
    let (mut weight, mut value) = (1_u64, 0_u64);
    for step in 0_u32.. {
        let dig = u64::from(ext.get(at).copied().and_then(digit)?); // none: cut off or no digit
        at += 1;
        let thr = (i64::from(BASE) * i64::from(step + 1) - i64::from(bias))
            .clamp(i64::from(T_MIN), i64::from(T_MAX));
        let thr = u64::try_from(thr).ok()?;
        value = value.checked_add(dig.checked_mul(weight)?)?;
        if value >= 1 << 40 {
            return None; // far beyond the last code point: refused like the codec does
        }
        if dig < thr {
            return Some((value, at));
        }
        weight = weight.checked_mul(u64::from(BASE) - thr)?;
    }
    None
}

/// Does `label` (the part after `xn--`) decode as Punycode?
#[must_use]
pub(crate) fn decodes(label: &str) -> bool {
    let bytes = label.as_bytes();
    let (basic, ext) = match bytes.iter().rposition(|c| *c == b'-') {
        Some(cut) => (cut, &bytes[cut + 1..]),
        None => (0, bytes),
    };
    let mut chars = u64::try_from(basic).unwrap_or(u64::MAX);
    let (mut code, mut pos, mut bias, mut at) = (0x80_u64, -1_i64, 72_u32, 0_usize);
    while at < ext.len() {
        let Some((delta, next)) = number(ext, at, bias) else {
            return false;
        };
        let Ok(delta_i) = i64::try_from(delta) else {
            return false;
        };
        pos += delta_i + 1;
        let size = i64::try_from(chars).unwrap_or(i64::MAX) + 1;
        code += u64::try_from(pos / size).unwrap_or(u64::MAX);
        if code > LAST {
            return false;
        }
        pos %= size;
        chars += 1;
        bias = adapt(
            u32::try_from(delta).unwrap_or(u32::MAX),
            at == 0,
            u32::try_from(chars).unwrap_or(u32::MAX),
        );
        at = next;
    }
    true
}
