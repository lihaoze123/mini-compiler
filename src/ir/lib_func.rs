use crate::ast::Ident;

use super::{
    IRBuilder,
    context::{Func, Type},
    error::IRBuilderErr,
};

pub(super) const LIB_FUNCS: &str = r#"decl @getint(): i32
decl @getch(): i32
decl @getarray(*i32): i32
decl @putint(i32)
decl @putch(i32)
decl @putarray(i32, *i32)
decl @starttime()
decl @stoptime()
"#;

impl IRBuilder {
    pub(super) fn register_lib_funcs(&mut self) -> Result<(), IRBuilderErr> {
        let i32_ptr = Type::Pointer(Box::new(Type::I32));
        let signatures = [
            ("getint", vec![], Type::I32),
            ("getch", vec![], Type::I32),
            ("getarray", vec![i32_ptr.clone()], Type::I32),
            ("putint", vec![Type::I32], Type::Void),
            ("putch", vec![Type::I32], Type::Void),
            ("putarray", vec![Type::I32, i32_ptr], Type::Void),
            ("starttime", vec![], Type::Void),
            ("stoptime", vec![], Type::Void),
        ];

        for (identifier, params, ret) in signatures {
            let id = Ident {
                id: identifier.to_owned(),
            };
            let func = Func {
                identifier: identifier.to_owned(),
                params,
                ret,
                defined: true,
            };
            self.context.register_global_func(&id, func)?;
        }

        Ok(())
    }
}
