use std::io::{IsTerminal, Write};

use termcolor::{self, Color, ColorSpec, StandardStream, WriteColor};

use crate::error::ApiResult;

struct Rainbow {
    stderr: StandardStream,
}

impl Rainbow {
    fn new() -> Self {
        /* detect if color output is reasonable */
        let cc = if std::io::stdout().is_terminal() {
            termcolor::ColorChoice::Auto
        } else {
            termcolor::ColorChoice::Never
        };

        Self {
            stderr: termcolor::StandardStream::stderr(cc),
        }
    }

    fn out(&mut self, line: &str) -> ApiResult<()> {
        /* array of (width, color) pairs */
        let colors = [
            (15, Color::Rgb(0xDC, 0x00, 0x00)),
            (6, Color::Rgb(0xFF, 0xA5, 0x00)),
            (11, Color::Rgb(0xD2, 0xD2, 0x00)),
            (10, Color::Rgb(0x00, 0xC0, 0x00)),
            (9, Color::Rgb(0x00, 0x00, 0xFF)),
            (8, Color::Rgb(0x70, 0x10, 0xB0)),
            (99, Color::Rgb(0x80, 0x10, 0x80)),
        ];

        let cols = colors
            .into_iter()
            .flat_map(|(r, c)| itertools::repeat_n(c, r));

        let mut last_color = None;

        for (c, col) in line.chars().zip(cols) {
            let color = match c {
                '░' | '=' => Some(col),
                _ => None,
            };

            /* Only output color code if the color changed */
            if last_color != color {
                last_color = color;
                self.stderr.set_color(ColorSpec::new().set_fg(color))?;
            }

            write!(self.stderr, "{c}")?;
        }

        /* reset colors after printing */
        self.stderr.set_color(&ColorSpec::new())?;

        /* final newline */
        writeln!(self.stderr)?;

        Ok(())
    }
}

pub fn print() -> ApiResult<()> {
    let mut rainbow = Rainbow::new();

    eprintln!();
    rainbow.out(r"  ===================================================================")?;
    rainbow.out(r"   ███████████   ███     ██████                              █████   ")?;
    rainbow.out(r"  ░░███░░░░░███ ░░░     ███░░███                            ░░███    ")?;
    rainbow.out(r"   ░███    ░███ ████   ░███ ░░░  ████████   ██████   █████  ███████  ")?;
    rainbow.out(r"   ░██████████ ░░███  ███████   ░░███░░███ ███░░███ ███░░  ░░░███░   ")?;
    rainbow.out(r"   ░███░░░░░███ ░███ ░░░███░     ░███ ░░░ ░███ ░███░░█████   ░███    ")?;
    rainbow.out(r"   ░███    ░███ ░███   ░███      ░███     ░███ ░███ ░░░░███  ░███ ███")?;
    rainbow.out(r"   ███████████  █████  █████     █████    ░░██████  ██████   ░░█████ ")?;
    rainbow.out(r"  ░░░░░░░░░░░  ░░░░░  ░░░░░     ░░░░░      ░░░░░░  ░░░░░░     ░░░░░  ")?;
    rainbow.out(r"  ===================================================================")?;
    eprintln!();

    Ok(())
}
