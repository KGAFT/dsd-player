use chardetng::EncodingDetector;


pub fn binary_to_text(binary: &[u8]) -> String {
    if binary.starts_with(&[0xFF, 0xFE]) {
        let (text, _, _) = encoding_rs::UTF_16LE.decode(&binary[2..]);
        return text.into_owned()
    }
    if binary.starts_with(&[0xFE, 0xFF]) {
        let (text, _, _) = encoding_rs::UTF_16BE.decode(&binary[2..]);
        return text.into_owned()
    }

    let mut detector = EncodingDetector::new();
    detector.feed(binary, true);
    let encoding = detector.guess(None, true);

    // Decode with replacement for invalid sequences
    let (decoded, _, had_errors) = encoding.decode(binary);

    let text = if had_errors {
        decoded.chars().filter(|c| *c != '\u{FFFD}').collect()
    } else {
        decoded.to_string()
    };



    text
}