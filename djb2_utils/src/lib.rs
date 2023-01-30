//! Nvidia ALF hash functions.


/// The initial seed used for djb2 hashes.
pub const DJB2_HASH_SEED: u32 = 0x1505;


/// Calculate the djb2 hash of a bytestring, with configurable starting
/// seed. (Use hash_djb2_default() if you want to start with the default
/// seed.)
#[allow(dead_code)]
#[inline(always)]
pub fn hash_djb2(s: &[u8], seed: u32) -> u32 {
    let mut hash = seed;
    for c in s {
        hash = hash.overflowing_mul(33).0 ^ (*c as u32);
    }
    hash
}


/// Calculate the djb2 hash of a bytestring, starting with the default
/// seed.
#[allow(dead_code)]
#[inline(always)]
pub fn hash_djb2_default(s: &[u8]) -> u32 {
    hash_djb2(s, DJB2_HASH_SEED)
}


/// "Undo" a suffix off of a djb2 hash value.
#[allow(dead_code)]
#[inline(always)]
pub fn invhash_djb2(s: &[u8], seed: u32) -> u32 {
    let mut hash = seed;
    for c in s.iter().rev() {
        // Magic number: multiplicative inverse of 33 mod 32
        hash = (hash ^ (*c as u32)).overflowing_mul(1041204193).0;
    }
    hash
}


/// "Undo" a numeric suffix off of a djb2 hash value.
/// Returns the new hash, and the number of characters that were undone
/// (this is in case you have *nested* length prefixes if needed) plus
/// num_chars_seed.
///
/// This is equivalent to
/// `(invhash_djb2(&value.to_string().to_bytes()[..], seed), num_chars_seed + value.to_string().len())`
#[allow(dead_code)]
#[inline(always)]
pub fn invhash_djb2_int(mut value: usize, seed: u32, length_seed: usize) -> (u32, usize) {
    let mut hash = seed;
    let mut length = length_seed;
    loop {
        // Magic number "48": ASCII value of '0'
        // Other magic number: multiplicative inverse of 33 mod 32
        hash = (hash ^ (((value % 10) + 48) as u32)).overflowing_mul(1041204193).0;
        value /= 10;
        length += 1;
        if value == 0 { break; }
    }
    (hash, length)
}


#[cfg(test)]
mod tests {
    use super::*;

    use anyhow::Result;

    #[test]
    fn test_hash_djb2() -> Result<()> {
        assert_eq!(hash_djb2(b"", 0x12345678), 0x12345678);
        assert_eq!(hash_djb2(b"mario", 0x12345678), 0x3f55d800);
        Ok(())
    }

    #[test]
    fn test_hash_djb2_default() -> Result<()> {
        assert_eq!(hash_djb2_default(b""), DJB2_HASH_SEED);
        assert_eq!(hash_djb2_default(b"mario"), 0x0a6729dd);
        Ok(())
    }

    #[test]
    fn test_invhash_djb2() -> Result<()> {
        assert_eq!(invhash_djb2(b"", 0x12345678), 0x12345678);
        assert_eq!(invhash_djb2(b"mario", 0x3f55d800), 0x12345678);
        assert_eq!(invhash_djb2(b"mario", 0x0a6729dd), DJB2_HASH_SEED);
        Ok(())
    }

    #[test]
    fn test_invhash_djb2_int() -> Result<()> {
        assert_eq!(invhash_djb2_int(0, 0x12345678, 0), (invhash_djb2(b"0", 0x12345678), 1));
        assert_eq!(invhash_djb2_int(1, 0x12345678, 1), (invhash_djb2(b"1", 0x12345678), 2));
        assert_eq!(invhash_djb2_int(9, 0x12345678, 2), (invhash_djb2(b"9", 0x12345678), 3));
        assert_eq!(invhash_djb2_int(10, 0x12345678, 0), (invhash_djb2(b"10", 0x12345678), 2));
        assert_eq!(invhash_djb2_int(11, 0x12345678, 1), (invhash_djb2(b"11", 0x12345678), 3));
        assert_eq!(invhash_djb2_int(99, 0x12345678, 2), (invhash_djb2(b"99", 0x12345678), 4));
        assert_eq!(invhash_djb2_int(100, 0x12345678, 0), (invhash_djb2(b"100", 0x12345678), 3));
        assert_eq!(invhash_djb2_int(101, 0x12345678, 1), (invhash_djb2(b"101", 0x12345678), 4));
        Ok(())
    }
}
