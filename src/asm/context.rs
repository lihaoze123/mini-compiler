use core::fmt;
use std::fmt::Write;

use super::error::GenerateAsmError;

pub(super) struct EdgeLabel(usize);

impl fmt::Display for EdgeLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, ".L_edge_{}", self.0)
    }
}

#[derive(Default)]
pub(super) struct AsmContext {
    output: String,
    edge_label_id: usize,
}

impl AsmContext {
    pub(super) fn reset_generation(&mut self) {
        self.output.clear();
        self.edge_label_id = 0;
    }

    pub(super) fn take_output(&mut self) -> String {
        std::mem::take(&mut self.output)
    }

    pub(super) fn emit_instruction(
        &mut self,
        args: fmt::Arguments<'_>,
    ) -> Result<(), GenerateAsmError> {
        writeln!(self.output, "\t{}", args)?;
        Ok(())
    }

    pub(super) fn emit_line(&mut self, args: fmt::Arguments<'_>) -> Result<(), GenerateAsmError> {
        writeln!(self.output, "{}", args)?;
        Ok(())
    }

    pub(super) fn new_edge_label(&mut self) -> EdgeLabel {
        let label = EdgeLabel(self.edge_label_id);
        self.edge_label_id += 1;
        label
    }
}
