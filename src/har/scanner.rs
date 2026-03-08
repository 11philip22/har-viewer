use super::types::{EntryRange, HarError};

pub fn scan_entry_ranges(bytes: &[u8]) -> Result<Vec<EntryRange>, HarError> {
    let (entries_start, entries_end) = find_entries_array(bytes)?;
    let mut i = entries_start + 1;
    let mut ranges = Vec::new();

    while i < entries_end {
        skip_ws_and_commas(bytes, &mut i);
        if i >= entries_end || bytes[i] == b']' {
            break;
        }

        if bytes[i] != b'{' {
            return Err(HarError::InvalidEntriesShape);
        }

        let end = match_delimited(bytes, i, b'{', b'}')?;
        ranges.push(EntryRange { start: i, end });
        i = end;
        skip_ws_and_commas(bytes, &mut i);
    }

    Ok(ranges)
}

pub fn find_entries_array(bytes: &[u8]) -> Result<(usize, usize), HarError> {
    ensure_utf8(bytes)?;

    let mut i = 0;
    skip_ws(bytes, &mut i);
    if i >= bytes.len() || bytes[i] != b'{' {
        return Err(HarError::InvalidEntriesShape);
    }

    let log_value_start = find_field_value_start(bytes, i, "log")?;
    if bytes.get(log_value_start) != Some(&b'{') {
        return Err(HarError::InvalidEntriesShape);
    }

    let entries_value_start = find_field_value_start(bytes, log_value_start, "entries")?;
    if bytes.get(entries_value_start) != Some(&b'[') {
        return Err(HarError::InvalidEntriesShape);
    }

    let entries_value_end = match_delimited(bytes, entries_value_start, b'[', b']')?;
    Ok((entries_value_start, entries_value_end))
}

fn ensure_utf8(bytes: &[u8]) -> Result<(), HarError> {
    std::str::from_utf8(bytes)
        .map(|_| ())
        .map_err(|_| HarError::InvalidUtf8)
}

fn find_field_value_start(
    bytes: &[u8],
    object_start: usize,
    wanted_key: &str,
) -> Result<usize, HarError> {
    if bytes.get(object_start) != Some(&b'{') {
        return Err(HarError::InvalidEntriesShape);
    }

    let mut i = object_start + 1;
    loop {
        skip_ws(bytes, &mut i);
        match bytes.get(i) {
            Some(b'}') => return Err(HarError::MissingEntries),
            Some(b',') => {
                i += 1;
                continue;
            }
            Some(b'"') => {
                let (key, end) = parse_json_string(bytes, i)?;
                i = end;
                skip_ws(bytes, &mut i);
                if bytes.get(i) != Some(&b':') {
                    return Err(HarError::InvalidEntriesShape);
                }
                i += 1;
                skip_ws(bytes, &mut i);
                if key == wanted_key {
                    return Ok(i);
                }
                i = skip_json_value(bytes, i)?;
            }
            Some(_) => return Err(HarError::InvalidEntriesShape),
            None => return Err(HarError::InvalidEntriesShape),
        }
    }
}

fn skip_json_value(bytes: &[u8], start: usize) -> Result<usize, HarError> {
    let Some(ch) = bytes.get(start) else {
        return Err(HarError::InvalidEntriesShape);
    };

    let end = match ch {
        b'"' => parse_json_string(bytes, start)?.1,
        b'{' => match_delimited(bytes, start, b'{', b'}')?,
        b'[' => match_delimited(bytes, start, b'[', b']')?,
        _ => {
            let mut i = start;
            while i < bytes.len() {
                match bytes[i] {
                    b',' | b'}' | b']' => break,
                    b' ' | b'\t' | b'\r' | b'\n' => break,
                    _ => i += 1,
                }
            }
            i
        }
    };

    Ok(end)
}

fn parse_json_string(bytes: &[u8], start: usize) -> Result<(String, usize), HarError> {
    if bytes.get(start) != Some(&b'"') {
        return Err(HarError::InvalidEntriesShape);
    }

    let mut i = start + 1;
    let mut escaped = false;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if !escaped => escaped = true,
            b'"' if !escaped => {
                let raw = &bytes[start..=i];
                let key = serde_json::from_slice::<String>(raw)?;
                return Ok((key, i + 1));
            }
            _ => escaped = false,
        }
        i += 1;
    }

    Err(HarError::InvalidEntriesShape)
}

fn match_delimited(bytes: &[u8], start: usize, open: u8, close: u8) -> Result<usize, HarError> {
    if bytes.get(start) != Some(&open) {
        return Err(HarError::InvalidEntriesShape);
    }

    let mut i = start;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    while i < bytes.len() {
        let ch = bytes[i];
        if in_string {
            match ch {
                b'\\' if !escaped => escaped = true,
                b'"' if !escaped => in_string = false,
                _ => escaped = false,
            }
            i += 1;
            continue;
        }

        match ch {
            b'"' => in_string = true,
            x if x == open => depth += 1,
            x if x == close => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Ok(i + 1);
                }
            }
            _ => {}
        }

        i += 1;
    }

    Err(HarError::InvalidEntriesShape)
}

fn skip_ws(bytes: &[u8], i: &mut usize) {
    while *i < bytes.len() {
        match bytes[*i] {
            b' ' | b'\t' | b'\r' | b'\n' => *i += 1,
            _ => break,
        }
    }
}

fn skip_ws_and_commas(bytes: &[u8], i: &mut usize) {
    while *i < bytes.len() {
        match bytes[*i] {
            b' ' | b'\t' | b'\r' | b'\n' | b',' => *i += 1,
            _ => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{find_entries_array, scan_entry_ranges};

    #[test]
    fn finds_entries_and_ranges() {
        let har = br#"{"log":{"entries":[{"id":1},{"id":2}]}}"#;
        let (start, end) = find_entries_array(har).expect("entries array");
        assert!(start < end);

        let ranges = scan_entry_ranges(har).expect("ranges");
        assert_eq!(ranges.len(), 2);
        assert_eq!(&har[ranges[0].start..ranges[0].end], br#"{"id":1}"#);
    }

    #[test]
    fn handles_escaped_quotes_and_braces_inside_strings() {
        let har = br#"{"log":{"entries":[{"msg":"a { brace and \"quote\""},{"msg":"b"}]}}"#;
        let ranges = scan_entry_ranges(har).expect("ranges");
        assert_eq!(ranges.len(), 2);
    }
}
