pub fn unique_values<'a, I>(iter: I) -> usize
where
    I: std::iter::Iterator<Item = &'a String>,
{
    let mut usernames = iter.collect::<Vec<_>>();
    usernames.sort_unstable();
    usernames.dedup();
    usernames.len()
}

/// Converts an iterator of &str to an  ``--Format`` argument
pub fn format_string<'a, I, S>(iter: I) -> String
where
    I: Iterator<Item = &'a S>,
    S: ?Sized + AsRef<str> + 'a,
{
    iter
        // Remove limit on field length (defaults to 20)
        .map(|v| format!("{}:0", v.as_ref()))
        .collect::<Vec<_>>()
        // Join fields by a character that does not potentially appear in values
        .join("|,")
}
