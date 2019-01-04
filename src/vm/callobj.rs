use rustc_hash::FxHashMap;
use std::collections::hash_map::Entry;

use super::error::RuntimeError;
use super::value::*;
use gc;
use gc::GcType;

pub type CallObjectRef = GcType<CallObject>;

#[derive(Clone, Debug)]
pub struct CallObject {
    /// map of variables belong to the scope.
    pub vals: PropMap,
    /// name of rest parameters. (if the function has no rest parameters, None.)
    pub rest_params: Option<String>,
    /// set of the name of parameter corresponds to applied arguments when the function was invoked.
    pub arguments: Vec<(Option<String>, Value)>,
    /// this value.
    pub this: Box<Value>,
    /// reference to the outer scope.
    pub parent: Option<CallObjectRef>,
}

impl PartialEq for CallObject {
    fn eq(&self, other: &CallObject) -> bool {
        self.vals == other.vals && self.parent == other.parent
    }
}

impl CallObject {
    pub fn new(this: Value) -> CallObject {
        CallObject {
            vals: gc::new(FxHashMap::default()),
            rest_params: None,
            arguments: vec![],
            this: Box::new(this),
            parent: None,
        }
    }

    pub fn new_global() -> CallObjectRef {
        let vals = gc::new(FxHashMap::default());
        let callobj = gc::new(CallObject {
            vals: vals.clone(),
            rest_params: None,
            arguments: vec![],
            this: Box::new(Value::Undefined),
            parent: None,
        });
        unsafe {
            *(*callobj).this = Value::Object(vals, ObjectKind::Ordinary);
        }
        callobj
    }

    pub fn apply_arguments(&mut self, func_info: FuncInfo, args: &Vec<Value>) {
        for (name, _) in &func_info.params {
            self.set_value(name.to_string(), Value::Undefined);
        }
        let mut rest_args = vec![];
        let mut rest_param_name = None;
        self.arguments.clear();

        for (i, arg) in args.iter().enumerate() {
            if let Some(name) = self.get_parameter_nth_name(func_info.clone(), i) {
                // When rest parameter. TODO: More features of rest parameter
                if func_info.params[i].1 {
                    self.arguments.push((None, arg.clone()));
                    rest_param_name = Some(name);
                    rest_args.push(arg.clone());
                } else {
                    self.arguments.push((Some(name.clone()), arg.clone()));
                    self.set_value(name.clone(), arg.clone());
                }
            } else {
                self.arguments.push((None, arg.clone()));
                rest_args.push(arg.clone());
            }
        }

        if let Some(rest_param_name) = rest_param_name {
            self.set_value(rest_param_name.clone(), Value::array_from_elems(rest_args));
            self.rest_params = Some(rest_param_name);
        };
    }

    pub fn set_value(&mut self, name: String, val: Value) {
        unsafe {
            (*self.vals).insert(name, Property::new(val));
        }
    }

    pub fn set_value_if_exist(&mut self, name: String, val: Value) {
        unsafe {
            match (*self.vals).entry(name.clone()) {
                Entry::Occupied(ref mut v) => *v.get_mut() = Property::new(val),
                Entry::Vacant(v) => {
                    match self.parent {
                        Some(ref parent) => return (**parent).set_value_if_exist(name, val),
                        None => v.insert(Property::new(val)),
                    };
                }
            }
        }
    }

    pub fn get_value(&self, name: &String) -> Result<Value, RuntimeError> {
        unsafe {
            if let Some(prop) = (*self.vals).get(name) {
                return Ok(prop.val.clone());
            }
            match self.parent {
                Some(ref parent) => (**parent).get_value(name),
                None => Err(RuntimeError::Reference(format!(
                    "reference error: '{}' is not defined",
                    name
                ))),
            }
        }
    }

    pub fn get_local_value(&self, name: &String) -> Result<Value, RuntimeError> {
        if let Some(prop) = unsafe { (*self.vals).get(name) } {
            Ok(prop.val.clone())
        } else {
            Err(RuntimeError::General(
                "get_local_value(): the argument did not found in local scope.".to_string(),
            ))
        }
    }

    pub fn get_arguments_nth_value(&self, n: usize) -> Result<Value, RuntimeError> {
        if n < self.arguments.len() {
            match self.arguments[n].0.clone() {
                Some(name) => self.get_local_value(&name),
                None => Ok(self.arguments[n].1.clone()),
            }
        } else {
            Ok(Value::Undefined)
        }
    }

    /// set the nth element of callObject.argument to val:Value.
    pub fn set_arguments_nth_value(&mut self, n: usize, val: Value) {
        if n < self.arguments.len() {
            let param_name = self.arguments[n].0.clone();
            if let Some(param_name) = param_name {
                self.set_value(param_name, val);
            } else {
                self.arguments[n].1 = val;
            }
        }
    }

    /// get length of callObject.arguments
    pub fn get_arguments_length(&self) -> usize {
        self.arguments.len()
    }

    /// get name of the nth element of func_info.params.
    pub fn get_parameter_nth_name(&self, func_info: FuncInfo, n: usize) -> Option<String> {
        if n < func_info.params.len() {
            return Some(func_info.params[n].0.clone());
        }
        None
    }
}
