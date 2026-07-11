use crate::ast::{ConstInitVal, Exp, InitVal};

use super::super::{context::Type, error::IRBuilderErr};

pub(super) enum InitNode<'a, E> {
    Expr(&'a E),
    List(Vec<InitNode<'a, E>>),
}

impl<'a> From<&'a InitVal> for InitNode<'a, Exp> {
    fn from(value: &'a InitVal) -> Self {
        match value {
            InitVal::Exp(exp) => Self::Expr(exp),
            InitVal::List(values) => Self::List(values.iter().map(Self::from).collect()),
        }
    }
}

impl<'a> From<&'a ConstInitVal> for InitNode<'a, Exp> {
    fn from(value: &'a ConstInitVal) -> Self {
        match value {
            ConstInitVal::Exp(const_exp) => Self::Expr(&const_exp.exp),
            ConstInitVal::List(values) => Self::List(values.iter().map(Self::from).collect()),
        }
    }
}

pub(super) fn scalar_count(ty: &Type) -> usize {
    match ty {
        Type::I32 | Type::Pointer(_) => 1,
        Type::Array(element, length) => scalar_count(element) * length,
        Type::Void => 0,
    }
}

pub(super) fn normalize_initializer<'a>(
    ty: &Type,
    init: InitNode<'a, Exp>,
) -> Result<Vec<Option<&'a Exp>>, IRBuilderErr> {
    let mut values = Vec::with_capacity(scalar_count(ty));
    fill_initializer(ty, &init, &mut values)?;
    Ok(values)
}

fn fill_initializer<'a>(
    ty: &Type,
    init: &InitNode<'a, Exp>,
    output: &mut Vec<Option<&'a Exp>>,
) -> Result<(), IRBuilderErr> {
    match ty {
        Type::I32 | Type::Pointer(_) => match init {
            InitNode::Expr(exp) => output.push(Some(*exp)),
            InitNode::List(values) if values.is_empty() => output.push(None),
            InitNode::List(values) if values.len() == 1 => {
                fill_initializer(ty, &values[0], output)?;
            }
            InitNode::List(_) => {
                return Err(IRBuilderErr::TooManyInitializers(ty.to_string()));
            }
        },
        Type::Array(element, length) => {
            let InitNode::List(values) = init else {
                return Err(IRBuilderErr::InvalidInitializer(ty.to_string()));
            };

            let start = output.len();
            let element_count = scalar_count(element);
            let total_count = element_count * length;

            for value in values {
                let used = output.len() - start;
                if used >= total_count {
                    return Err(IRBuilderErr::TooManyInitializers(ty.to_string()));
                }

                match value {
                    InitNode::Expr(exp) => output.push(Some(*exp)),
                    InitNode::List(_) => {
                        let (target_type, target_count) =
                            aligned_subobject(element, used, total_count - used);
                        let target_start = output.len();
                        fill_initializer(target_type, value, output)?;
                        let initialized = output.len() - target_start;
                        if initialized > target_count {
                            return Err(IRBuilderErr::TooManyInitializers(target_type.to_string()));
                        }
                        output.resize(target_start + target_count, None);
                    }
                }
            }

            output.resize(start + total_count, None);
        }
        Type::Void => return Err(IRBuilderErr::InvalidInitializer(ty.to_string())),
    }
    Ok(())
}

fn aligned_subobject(mut ty: &Type, initialized: usize, remaining: usize) -> (&Type, usize) {
    loop {
        let count = scalar_count(ty);
        if initialized.is_multiple_of(count) && count <= remaining {
            return (ty, count);
        }
        let Type::Array(element, _) = ty else {
            return (ty, count);
        };
        ty = element;
    }
}

pub(super) fn format_aggregate(ty: &Type, values: &[i32]) -> String {
    let mut cursor = 0;
    format_value(ty, values, &mut cursor)
}

fn format_value(ty: &Type, values: &[i32], cursor: &mut usize) -> String {
    match ty {
        Type::I32 | Type::Pointer(_) => {
            let value = values[*cursor];
            *cursor += 1;
            value.to_string()
        }
        Type::Array(element, length) => {
            let elements = (0..*length)
                .map(|_| format_value(element, values, cursor))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{{elements}}}")
        }
        Type::Void => unreachable!(),
    }
}
