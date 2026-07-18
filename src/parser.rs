pub fn parse(input: &str) -> Vec<String> {
    let mut result = Vec::new();

    let mut iq = false;
    let mut st = None;

    for (idx, ch) in input.char_indices() {
        match ch {
            '"' => {
                if iq {
                    if let Some(s) = st {
                        result.push(String::from(&input[s..idx]));
                    }
                    st = None;
                } else {
                    st = Some(idx + 1);
                }

                iq = !iq;
            }
            ' ' | '\n' if !iq => {
                if let Some(s) = st {
                    result.push(String::from(&input[s..idx]));
                    st = None;
                }
            }
            _ => {
                if st.is_none() {
                    st = Some(idx);
                }
            }
        }
    }

    if let Some(s) = st {
        result.push(String::from(&input[s..]));
    }

    result
}
