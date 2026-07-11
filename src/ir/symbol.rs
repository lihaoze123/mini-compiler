use core::fmt;
use std::collections::HashMap;

use crate::{
    ast::Ident,
    ir::context::{Func, Type},
};

use super::{
    context::{Immediate, VariableAddress},
    error::IRBuilderErr,
};

#[derive(Clone)]
pub(super) enum Symbol {
    Const(Immediate),
    Object(Object),
    Func(Func),
}

#[derive(Clone)]
pub(super) struct Object {
    pub(super) address: VariableAddress,
    pub(super) ty: Type,
    pub(super) mutable: bool,
    pub(super) const_values: Option<Vec<i32>>,
}

#[derive(Default)]
pub(super) struct Scope(HashMap<Ident, Symbol>);

impl Scope {
    pub(super) fn define(&mut self, id: &Ident, symbol: Symbol) -> Result<(), IRBuilderErr> {
        if self.0.contains_key(id) {
            return Err(IRBuilderErr::DuplicateSymbol(id.to_string()));
        }
        self.0.insert(id.clone(), symbol);
        Ok(())
    }

    pub(super) fn get(&self, id: &Ident) -> Option<&Symbol> {
        self.0.get(id)
    }

    pub(super) fn get_mut(&mut self, id: &Ident) -> Option<&mut Symbol> {
        self.0.get_mut(id)
    }
}

#[derive(Default)]
pub(super) struct ScopeStack {
    scopes: Vec<Scope>,
}

impl ScopeStack {
    pub(super) fn clear(&mut self) {
        self.scopes.clear();
    }

    pub(super) fn push(&mut self) {
        self.scopes.push(Scope::default());
    }

    pub(super) fn pop(&mut self) {
        self.scopes.pop();
    }

    pub(super) fn define(&mut self, id: &Ident, symbol: Symbol) -> Result<(), IRBuilderErr> {
        let scope = self.scopes.last_mut().ok_or(IRBuilderErr::NoScope)?;
        scope.define(id, symbol)
    }

    pub(super) fn find(&self, id: &Ident) -> Option<Symbol> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(id).cloned())
    }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{}", self.id)
    }
}
