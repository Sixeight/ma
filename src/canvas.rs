pub(crate) struct Canvas {
    cells: Vec<char>,
    width: usize,
    height: usize,
}

impl Canvas {
    pub(crate) fn new(width: usize, height: usize) -> Self {
        Self {
            cells: vec![' '; width * height],
            width,
            height,
        }
    }

    pub(crate) fn width(&self) -> usize {
        self.width
    }

    pub(crate) fn set(&mut self, row: usize, col: usize, ch: char) {
        if row < self.height && col < self.width {
            let index = self.index(row, col);
            if self.cells[index] == '\0' && col > 0 && self.cells[index - 1] != '\0' {
                self.cells[index - 1] = ' ';
            }
            self.cells[index] = ch;
        }
    }

    pub(crate) fn write_str(&mut self, row: usize, col: usize, text: &str) {
        let mut offset = 0;
        for ch in text.chars() {
            self.set(row, col + offset, ch);
            let width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
            for continuation in 1..width {
                self.set(row, col + offset + continuation, '\0');
            }
            offset += width;
        }
    }

    pub(crate) fn set_merged(
        &mut self,
        row: usize,
        col: usize,
        ch: char,
        merge: impl FnOnce(char, char) -> char,
    ) {
        if row < self.height && col < self.width {
            let index = self.index(row, col);
            self.set(row, col, merge(self.cells[index], ch));
        }
    }

    pub(crate) fn render(&self) -> String {
        let mut output = String::with_capacity(self.cells.len() + self.height.saturating_sub(1));

        // Reserved space the drawing did not need renders as blank rows; they
        // carry no meaning at the end of the output.
        let last_row = (0..self.height)
            .rposition(|row| {
                self.cells[row * self.width..(row + 1) * self.width]
                    .iter()
                    .any(|ch| *ch != ' ')
            })
            .map_or(0, |row| row + 1);

        for row in 0..last_row {
            if row > 0 {
                output.push('\n');
            }

            let cells = &self.cells[row * self.width..(row + 1) * self.width];
            let last_content = cells
                .iter()
                .rposition(|ch| *ch != ' ')
                .map_or(0, |index| index + 1);

            for ch in &cells[..last_content] {
                if *ch != '\0' {
                    output.push(*ch);
                }
            }
        }

        output
    }

    fn index(&self, row: usize, col: usize) -> usize {
        row * self.width + col
    }
}

#[cfg(test)]
mod tests {
    use super::Canvas;
    use pretty_assertions::assert_eq;

    #[test]
    fn overwriting_a_wide_character_continuation_clears_its_base() {
        let mut canvas = Canvas::new(6, 1);
        canvas.write_str(0, 0, "テス");
        canvas.set(0, 3, '│');

        assert_eq!(canvas.render(), "テ │");
    }

    #[test]
    fn render_preserves_blank_rows_and_omits_wide_character_continuations() {
        let mut canvas = Canvas::new(5, 3);
        canvas.write_str(0, 0, "テ");
        canvas.write_str(2, 2, "x");

        assert_eq!(canvas.render(), "テ\n\n  x");
    }
}
