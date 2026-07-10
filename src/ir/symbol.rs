use core::fmt;
use std::collections::HashMap;

use crate::ast::Ident;

use super::{
    context::{Immediate, VariableAddress},
    error::IRBuilderErr,
};

#[derive(Clone)]
pub(super) enum Symbol {
    Const(Immediate),
    Var(VariableAddress),
}

#[derive(Default)]
pub(super) struct ScopeStack {
    scopes: Vec<HashMap<Ident, Symbol>>,
}

impl ScopeStack {
    pub(super) fn clear(&mut self) {
        self.scopes.clear();
    }

    pub(super) fn push(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub(super) fn pop(&mut self) {
        self.scopes.pop();
    }

    pub(super) fn define(&mut self, id: &Ident, symbol: Symbol) -> Result<(), IRBuilderErr> {
        let scope = self.scopes.last_mut().ok_or(IRBuilderErr::NoScope)?;
        if scope.contains_key(id) {
            return Err(IRBuilderErr::DuplicateSymbol(id.to_string()));
        }

        scope.insert(id.clone(), symbol);
        Ok(())
    }

    pub(super) fn get(&self, id: &Ident) -> Result<Symbol, IRBuilderErr> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(id).cloned())
            .ok_or_else(|| IRBuilderErr::UndefinedSymbol(id.to_string()))
    }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{}", self.id)
    }
}
