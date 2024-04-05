use std::iter::repeat;

pub fn replace_newline(text: &mut String, replacement: &str) {
    if replacement.is_empty() {
        text.retain(|c| c != '\n');
        return;
    }
    let newline_count = text.chars().filter(|&c| c == '\n').count();
    let additional_len = (replacement.len() - 1) * newline_count;
    text.reserve(additional_len);
    text.extend(repeat('\0').take(additional_len));

    let mut dest = text.len();
    let mut src = text.len() - additional_len;

    unsafe {
        let buffer = text.as_bytes_mut();
        while dest > src {
            src -= 1;
            let byte = buffer[src];
            if byte == b'\n' {
                dest -= replacement.len();
                buffer[dest..dest + replacement.len()].copy_from_slice(replacement.as_bytes());
            } else {
                dest -= 1;
                buffer[dest] = byte;
            }
        }
    }
}
