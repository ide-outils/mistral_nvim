const MAX_LEN: usize = 10;

/// Convertit une chaîne de lettres (A-Z, a-z, 0-9) en `usize` en utilisant 6 bits par caractère.
/// Tronque à 10 caractères pour éviter les collisions sur 64 bits (10 * 6 < 64).
pub fn tool_id_to_usize(s: &str) -> usize {
    let mut result = 0;
    for (i, c) in s.chars().take(MAX_LEN).enumerate() {
        let value = match c {
            // A-Z : 1-26
            'A'..='Z' => (c as usize) - ('A' as usize) + 1,
            // a-z : 27-52
            'a'..='z' => (c as usize) - ('a' as usize) + 1 + 26,
            // 0-9 : 53-62
            '0'..='9' => (c as usize) - ('0' as usize) + 1 + 26 + 26,
            _ => 0,
        };
        // Décale de 6 bits par caractère (61 < 2^6)
        result += value << (i * 6);
    }
    result
}
pub fn usize_to_tool_id(mut s: usize) -> String {
    let mut result = String::with_capacity(MAX_LEN);
    for _ in 0..MAX_LEN {
        // On extrait les 6 bits les moins significatifs
        let value = (s & 0b111111) as u8;
        s >>= 6;
        // On convertit la valeur en caractère
        let c = match value {
            1..=26 => (value - 1 + b'A') as char,
            27..=52 => (value - 27 + b'a') as char,
            53..=62 => (value - 53 + b'0') as char,
            _ => break, // Si la valeur est 0, on arrête (caractère vide)
        };
        result.push(c);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tool_id_conversion() {
        let id = "Azd3fdFs";
        let id_usize = tool_id_to_usize(id);
        let id_back = usize_to_tool_id(id_usize);
        assert_eq!(id_back, id);
    }
    #[test]
    fn tool_id_conversion_digit() {
        let id = "0";
        let id_usize = tool_id_to_usize(id);
        assert_eq!(id_usize, 53);
        let id_back = usize_to_tool_id(id_usize);
        assert_eq!(id_back, id);
    }
}
