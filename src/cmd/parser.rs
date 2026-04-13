/// Tokenize a command line, respecting quotes.
pub fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escape_next = false;

    for ch in input.chars() {
        if escape_next {
            current.push(ch);
            escape_next = false;
            continue;
        }

        match ch {
            '\\' if !in_single_quote => {
                escape_next = true;
            }
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
            }
            ' ' | '\t' if !in_single_quote && !in_double_quote => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

#[derive(Debug)]
pub struct ParsedCommand {
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Debug)]
pub struct Pipeline {
    pub commands: Vec<ParsedCommand>,
}

pub fn parse_pipeline(input: &str) -> Pipeline {
    let segments: Vec<&str> = split_on_pipes(input);
    let commands = segments
        .into_iter()
        .filter_map(|seg| {
            let tokens = tokenize(seg.trim());
            if tokens.is_empty() {
                return None;
            }
            Some(ParsedCommand {
                program: tokens[0].clone(),
                args: tokens[1..].to_vec(),
            })
        })
        .collect();
    Pipeline { commands }
}

fn split_on_pipes(input: &str) -> Vec<&str> {
    let mut segments = Vec::new();
    let mut start = 0;
    let mut in_single = false;
    let mut in_double = false;

    for (i, ch) in input.char_indices() {
        match ch {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '|' if !in_single && !in_double => {
                segments.push(&input[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    segments.push(&input[start..]);
    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_simple() {
        assert_eq!(tokenize("ls -la /foo"), vec!["ls", "-la", "/foo"]);
    }

    #[test]
    fn test_tokenize_quotes() {
        assert_eq!(
            tokenize(r#"grep "hello world" file.md"#),
            vec!["grep", "hello world", "file.md"]
        );
    }

    #[test]
    fn test_pipeline() {
        let pipeline = parse_pipeline("grep TODO notes/ | head -5 | wc -l");
        assert_eq!(pipeline.commands.len(), 3);
        assert_eq!(pipeline.commands[0].program, "grep");
        assert_eq!(pipeline.commands[1].program, "head");
        assert_eq!(pipeline.commands[2].program, "wc");
    }
}
