//! Splits `haystack` at the first occurrence of `needle`, returning None if no needle was found
pub fn split_first(haystack: &[u8], needle: u8) -> Option<(&[u8], &[u8])> {
    if let Some((index, _)) = haystack.iter().enumerate().find(|(_, &c)| c == needle) {
        let (key, haystack) = haystack.split_at(index);

        Some((key, &haystack[1..]))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_first() {
        assert_eq!(split_first(b"", b'/'), None);
        assert_eq!(split_first(b"non-empty", b'/'), None);
        assert_eq!(split_first(b"/", b'/'), Some((&b""[..], &b""[..])));
        assert_eq!(split_first(b"abc/", b'/'), Some((&b"abc"[..], &b""[..])));
        assert_eq!(split_first(b"/defg", b'/'), Some((&b""[..], &b"defg"[..])));
        assert_eq!(
            split_first(b"abc/defg", b'/'),
            Some((&b"abc"[..], &b"defg"[..]))
        );
        assert_eq!(
            split_first(b"ab/cde/fg", b'/'),
            Some((&b"ab"[..], &b"cde/fg"[..]))
        );
    }
}
