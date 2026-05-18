use crate::editor::error::EditorError;

#[derive(Debug, Clone, Copy)]
pub enum Addr {
    Current,
    Last,
    Absolute(usize),
}

#[derive(Debug, Clone, Copy)]
pub struct AddressRange {
    pub start: Addr,
    pub end: Addr,
}

impl AddressRange {
    pub fn single(addr: Addr) -> Self {
        Self {
            start: addr,
            end: addr,
        }
    }
}

pub fn parse_address_range(input: &str) -> Result<Option<(AddressRange, usize)>, EditorError> {
    if input.is_empty() {
        return Ok(None);
    }

    if input.starts_with('%') {
        return Ok(Some((
            AddressRange {
                start: Addr::Absolute(1),
                end: Addr::Last,
            },
            1,
        )));
    }

    let (first, first_len) = parse_addr(input)?;
    let Some(first) = first else {
        return Ok(None);
    };

    let rest = &input[first_len..];
    if let Some(stripped) = rest.strip_prefix(',') {
        let (second, second_len) = parse_addr(stripped)?;
        let Some(second) = second else {
            return Err(EditorError::InvalidAddress(input.to_string()));
        };

        return Ok(Some((
            AddressRange {
                start: first,
                end: second,
            },
            first_len + 1 + second_len,
        )));
    }

    Ok(Some((AddressRange::single(first), first_len)))
}

fn parse_addr(input: &str) -> Result<(Option<Addr>, usize), EditorError> {
    let mut chars = input.chars();
    let Some(ch) = chars.next() else {
        return Ok((None, 0));
    };

    match ch {
        '.' => Ok((Some(Addr::Current), 1)),
        '$' => Ok((Some(Addr::Last), 1)),
        c if c.is_ascii_digit() => {
            let mut len = 1;
            for c in input[1..].chars() {
                if c.is_ascii_digit() {
                    len += 1;
                } else {
                    break;
                }
            }

            let number = input[..len]
                .parse::<usize>()
                .map_err(|_| EditorError::InvalidAddress(input.to_string()))?;

            Ok((Some(Addr::Absolute(number)), len))
        }
        _ => Ok((None, 0)),
    }
}
