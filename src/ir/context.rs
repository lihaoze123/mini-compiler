use core::fmt;
use std::fmt::Write;

use crate::{
    ast::{BType, FuncType, Ident},
    ir::symbol::Scope,
};

use super::{
    error::IRBuilderErr,
    symbol::{ScopeStack, Symbol},
};

#[derive(Clone, Copy)]
pub(super) struct Immediate(i32);

impl Immediate {
    pub(super) fn value(self) -> i32 {
        self.0
    }
}

impl From<i32> for Immediate {
    fn from(value: i32) -> Self {
        Self(value)
    }
}

impl fmt::Display for Immediate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Copy)]
pub(super) struct Temporary(usize);

impl fmt::Display for Temporary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "%{}", self.0)
    }
}

#[derive(Clone)]
pub(super) struct VariableAddress {
    identifier: String,
    id: usize,
}

impl fmt::Display for VariableAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "%{}_{}", self.identifier, self.id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Type {
    I32,
    Void,
}

impl From<BType> for Type {
    fn from(value: BType) -> Self {
        match value {
            BType::Int => Type::I32,
        }
    }
}

impl From<FuncType> for Type {
    fn from(value: FuncType) -> Self {
        match value {
            FuncType::Int => Type::I32,
            FuncType::Void => Type::Void,
        }
    }
}

#[derive(Clone)]
pub(super) struct Func {
    pub(super) identifier: String,
    pub(super) params: Vec<Type>,
    pub(super) ret: Type,
}

impl fmt::Display for Func {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "@{}", self.identifier)
    }
}

#[derive(Clone)]
pub(super) struct Label {
    name: &'static str,
    id: usize,
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "%L_{}_{}", self.name, self.id)
    }
}

#[derive(Clone, Copy)]
pub(super) enum Value {
    Immediate(Immediate),
    Temporary(Temporary),
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Self::Immediate(value.into())
    }
}

impl From<Immediate> for Value {
    fn from(value: Immediate) -> Self {
        Self::Immediate(value)
    }
}

impl From<Temporary> for Value {
    fn from(value: Temporary) -> Self {
        Self::Temporary(value)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Immediate(value) => value.fmt(f),
            Self::Temporary(value) => value.fmt(f),
        }
    }
}

pub(super) struct LoopFrame {
    pub(super) continue_label: Label,
    pub(super) break_label: Label,
    pub(super) has_break: bool,
}

#[derive(Default)]
pub(super) struct IRContext {
    output: String,
    temp_id: usize,
    label_id: usize,
    var_id: usize,
    symbols: ScopeStack,
    loops: Vec<LoopFrame>,
    global_symbols: Scope,
    current_return_type: Option<Type>,
}

impl IRContext {
    pub(super) fn reset_generation(&mut self) {
        self.output.clear();
        self.temp_id = 0;
        self.symbols.clear();
        self.loops.clear();
        self.current_return_type = None;
    }

    pub(super) fn take_output(&mut self) -> String {
        std::mem::take(&mut self.output)
    }

    pub(super) fn new_temp(&mut self) -> Temporary {
        let temp = Temporary(self.temp_id);
        self.temp_id += 1;
        temp
    }

    pub(super) fn new_label(&mut self, name: &'static str) -> Label {
        let label = Label {
            name,
            id: self.label_id,
        };
        self.label_id += 1;
        label
    }

    pub(super) fn new_variable(&mut self, id: &Ident) -> VariableAddress {
        let variable = VariableAddress {
            identifier: id.id.clone(),
            id: self.var_id,
        };
        self.var_id += 1;
        variable
    }

    pub(super) fn set_current_return_type(&mut self, return_type: Option<Type>) {
        self.current_return_type = return_type;
    }

    pub(super) fn current_return_type(&self) -> Result<Type, IRBuilderErr> {
        self.current_return_type
            .ok_or(IRBuilderErr::NoCurrentFunction)
    }

    pub(super) fn emit_instruction(
        &mut self,
        args: fmt::Arguments<'_>,
    ) -> Result<(), IRBuilderErr> {
        writeln!(self.output, "\t{}", args)?;
        Ok(())
    }

    pub(super) fn emit_line(&mut self, args: fmt::Arguments<'_>) -> Result<(), IRBuilderErr> {
        writeln!(self.output, "{}", args)?;
        Ok(())
    }

    pub(super) fn emit_binary(
        &mut self,
        op: impl fmt::Display,
        lhs: Value,
        rhs: Value,
    ) -> Result<Value, IRBuilderErr> {
        let temp = self.new_temp();
        self.emit_instruction(format_args!("{temp} = {op} {lhs}, {rhs}"))?;
        Ok(temp.into())
    }

    pub(super) fn enter_scope(&mut self) {
        self.symbols.push();
    }

    pub(super) fn exit_scope(&mut self) {
        self.symbols.pop();
    }

    pub(super) fn define_symbol(&mut self, id: &Ident, symbol: Symbol) -> Result<(), IRBuilderErr> {
        self.symbols.define(id, symbol)
    }

    pub(super) fn get_symbol(&self, id: &Ident) -> Result<Symbol, IRBuilderErr> {
        self.symbols.get(id)
    }

    pub(super) fn define_global_symbol(
        &mut self,
        id: &Ident,
        symbol: Symbol,
    ) -> Result<(), IRBuilderErr> {
        self.global_symbols.define(id, symbol)
    }

    pub(super) fn get_global_symbol(&self, id: &Ident) -> Result<Symbol, IRBuilderErr> {
        self.global_symbols
            .get(id)
            .cloned()
            .ok_or(IRBuilderErr::UndefinedSymbol(id.to_string()))
    }

    pub(super) fn push_loop(&mut self, continue_label: Label, break_label: Label) {
        self.loops.push(LoopFrame {
            continue_label,
            break_label,
            has_break: false,
        });
    }

    pub(super) fn pop_loop(&mut self) -> Result<LoopFrame, IRBuilderErr> {
        self.loops
            .pop()
            .ok_or(IRBuilderErr::EmptyLoopStack(String::new()))
    }

    pub(super) fn break_target(&mut self) -> Result<Label, IRBuilderErr> {
        let frame = self
            .loops
            .last_mut()
            .ok_or(IRBuilderErr::EmptyLoopStack("break".to_owned()))?;
        frame.has_break = true;
        Ok(frame.break_label.clone())
    }

    pub(super) fn continue_target(&self) -> Result<Label, IRBuilderErr> {
        self.loops
            .last()
            .map(|frame| frame.continue_label.clone())
            .ok_or(IRBuilderErr::EmptyLoopStack("continue".to_owned()))
    }
}
