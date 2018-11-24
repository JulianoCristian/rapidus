use rustc_hash::FxHashMap;
use std::ffi::CString;

use super::{
    callobj::{CallObject, CallObjectRef}, error::*, value::{ArrayValue, FuncId, Value, ValueBase},
};
use builtin;
use bytecode_gen::{ByteCode, VMInst};
use gc;
use jit::TracingJit;

pub struct VM {
    pub jit: TracingJit,
    pub state: VMState,
    pub const_table: ConstantTable,
    pub cur_func_id: FuncId, // id == 0: main
    pub op_table: [fn(&mut VM, &ByteCode) -> Result<(), RuntimeError>; 51],
}

pub struct VMState {
    pub stack: Vec<Value>,
    pub scope: Vec<CallObjectRef>,
    pub pc: isize,
    pub history: Vec<(usize, isize)>, // sp, return_pc
}

#[derive(Debug, Clone)]
pub struct ConstantTable {
    pub value: Vec<Value>,
    pub string: Vec<String>,
}

impl ConstantTable {
    pub fn new() -> ConstantTable {
        ConstantTable {
            value: vec![],
            string: vec![],
        }
    }
}


impl VM {
    pub fn new(global_vals: CallObjectRef) -> VM {
        // TODO: Support for 'require' is not enough.
        unsafe {
            (*global_vals).set_value(
                "require".to_string(),
                Value::builtin_function(
                    builtin::require,
                    builtin::Builtins::Require,
                    CallObject::new(Value::undefined()),
                ),
            );

            let module_exports = Value::object(gc::new(FxHashMap::default()));
            (*global_vals).set_value("module".to_string(), {
                let mut map = FxHashMap::default();
                map.insert("exports".to_string(), module_exports.clone());
                Value::object(gc::new(map))
            });
            (*global_vals).set_value("exports".to_string(), module_exports);
        }

        unsafe {
            (*global_vals).set_value("console".to_string(), {
                let mut map = FxHashMap::default();
                map.insert(
                    "log".to_string(),
                    Value::builtin_function(
                        builtin::console_log,
                        builtin::Builtins::ConsoleLog,
                        CallObject::new(Value::undefined()),
                    ),
                );
                Value::object(gc::new(map))
            });
        }

        unsafe {
            (*global_vals).set_value("process".to_string(), {
                let mut map = FxHashMap::default();
                map.insert("stdout".to_string(), {
                    let mut map = FxHashMap::default();
                    map.insert(
                        "write".to_string(),
                        Value::builtin_function(
                            builtin::process_stdout_write,
                            builtin::Builtins::ProcessStdoutWrite,
                            CallObject::new(Value::undefined()),
                        ),
                    );
                    Value::object(gc::new(map))
                });
                Value::object(gc::new(map))
            });
        }

        unsafe {
            use builtins::array::ARRAY_OBJ;
            (*global_vals).set_value("Array".to_string(), ARRAY_OBJ.with(|x| x.clone()));
        }

        unsafe {
            (*global_vals).set_value("Math".to_string(), {
                let mut map = FxHashMap::default();
                map.insert("PI".to_string(), Value::number(::std::f64::consts::PI));
                map.insert(
                    "floor".to_string(),
                    Value::builtin_function(
                        builtin::math_floor,
                        builtin::Builtins::MathFloor,
                        CallObject::new(Value::undefined()),
                    ),
                );
                map.insert(
                    "random".to_string(),
                    Value::builtin_function(
                        builtin::math_random,
                        builtin::Builtins::MathRandom,
                        CallObject::new(Value::undefined()),
                    ),
                );
                map.insert(
                    "pow".to_string(),
                    Value::builtin_function(
                        builtin::math_pow,
                        builtin::Builtins::MathPow,
                        CallObject::new(Value::undefined()),
                    ),
                );
                map.insert(
                    "abs".to_string(),
                    Value::builtin_function(
                        builtin::math_abs,
                        builtin::Builtins::MathAbs,
                        CallObject::new(Value::undefined()),
                    ),
                );
                map.insert(
                    "acos".to_string(),
                    Value::builtin_function(
                        builtin::math_acos,
                        builtin::Builtins::MathAcos,
                        CallObject::new(Value::undefined()),
                    ),
                );
                map.insert(
                    "acosh".to_string(),
                    Value::builtin_function(
                        builtin::math_acosh,
                        builtin::Builtins::MathAcosh,
                        CallObject::new(Value::undefined()),
                    ),
                );
                map.insert(
                    "asin".to_string(),
                    Value::builtin_function(
                        builtin::math_asin,
                        builtin::Builtins::MathAsin,
                        CallObject::new(Value::undefined()),
                    ),
                );
                map.insert(
                    "asinh".to_string(),
                    Value::builtin_function(
                        builtin::math_asinh,
                        builtin::Builtins::MathAsinh,
                        CallObject::new(Value::undefined()),
                    ),
                );
                map.insert(
                    "atan".to_string(),
                    Value::builtin_function(
                        builtin::math_atan,
                        builtin::Builtins::MathAtan,
                        CallObject::new(Value::undefined()),
                    ),
                );
                map.insert(
                    "atanh".to_string(),
                    Value::builtin_function(
                        builtin::math_atanh,
                        builtin::Builtins::MathAtanh,
                        CallObject::new(Value::undefined()),
                    ),
                );
                map.insert(
                    "atan2".to_string(),
                    Value::builtin_function(
                        builtin::math_atan2,
                        builtin::Builtins::MathAtan2,
                        CallObject::new(Value::undefined()),
                    ),
                );
                map.insert(
                    "cbrt".to_string(),
                    Value::builtin_function(
                        builtin::math_cbrt,
                        builtin::Builtins::MathCbrt,
                        CallObject::new(Value::undefined()),
                    ),
                );
                map.insert(
                    "ceil".to_string(),
                    Value::builtin_function(
                        builtin::math_ceil,
                        builtin::Builtins::MathCeil,
                        CallObject::new(Value::undefined()),
                    ),
                );
                map.insert(
                    "clz32".to_string(),
                    Value::builtin_function(
                        builtin::math_clz32,
                        builtin::Builtins::MathClz32,
                        CallObject::new(Value::undefined()),
                    ),
                );
                map.insert(
                    "cos".to_string(),
                    Value::builtin_function(
                        builtin::math_cos,
                        builtin::Builtins::MathCos,
                        CallObject::new(Value::undefined()),
                    ),
                );
                map.insert(
                    "cosh".to_string(),
                    Value::builtin_function(
                        builtin::math_cosh,
                        builtin::Builtins::MathCosh,
                        CallObject::new(Value::undefined()),
                    ),
                );
                map.insert(
                    "exp".to_string(),
                    Value::builtin_function(
                        builtin::math_exp,
                        builtin::Builtins::MathExp,
                        CallObject::new(Value::undefined()),
                    ),
                );
                map.insert(
                    "expm1".to_string(),
                    Value::builtin_function(
                        builtin::math_expm1,
                        builtin::Builtins::MathExpm1,
                        CallObject::new(Value::undefined()),
                    ),
                );
                map.insert(
                    "fround".to_string(),
                    Value::builtin_function(
                        builtin::math_fround,
                        builtin::Builtins::MathFround,
                        CallObject::new(Value::undefined()),
                    ),
                );
                map.insert(
                    "hypot".to_string(),
                    Value::builtin_function(
                        builtin::math_hypot,
                        builtin::Builtins::MathHypot,
                        CallObject::new(Value::undefined()),
                    ),
                );
                map.insert(
                    "log".to_string(),
                    Value::builtin_function(
                        builtin::math_log,
                        builtin::Builtins::MathLog,
                        CallObject::new(Value::undefined()),
                    ),
                );
                map.insert(
                    "log1p".to_string(),
                    Value::builtin_function(
                        builtin::math_log1p,
                        builtin::Builtins::MathLog1p,
                        CallObject::new(Value::undefined()),
                    ),
                );
                map.insert(
                    "log10".to_string(),
                    Value::builtin_function(
                        builtin::math_log10,
                        builtin::Builtins::MathLog10,
                        CallObject::new(Value::undefined()),
                    ),
                );
                map.insert(
                    "log2".to_string(),
                    Value::builtin_function(
                        builtin::math_log2,
                        builtin::Builtins::MathLog2,
                        CallObject::new(Value::undefined()),
                    ),
                );
                map.insert(
                    "max".to_string(),
                    Value::builtin_function(
                        builtin::math_max,
                        builtin::Builtins::MathMax,
                        CallObject::new(Value::undefined()),
                    ),
                );
                map.insert(
                    "min".to_string(),
                    Value::builtin_function(
                        builtin::math_min,
                        builtin::Builtins::MathMin,
                        CallObject::new(Value::undefined()),
                    ),
                );
                map.insert(
                    "round".to_string(),
                    Value::builtin_function(
                        builtin::math_round,
                        builtin::Builtins::MathRound,
                        CallObject::new(Value::undefined()),
                    ),
                );
                map.insert(
                    "sign".to_string(),
                    Value::builtin_function(
                        builtin::math_sign,
                        builtin::Builtins::MathSign,
                        CallObject::new(Value::undefined()),
                    ),
                );
                map.insert(
                    "sin".to_string(),
                    Value::builtin_function(
                        builtin::math_sin,
                        builtin::Builtins::MathSin,
                        CallObject::new(Value::undefined()),
                    ),
                );
                map.insert(
                    "sinh".to_string(),
                    Value::builtin_function(
                        builtin::math_sinh,
                        builtin::Builtins::MathSinh,
                        CallObject::new(Value::undefined()),
                    ),
                );
                map.insert(
                    "sqrt".to_string(),
                    Value::builtin_function(
                        builtin::math_sqrt,
                        builtin::Builtins::MathSqrt,
                        CallObject::new(Value::undefined()),
                    ),
                );
                map.insert(
                    "tan".to_string(),
                    Value::builtin_function(
                        builtin::math_tan,
                        builtin::Builtins::MathTan,
                        CallObject::new(Value::undefined()),
                    ),
                );
                map.insert(
                    "tanh".to_string(),
                    Value::builtin_function(
                        builtin::math_tanh,
                        builtin::Builtins::MathTanh,
                        CallObject::new(Value::undefined()),
                    ),
                );
                map.insert(
                    "trunc".to_string(),
                    Value::builtin_function(
                        builtin::math_trunc,
                        builtin::Builtins::MathTrunc,
                        CallObject::new(Value::undefined()),
                    ),
                );
                Value::object(gc::new(map))
            });
        }

        VM {
            jit: unsafe { TracingJit::new() },
            state: VMState {
                stack: { Vec::with_capacity(128) },
                scope: vec![global_vals],
                history: {
                    let mut s = Vec::with_capacity(128);
                    s.push((0, 0));
                    s
                },
                pc: 0isize,
            },
            const_table: ConstantTable::new(),
            cur_func_id: 0, // 0 is main
            op_table: [
                end,
                create_context,
                construct,
                create_object,
                create_array,
                push_int8,
                push_int32,
                push_false,
                push_true,
                push_const,
                push_this,
                push_arguments,
                push_undefined,
                lnot,
                posi,
                neg,
                add,
                sub,
                mul,
                div,
                rem,
                lt,
                gt,
                le,
                ge,
                eq,
                ne,
                seq,
                sne,
                and,
                or,
                xor,
                shl,
                shr,
                zfshr,
                get_member,
                set_member,
                jmp_if_false,
                jmp,
                call,
                return_,
                double,
                pop,
                land,
                lor,
                set_cur_callobj,
                get_name,
                set_name,
                decl_var,
                cond_op,
                loop_start,
            ],
        }
    }
}

impl VM {
    pub fn run(&mut self, iseq: ByteCode) -> Result<(), RuntimeError> {
        // self.iseq = iseq;
        // Unlock the mutex and start the profiler
        // PROFILER
        //     .lock()
        //     .unwrap()
        //     .start("./my-prof.profile")
        //     .expect("Couldn't start");

        self.do_run(&iseq)

        // Unwrap the mutex and stop the profiler
        // PROFILER.lock().unwrap().stop().expect("Couldn't stop");
    }

    pub fn do_run(&mut self, iseq: &ByteCode) -> Result<(), RuntimeError> {
        // let id = self.cur_func_id;
        loop {
            let code = iseq[self.state.pc as usize];
            self.op_table[code as usize](self, iseq)?;
            if code == VMInst::RETURN || code == VMInst::END {
                break;
            }
            // println!("stack trace: {:?} - {}", self.stack, *pc);
        }

        Ok(())
    }
}

macro_rules! get_int8 {
    ($self:ident, $iseq:ident, $var:ident, $ty:ty) => {
        let $var = $iseq[$self.state.pc as usize] as $ty;
        $self.state.pc += 1;
    };
}

macro_rules! get_int32 {
    ($self:ident, $iseq:ident, $var:ident, $ty:ty) => {
        let $var = (($iseq[$self.state.pc as usize + 3] as $ty) << 24)
            + (($iseq[$self.state.pc as usize + 2] as $ty) << 16)
            + (($iseq[$self.state.pc as usize + 1] as $ty) << 8)
            + ($iseq[$self.state.pc as usize + 0] as $ty);
        $self.state.pc += 4;
    };
}

fn end(_self: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    Ok(())
}

fn create_context(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // create_context
    Ok(())
}

fn construct(self_: &mut VM, iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // construct
    get_int32!(self_, iseq, argc, usize);

    let callee = self_.state.stack.pop().unwrap();

    match callee.val.clone() {
        ValueBase::BuiltinFunction(box (x, obj, mut callobj)) => {
            let new_this = {
                let mut map = FxHashMap::default();
                map.insert("__proto__".to_string(), unsafe {
                    (*obj)
                        .get("prototype")
                        .unwrap_or(&Value::undefined())
                        .clone()
                });
                gc::new(map)
            };
            let mut args = vec![];

            for _ in 0..argc {
                args.push(self_.state.stack.pop().unwrap());
            }

            *callobj.this = Value::object(new_this);

            (x.func)(self_, &args, &callobj);
        }
        ValueBase::Function(box (id, iseq, obj, mut callobj)) => {
            let new_this = {
                let mut map = FxHashMap::default();
                map.insert("__proto__".to_string(), unsafe {
                    (*obj)
                        .get("prototype")
                        .unwrap_or(&Value::undefined())
                        .clone()
                });
                gc::new(map)
            };

            callobj.vals = gc::new(FxHashMap::default());

            // similar code is used some times. should make it a function.
            let mut args = vec![];
            let mut rest_args = vec![];
            let mut rest_param_name = None;
            for _ in 0..argc {
                args.push(self_.state.stack.pop().unwrap());
            }
            for (i, arg) in args.iter().enumerate() {
                if let Some(name) = callobj.get_parameter_nth_name(i) {
                    // When rest parameter
                    if callobj.params[i].1 {
                        rest_param_name = Some(name);
                        rest_args.push(arg.clone());
                    } else {
                        callobj.set_value(name, arg.clone());
                    }
                } else {
                    rest_args.push(arg.clone());
                }
            }
            if let Some(rest_param_name) = rest_param_name {
                callobj.set_value(
                    rest_param_name,
                    Value::array(gc::new(ArrayValue::new(rest_args))),
                );
            } else {
                for arg in rest_args {
                    callobj.arg_rest_vals.push(arg.clone());
                }
            }

            *callobj.this = Value::object(new_this);
            self_.state.scope.push(gc::new(callobj));
            self_
                .state
                .history
                .push((self_.state.stack.len(), self_.state.pc));
            self_.state.pc = 0;
            let save_id = self_.cur_func_id;
            self_.cur_func_id = id;

            self_.do_run(&iseq)?;

            self_.cur_func_id = save_id;
            self_.state.scope.pop();

            match self_.state.stack.last_mut().unwrap() {
                &mut Value {
                    val: ValueBase::Object(_),
                    ..
                }
                | &mut Value {
                    val: ValueBase::Array(_),
                    ..
                }
                | &mut Value {
                    val: ValueBase::Function(box (_, _, _, _)),
                    ..
                }
                | &mut Value {
                    val: ValueBase::BuiltinFunction(box (_, _, _)),
                    ..
                } => {}
                others => *others = Value::object(new_this),
            };
        }
        c => {
            return Err(RuntimeError::Type(format!(
                "type error(pc:{}): '{:?}' is not a constructor",
                self_.state.pc, c
            )));
        }
    };

    Ok(())
}

fn create_object(self_: &mut VM, iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // create_object
    get_int32!(self_, iseq, len, usize);

    let mut map = FxHashMap::default();
    for _ in 0..len {
        let name = if let ValueBase::String(name) = self_.state.stack.pop().unwrap().val {
            name.into_string().unwrap()
        } else {
            unreachable!()
        };
        let val = self_.state.stack.pop().unwrap();
        map.insert(name, val.clone());
    }

    self_.state.stack.push(Value::object(gc::new(map)));

    gc::mark_and_sweep(&self_.state);

    Ok(())
}

fn create_array(self_: &mut VM, iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // create_array
    get_int32!(self_, iseq, len, usize);

    let mut arr = vec![];
    for _ in 0..len {
        let val = self_.state.stack.pop().unwrap();
        arr.push(val);
    }

    self_
        .state
        .stack
        .push(Value::array(gc::new(ArrayValue::new(arr))));

    gc::mark_and_sweep(&self_.state);

    Ok(())
}

fn push_int8(self_: &mut VM, iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // push_int
    get_int8!(self_, iseq, n, i8);
    self_.state.stack.push(Value::number(n as f64));
    Ok(())
}

fn push_int32(self_: &mut VM, iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // push_int
    get_int32!(self_, iseq, n, i32);
    self_.state.stack.push(Value::number(n as f64));
    Ok(())
}

fn push_false(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // push_false
    self_.state.stack.push(Value::bool(false));
    Ok(())
}

fn push_true(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // push_true
    self_.state.stack.push(Value::bool(true));
    Ok(())
}

fn push_const(self_: &mut VM, iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // push_const
    get_int32!(self_, iseq, n, usize);
    self_.state.stack.push(self_.const_table.value[n].clone());
    Ok(())
}

fn push_this(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // push_this
    let this = unsafe { *(**self_.state.scope.last().unwrap()).this.clone() };
    self_.state.stack.push(this);
    Ok(())
}

fn push_arguments(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // push_arguments
    self_.state.stack.push(Value::arguments());
    Ok(())
}

fn push_undefined(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // push_defined
    self_.state.stack.push(Value::undefined());
    Ok(())
}

fn lnot(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // lnot
    let expr = self_.state.stack.last_mut().unwrap();
    expr.val = ValueBase::Bool(!expr.val.to_boolean());
    Ok(())
}

fn posi(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // posi
    let expr = self_.state.stack.last_mut().unwrap();
    expr.val = ValueBase::Number(expr.val.to_number());
    Ok(())
}

fn neg(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // neg
    let expr = self_.state.stack.last_mut().unwrap();
    match &mut expr.val {
        &mut ValueBase::Number(ref mut n) => *n = -*n,
        _ => return Err(RuntimeError::Unimplemented),
    };
    Ok(())
}

fn add(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // $name
    let rhs = self_.state.stack.pop().unwrap();
    let lhs = self_.state.stack.pop().unwrap();
    self_.state.stack.push(match (lhs.val, rhs.val) {
        (ValueBase::Number(l), ValueBase::Number(r)) => Value::number(l + r),
        (ValueBase::Bool(false), ValueBase::Number(x))
        | (ValueBase::Number(x), ValueBase::Bool(false)) => Value::number(x),
        (ValueBase::Bool(true), ValueBase::Number(x))
        | (ValueBase::Number(x), ValueBase::Bool(true)) => Value::number(x + 1.0),
        // TODO: We need the correct implementation.
        (ValueBase::Undefined, _) | (_, ValueBase::Undefined) => Value::number(::std::f64::NAN),
        (l, r) => Value::string(CString::new(l.to_string() + r.to_string().as_str()).unwrap()),
    });
    Ok(())
}

fn sub(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // $name
    let rhs = self_.state.stack.pop().unwrap();
    let lhs = self_.state.stack.pop().unwrap();
    self_.state.stack.push(match (lhs.val, rhs.val) {
        (ValueBase::Number(l), ValueBase::Number(r)) => Value::number(l - r),
        _ => return Err(RuntimeError::Unimplemented),
    });
    Ok(())
}

fn mul(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // $name
    let rhs = self_.state.stack.pop().unwrap();
    let lhs = self_.state.stack.pop().unwrap();
    self_.state.stack.push(match (lhs.val, rhs.val) {
        (ValueBase::Number(l), ValueBase::Number(r)) => Value::number(l * r),
        (ValueBase::String(l), ValueBase::Number(r)) => {
            Value::string(CString::new(l.to_str().unwrap().repeat(r as usize)).unwrap())
        }
        _ => return Err(RuntimeError::Unimplemented),
    });
    Ok(())
}

fn div(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // $name
    let rhs = self_.state.stack.pop().unwrap();
    let lhs = self_.state.stack.pop().unwrap();
    self_.state.stack.push(match (lhs.val, rhs.val) {
        (ValueBase::Number(l), ValueBase::Number(r)) => Value::number(l / r),
        _ => return Err(RuntimeError::Unimplemented),
    });
    Ok(())
}

fn rem(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // $name
    let rhs = self_.state.stack.pop().unwrap();
    let lhs = self_.state.stack.pop().unwrap();
    self_.state.stack.push(match (lhs.val, rhs.val) {
        (ValueBase::Number(l), ValueBase::Number(r)) => Value::number((l as i64 % r as i64) as f64),
        _ => return Err(RuntimeError::Unimplemented),
    });
    Ok(())
}

fn lt(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // $name
    let rhs = self_.state.stack.pop().unwrap();
    let lhs = self_.state.stack.pop().unwrap();
    self_.state.stack.push(match (lhs.val, rhs.val) {
        (ValueBase::Number(l), ValueBase::Number(r)) => Value::bool(l < r),
        (ValueBase::String(l), ValueBase::String(r)) => Value::bool(l < r),
        _ => return Err(RuntimeError::Unimplemented),
    });
    Ok(())
}

fn gt(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // $name
    let rhs = self_.state.stack.pop().unwrap();
    let lhs = self_.state.stack.pop().unwrap();
    self_.state.stack.push(match (lhs.val, rhs.val) {
        (ValueBase::Number(l), ValueBase::Number(r)) => Value::bool(l > r),
        (ValueBase::String(l), ValueBase::String(r)) => Value::bool(l > r),
        _ => return Err(RuntimeError::Unimplemented),
    });
    Ok(())
}

fn le(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // $name
    let rhs = self_.state.stack.pop().unwrap();
    let lhs = self_.state.stack.pop().unwrap();
    self_.state.stack.push(match (lhs.val, rhs.val) {
        (ValueBase::Number(l), ValueBase::Number(r)) => Value::bool(l <= r),
        (ValueBase::String(l), ValueBase::String(r)) => Value::bool(l <= r),
        _ => return Err(RuntimeError::Unimplemented),
    });
    Ok(())
}

fn ge(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // $name
    let rhs = self_.state.stack.pop().unwrap();
    let lhs = self_.state.stack.pop().unwrap();
    self_.state.stack.push(match (lhs.val, rhs.val) {
        (ValueBase::Number(l), ValueBase::Number(r)) => Value::bool(l >= r),
        (ValueBase::String(l), ValueBase::String(r)) => Value::bool(l >= r),
        _ => return Err(RuntimeError::Unimplemented),
    });
    Ok(())
}

// TODO: Need more precise implemention
fn eq(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // $name
    let rhs = self_.state.stack.pop().unwrap();
    let lhs = self_.state.stack.pop().unwrap();
    self_.state.stack.push(match (lhs.val, rhs.val) {
        (ValueBase::Number(l), ValueBase::Number(r)) => Value::bool(l == r),
        (ValueBase::String(l), ValueBase::String(r)) => Value::bool(l == r),
        _ => return Err(RuntimeError::Unimplemented),
    });
    Ok(())
}

// TODO: Need more precise implemention
fn ne(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // $name
    let rhs = self_.state.stack.pop().unwrap();
    let lhs = self_.state.stack.pop().unwrap();
    self_.state.stack.push(match (lhs.val, rhs.val) {
        (ValueBase::Null, ValueBase::Null) => Value::bool(false),
        (ValueBase::Undefined, ValueBase::Undefined) => Value::bool(false),
        (ValueBase::Number(l), ValueBase::Number(r)) => Value::bool(l != r),
        (ValueBase::String(l), ValueBase::String(r)) => Value::bool(l != r),
        _ => return Err(RuntimeError::Unimplemented),
    });
    Ok(())
}

// TODO: Need more precise implemention
fn seq(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // $name
    let rhs = self_.state.stack.pop().unwrap();
    let lhs = self_.state.stack.pop().unwrap();
    self_.state.stack.push(match (lhs.val, rhs.val) {
        (ValueBase::Null, ValueBase::Null) => Value::bool(true),
        (ValueBase::Undefined, ValueBase::Undefined) => Value::bool(true),
        (ValueBase::Number(l), ValueBase::Number(r)) => Value::bool(l == r),
        (ValueBase::String(l), ValueBase::String(r)) => Value::bool(l == r),
        _ => return Err(RuntimeError::Unimplemented),
    });
    Ok(())
}

// TODO: Need more precise implemention
fn sne(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // $name
    let rhs = self_.state.stack.pop().unwrap();
    let lhs = self_.state.stack.pop().unwrap();
    self_.state.stack.push(match (lhs.val, rhs.val) {
        (ValueBase::Number(l), ValueBase::Number(r)) => Value::bool(l != r),
        (ValueBase::String(l), ValueBase::String(r)) => Value::bool(l != r),
        _ => return Err(RuntimeError::Unimplemented),
    });
    Ok(())
}

fn and(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // $name
    let rhs = self_.state.stack.pop().unwrap();
    let lhs = self_.state.stack.pop().unwrap();
    self_.state.stack.push(match (lhs.val, rhs.val) {
        (ValueBase::Number(l), ValueBase::Number(r)) => {
            Value::number(((l as i64 as i32) & (r as i64 as i32)) as f64)
        }
        _ => return Err(RuntimeError::Unimplemented),
    });
    Ok(())
}

fn or(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // $name
    let rhs = self_.state.stack.pop().unwrap();
    let lhs = self_.state.stack.pop().unwrap();
    self_.state.stack.push(match (lhs.val, rhs.val) {
        (ValueBase::Number(l), ValueBase::Number(r)) => {
            Value::number(((l as i64 as i32) | (r as i64 as i32)) as f64)
        }
        _ => return Err(RuntimeError::Unimplemented),
    });
    Ok(())
}

fn xor(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // $name
    let rhs = self_.state.stack.pop().unwrap();
    let lhs = self_.state.stack.pop().unwrap();
    self_.state.stack.push(match (lhs.val, rhs.val) {
        (ValueBase::Number(l), ValueBase::Number(r)) => {
            Value::number(((l as i64 as i32) ^ (r as i64 as i32)) as f64)
        }
        _ => return Err(RuntimeError::Unimplemented),
    });
    Ok(())
}

fn shl(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // $name
    let rhs = self_.state.stack.pop().unwrap();
    let lhs = self_.state.stack.pop().unwrap();
    self_.state.stack.push(match (lhs.val, rhs.val) {
        (ValueBase::Number(l), ValueBase::Number(r)) => {
            Value::number(((l as i64 as i32) << (r as i64 as i32)) as f64)
        }
        _ => return Err(RuntimeError::Unimplemented),
    });
    Ok(())
}

fn shr(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // $name
    let rhs = self_.state.stack.pop().unwrap();
    let lhs = self_.state.stack.pop().unwrap();
    self_.state.stack.push(match (lhs.val, rhs.val) {
        (ValueBase::Number(l), ValueBase::Number(r)) => {
            Value::number(((l as i64 as i32) >> (r as i64 as i32)) as f64)
        }
        _ => return Err(RuntimeError::Unimplemented),
    });
    Ok(())
}

fn zfshr(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // $name
    let rhs = self_.state.stack.pop().unwrap();
    let lhs = self_.state.stack.pop().unwrap();
    self_.state.stack.push(match (lhs.val, rhs.val) {
        (ValueBase::Number(l), ValueBase::Number(r)) => {
            Value::number(((l as u64 as u32) >> (r as u64 as u32)) as f64)
        }
        _ => return Err(RuntimeError::Unimplemented),
    });
    Ok(())
}

fn get_member(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // get_global
    let member = self_.state.stack.pop().unwrap();
    let parent = self_.state.stack.pop().unwrap();
    let val = parent.get_property(member.val, Some(self_.state.scope.last().unwrap()));
    self_.state.stack.push(val);
    Ok(())
}

fn set_member(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // get_global
    let member = self_.state.stack.pop().unwrap();
    let parent = self_.state.stack.pop().unwrap();
    let val = self_.state.stack.pop().unwrap();
    // TODO: The following code should be a function (like Value::set_property).
    match parent.val {
        ValueBase::Object(map) | ValueBase::Function(box (_, _, map, _)) => unsafe {
            *(*map)
                .entry(member.to_string())
                .or_insert_with(|| Value::undefined()) = val;
        },
        ValueBase::Array(map) => unsafe {
            fn set_by_idx(map: &mut ArrayValue, n: usize, val: Value) {
                if n >= map.length as usize {
                    map.length = n + 1;
                    while map.elems.len() < n + 1 {
                        map.elems.push(Value::empty());
                    }
                }
                map.elems[n] = val;
            };

            let mut map = &mut *map;

            match member.val {
                // Index
                ValueBase::Number(n) if n - n.floor() == 0.0 && n >= 0.0 => {
                    set_by_idx(map, n as usize, val)
                }
                ValueBase::String(ref s) if s.to_str().unwrap() == "length" => match val.val {
                    ValueBase::Number(n) if n - n.floor() == 0.0 && n >= 0.0 => {
                        map.length = n as usize;
                        while map.elems.len() < n as usize + 1 {
                            map.elems.push(Value::empty());
                        }
                    }
                    _ => {}
                },
                // https://www.ecma-international.org/ecma-262/9.0/index.html#sec-array-exotic-objects
                ValueBase::String(ref s)
                    if Value::number(member.val.to_uint32()).to_string() == s.to_str().unwrap() =>
                {
                    let num = member.val.to_uint32();
                    set_by_idx(map, num as usize, val)
                }
                _ => {
                    *map.obj
                        .entry(member.to_string())
                        .or_insert_with(|| Value::undefined()) = val
                }
            }
        },
        ValueBase::Arguments => {
            match member.val {
                // Index
                ValueBase::Number(n) if n - n.floor() == 0.0 => unsafe {
                    (**self_.state.scope.last().unwrap()).set_arguments_nth_value(n as usize, val);
                },
                // TODO: 'length'
                _ => {}
            }
        }
        _ => {}
    };

    Ok(())
}

fn jmp(self_: &mut VM, iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // jmp
    get_int32!(self_, iseq, dst, i32);
    self_.state.pc += dst as isize;
    Ok(())
}

fn jmp_if_false(self_: &mut VM, iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // jmp_if_false
    get_int32!(self_, iseq, dst, i32);
    let cond = self_.state.stack.pop().unwrap();
    if let ValueBase::Bool(false) = cond.val {
        self_.state.pc += dst as isize
    }
    Ok(())
}

pub fn call_function(
    self_: &mut VM,
    id: FuncId,
    iseq: &ByteCode,
    args: &Vec<Value>,
    mut callobj: CallObject,
) -> Result<(), RuntimeError> {
    let argc = args.len();
    let mut args_all_numbers = true;
    let mut rest_args = vec![];
    let mut rest_param_name = None;
    for (i, arg) in args.iter().enumerate() {
        if let Some(name) = callobj.get_parameter_nth_name(i) {
            // When rest parameter
            if callobj.params[i].1 {
                rest_param_name = Some(name);
                rest_args.push(arg.clone());
            } else {
                callobj.set_value(name, arg.clone());
            }
        } else {
            rest_args.push(arg.clone());
        }

        match &arg.val {
            &ValueBase::Number(_) => {}
            _ => args_all_numbers = false,
        }
    }
    if let Some(rest_param_name) = rest_param_name {
        callobj.set_value(
            rest_param_name,
            Value::array(gc::new(ArrayValue::new(rest_args))),
        );
    } else {
        for arg in rest_args {
            callobj.arg_rest_vals.push(arg.clone());
        }
    }

    self_.state.scope.push(gc::new(callobj));

    if args_all_numbers {
        let scope = (*self_.state.scope.last().unwrap()).clone();
        if let Some(f) = unsafe {
            self_
                .jit
                .can_jit(id, iseq, &*scope, &self_.const_table, argc)
        } {
            self_
                .state
                .stack
                .push(unsafe { self_.jit.run_llvm_func(id, f, &args) });
            self_.state.scope.pop();
            return Ok(());
        }
    }

    self_
        .state
        .history
        .push((self_.state.stack.len(), self_.state.pc));
    self_.state.pc = 0;

    let save_id = self_.cur_func_id;
    self_.cur_func_id = id;

    self_.do_run(iseq)?;

    self_.cur_func_id = save_id;
    self_.state.scope.pop();

    self_
        .jit
        .record_function_return_type(id, self_.state.stack.last().unwrap());

    Ok(())
}

fn call(self_: &mut VM, iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // Call
    get_int32!(self_, iseq, argc, usize);

    let callee = self_.state.stack.pop().unwrap();

    let mut args = vec![];
    for _ in 0..argc {
        args.push(self_.state.stack.pop().unwrap());
    }

    match callee.val {
        ValueBase::BuiltinFunction(box (ref info, _, ref callobj)) => {
            (info.func)(self_, &args, callobj);
        }
        ValueBase::Function(box (id, ref iseq, _, ref callobj)) => {
            let mut callobj = callobj.clone();
            callobj.vals = gc::new(FxHashMap::default());
            call_function(self_, id, iseq, &args, callobj)?;
        }
        c => {
            return Err(RuntimeError::Type(format!(
                "type error(pc:{}): '{:?}' is not a function but called",
                self_.state.pc, c
            )));
        }
    };

    Ok(())
}

fn return_(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    let len = self_.state.stack.len();
    if let Some((previous_sp, return_pc)) = self_.state.history.pop() {
        self_.state.stack.drain(previous_sp..len - 1);
        self_.state.pc = return_pc;
    } else {
        unreachable!()
    }
    Ok(())
}

fn double(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // double
    let stack_top_val = self_.state.stack.last().unwrap().clone();
    self_.state.stack.push(stack_top_val);
    Ok(())
}

fn pop(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // double
    self_.state.stack.pop();
    Ok(())
}

// 'land' and 'lor' are for JIT compiler. Nope for VM.

fn land(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // land
    Ok(())
}

fn lor(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1; // lor
    Ok(())
}

fn set_cur_callobj(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1;
    if let Some(Value {
        val: ValueBase::Function(box (_, _, _, ref mut callobj)),
        ..
    }) = self_.state.stack.last_mut()
    {
        callobj.parent = Some(self_.state.scope.last().unwrap().clone());
    }
    Ok(())
}

fn get_name(self_: &mut VM, iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1;
    get_int32!(self_, iseq, name_id, usize);
    let name = &self_.const_table.string[name_id];
    let val = unsafe { (**self_.state.scope.last().unwrap()).get_value(name)? };
    self_.state.stack.push(val);
    Ok(())
}

fn set_name(self_: &mut VM, iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1;
    get_int32!(self_, iseq, name_id, usize);
    let name = self_.const_table.string[name_id].clone();
    let mut val = self_.state.stack.pop().unwrap();

    // We have to change cobj.this to the current scope one. (./examples/this.js)
    if let ValueBase::Function(box (_, _, _, ref mut cobj))
    | ValueBase::BuiltinFunction(box (_, _, ref mut cobj)) = &mut val.val
    {
        unsafe {
            cobj.this = (**self_.state.scope.last().unwrap()).this.clone();
        }
    }

    unsafe { (**self_.state.scope.last().unwrap()).set_value_if_exist(name, val) };

    Ok(())
}

fn decl_var(self_: &mut VM, iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1;
    get_int32!(self_, iseq, name_id, usize);
    let name = self_.const_table.string[name_id].clone();
    let mut val = self_.state.stack.pop().unwrap();

    // We have to change cobj.this to the current scope one. (./examples/this.js)
    if let ValueBase::Function(box (_, _, _, ref mut cobj))
    | ValueBase::BuiltinFunction(box (_, _, ref mut cobj)) = &mut val.val
    {
        unsafe {
            cobj.this = (**self_.state.scope.last().unwrap()).this.clone();
        }
    }

    unsafe {
        (**self_.state.scope.last().unwrap()).set_value(name, val);
    }

    Ok(())
}

// 'cond_op' is for JIT compiler. Nope for VM.
fn cond_op(self_: &mut VM, _iseq: &ByteCode) -> Result<(), RuntimeError> {
    self_.state.pc += 1;
    Ok(())
}

fn loop_start(self_: &mut VM, iseq: &ByteCode) -> Result<(), RuntimeError> {
    let loop_start = self_.state.pc as usize;

    self_.state.pc += 1;
    get_int32!(self_, iseq, loop_end, usize);

    let id = self_.cur_func_id;

    if let Some(pc) = unsafe {
        self_.jit.can_loop_jit(
            id,
            &iseq,
            &self_.const_table,
            &mut self_.state,
            loop_start,
            loop_end,
        )
    } {
        self_.state.pc = pc;
    }

    Ok(())
}