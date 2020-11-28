
pub fn hash_fnv1a(data: &[u8]) -> u64 {
    const FNV_PRIME: u64 = 1099511628211;
    const FNV_OFFSET_BASIS: u64 = 14695981039346656037;

    let mut hash = FNV_OFFSET_BASIS;
    for byte in data {
        hash = hash ^ *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    return hash;
}

pub fn filepath_hash(data: String) -> u64 {
    let lowercase_salted = format!("{}++", data.to_lowercase());
    return hash_fnv1a(lowercase_salted.as_bytes());
}