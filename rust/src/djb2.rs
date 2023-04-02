/// Implements the djb2 hash function for a string.
///
/// The djb2 hash function is a simple and efficient hash function that produces
/// good hash values for short strings.
pub fn djb2_hash<T: AsRef<str>>(string: T) -> u32 {
    // Convert the string to a byte slice.
    let string = string.as_ref().as_bytes();

    // Initialize the hash value to zero.
    let mut hash: u32 = 0;

    // Iterate over each byte in the string and update the hash value.
    for c in string {
        // Update the hash value using the djb2 algorithm.
        hash = ((hash << 5).wrapping_sub(hash)) + *c as u32;
    }

    // Return the final hash value.
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_djb2_hash_1() {
        let hashed_value = djb2_hash("m50_50");
        let assert_val = -1123920270;

        assert_eq!(hashed_value, assert_val as u32);
    }

    #[test]
    fn test_djb2_2() {
        let hashed_value = djb2_hash("huffman");
        let assert_val = 1258058669;

        assert_eq!(hashed_value, assert_val as u32);
    }
}
