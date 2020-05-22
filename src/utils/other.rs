pub fn cutoff_on_last_dot(text: &str, length: usize) -> &str {
    let mut last: usize = 0;
    for (index, character) in text.chars().enumerate() {
        if character == '.' {
            last = index
        } else if index >= length - 1 {
            if last != 0 {
                return &text[..(last + 1)];
            } else {
                return &text[..length];
            }
        }
    }
    return text;
}
