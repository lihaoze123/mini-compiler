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

struct LibFunc {
    identifier: &'static str,
    param_count: usize,
    ret: Type,
}

const LIB_FUNC_SIGNATURES: &[LibFunc] = &[
    LibFunc {
        identifier: "getint",
        param_count: 0,
        ret: Type::I32,
    },
    LibFunc {
        identifier: "getch",
        param_count: 0,
        ret: Type::I32,
    },
    LibFunc {
        identifier: "getarray",
        param_count: 1,
        ret: Type::I32,
    },
    LibFunc {
        identifier: "putint",
        param_count: 1,
        ret: Type::Void,
    },
    LibFunc {
        identifier: "putch",
        param_count: 1,
        ret: Type::Void,
    },
    LibFunc {
        identifier: "putarray",
        param_count: 2,
        ret: Type::Void,
    },
    LibFunc {
        identifier: "starttime",
        param_count: 0,
        ret: Type::Void,
    },
    LibFunc {
        identifier: "stoptime",
        param_count: 0,
        ret: Type::Void,
    },
];

impl IRBuilder {
    pub(super) fn register_lib_funcs(&mut self) -> Result<(), IRBuilderErr> {
        for lib_func in LIB_FUNC_SIGNATURES {
            let id = Ident {
                id: lib_func.identifier.to_owned(),
            };
            let func = Func {
                identifier: lib_func.identifier.to_owned(),
                params: vec![Type::I32; lib_func.param_count],
                ret: lib_func.ret,
                defined: true,
            };
            self.context.register_global_func(&id, func)?;
        }

        Ok(())
    }
}
