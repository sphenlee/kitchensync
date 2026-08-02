/// Small helper used by the test harness to make line ordering deterministic.
///
/// It reads stdin line by line, collects runs of lines that begin with a single
/// uppercase letter followed by a space, sorts those collected lines using a
/// simple byte-wise comparison, and then writes them back out before passing
/// through any other line unchanged. This lets tests exercise deterministic
/// ordering. Only the status lines are sorted since these are printed out
/// from a threadpool and can appear in any order. Other messages retain their
/// current order.

use std::io::{self, BufRead, Write};

fn is_buffer_line(line: &str) -> bool {
    let bytes = line.as_bytes();
    bytes.len() > 1 && bytes[0].is_ascii_uppercase() && bytes[1] == b' '
}

fn process_input<R: BufRead, W: Write>(input: R, output: &mut W) -> io::Result<()> {
    let mut buffer = Vec::new();

    for line_result in input.lines() {
        let line = line_result?;

        if is_buffer_line(&line) {
            buffer.push(line);
        } else {
            if !buffer.is_empty() {
                buffer.sort_unstable();
                for item in &buffer {
                    writeln!(output, "{item}")?;
                }
                buffer.clear();
            }
            writeln!(output, "{line}")?;
        }
    }

    if !buffer.is_empty() {
        buffer.sort_unstable();
        for item in &buffer {
            writeln!(output, "{item}")?;
        }
    }

    Ok(())
}

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    if let Err(err) = process_input(stdin.lock(), &mut handle) {
        eprintln!("sort_helper: {err}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufReader, Cursor};


    #[test]
    fn sorts_runs_of_capitalized_lines() {
        let input = "B beta\nA alpha\nC gamma\nplain\nD delta\n";
        let mut output = Vec::new();
        let reader = BufReader::new(Cursor::new(input.as_bytes().to_vec()));

        process_input(reader, &mut output).unwrap();

        assert_eq!(
            String::from_utf8(output).unwrap(),
            "A alpha\nB beta\nC gamma\nplain\nD delta\n"
        );
    }

    #[test]
    fn flushes_buffer_at_eof() {
        let input = "A zeta\nB alpha\n";
        let mut output = Vec::new();
        let reader = BufReader::new(Cursor::new(input.as_bytes().to_vec()));

        process_input(reader, &mut output).unwrap();

        assert_eq!(String::from_utf8(output).unwrap(), "A zeta\nB alpha\n");
    }
}
