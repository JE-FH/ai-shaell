use crate::ast::FunctionBody;
use crate::error::RuntimeError;
use crate::gc::{GcHeap, GcRef, Remap, Trace};
use crate::scope::ScopeManager;
use std::collections::HashMap;
use std::fmt;

pub type ValueRef = GcRef<Value>;

/// The unified value enum — every Shæll value is one of these variants.
/// Stored in the GC heap via `heap.allocate(Value::Number(42.0))`.
#[derive(Clone)]
pub enum Value {
    Number(f64),
    String(String),
    BString(Vec<u8>),
    Bool(bool),
    Null,
    File(String),
    Table(TableData),
    Function(FuncData),
    NativeFunction(NativeFuncData),
}

// ===== TableData =====
#[derive(Clone)]
pub struct TableData {
    pub fields: HashMap<String, ValueRef>,
    pub array: Vec<ValueRef>,
}

// ===== FuncData =====
#[derive(Clone)]
pub struct FuncData {
    pub name: String,
    pub params: Vec<String>,
    pub body: FunctionBody,
    pub captured_scope: ScopeManager,
}

// ===== NativeFuncData =====
pub type NativeFn = std::rc::Rc<dyn Fn(&GcHeap, &[ValueRef]) -> Result<ValueRef, RuntimeError>>;

#[derive(Clone)]
pub struct NativeFuncData {
    pub name: String,
    pub func: NativeFn,
}

// ===== Trace implementations =====
impl Trace for Value {
    fn trace(&self, visit: &mut dyn FnMut(usize)) {
        match self {
            Value::Table(t) => t.trace(visit),
            Value::Function(f) => f.trace(visit),
            Value::NativeFunction(n) => n.trace(visit),
            _ => {}
        }
    }
}

impl Trace for TableData {
    fn trace(&self, visit: &mut dyn FnMut(usize)) {
        for v in self.fields.values() {
            visit(v.offset());
        }
        for v in &self.array {
            visit(v.offset());
        }
    }
}

impl Trace for FuncData {
    fn trace(&self, visit: &mut dyn FnMut(usize)) {
        // ScopeManager may contain GcRefs; trace them
        self.captured_scope.trace(visit);
    }
}

impl Trace for NativeFuncData {
    fn trace(&self, _: &mut dyn FnMut(usize)) {}
}

// ===== Remap implementations =====
impl Remap for Value {
    fn remap(&mut self, map: &[(usize, usize)]) {
        match self {
            Value::Table(t) => t.remap(map),
            Value::Function(f) => f.remap(map),
            Value::NativeFunction(n) => n.remap(map),
            _ => {}
        }
    }
}

impl Remap for TableData {
    fn remap(&mut self, map: &[(usize, usize)]) {
        for v in self.fields.values_mut() {
            remap_ref(v, map);
        }
        for v in &mut self.array {
            remap_ref(v, map);
        }
    }
}

impl Remap for FuncData {
    fn remap(&mut self, map: &[(usize, usize)]) {
        self.captured_scope.remap(map);
    }
}

impl Remap for NativeFuncData {
    fn remap(&mut self, _: &[(usize, usize)]) {}
}

fn remap_ref(r: &mut GcRef<Value>, map: &[(usize, usize)]) {
    if r.is_null() {
        return;
    }
    for &(old, new) in map {
        if old == r.offset() {
            *r = GcRef::from_offset(new);
            return;
        }
    }
}

// ===== Value methods (replacing IValue trait) =====
impl Value {
    pub fn get_type_name(&self) -> &str {
        match self {
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::BString(_) => "bstring",
            Value::Bool(_) => "bool",
            Value::Null => "null",
            Value::File(_) => "file",
            Value::Table(_) => "table",
            Value::Function(_) => "function",
            Value::NativeFunction(_) => "function",
        }
    }

    pub fn to_bool(&self) -> Result<bool, RuntimeError> {
        match self {
            Value::Number(n) => Ok(*n != 0.0),
            Value::String(s) => Ok(!s.is_empty()),
            Value::BString(b) => Ok(!b.is_empty()),
            Value::Bool(b) => Ok(*b),
            Value::Null => Ok(false),
            Value::Table(t) => Ok(!t.fields.is_empty() || !t.array.is_empty()),
            _ => Ok(true),
        }
    }

    pub fn to_number(&self) -> Result<f64, RuntimeError> {
        match self {
            Value::Number(n) => Ok(*n),
            Value::String(s) => s
                .parse::<f64>()
                .map_err(|_| RuntimeError::new(format!("Cannot convert '{}' to number", s))),
            Value::BString(b) => String::from_utf8_lossy(b)
                .parse::<f64>()
                .map_err(|_| RuntimeError::new("Cannot convert bstring to number".to_string())),
            Value::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
            Value::Null => Ok(0.0),
            _ => Err(RuntimeError::new(format!(
                "Cannot convert {} to number",
                self.get_type_name()
            ))),
        }
    }

    pub fn to_sstring(&self) -> Result<String, RuntimeError> {
        match self {
            Value::String(s) => Ok(s.clone()),
            Value::Number(n) => Ok(if *n == n.trunc() && n.is_finite() {
                format!("{}", *n as i64)
            } else {
                format!("{}", n)
            }),
            Value::Bool(b) => Ok(if *b {
                "true".to_string()
            } else {
                "false".to_string()
            }),
            Value::Null => Ok("null".to_string()),
            Value::BString(b) => Ok(String::from_utf8_lossy(b).to_string()),
            Value::File(p) => Ok(p.clone()),
            _ => Ok(self.get_type_name().to_string()),
        }
    }

    pub fn to_file_path(&self) -> Result<String, RuntimeError> {
        match self {
            Value::File(p) => Ok(p.clone()),
            _ => Err(RuntimeError::new(format!(
                "Cannot convert {} to file",
                self.get_type_name()
            ))),
        }
    }

    pub fn is_equal(&self, other: &Value) -> Result<bool, RuntimeError> {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => Ok((a - b).abs() < f64::EPSILON),
            (Value::String(a), Value::String(b)) => Ok(a == b),
            (Value::Bool(a), Value::Bool(b)) => Ok(a == b),
            (Value::Null, Value::Null) => Ok(true),
            (Value::File(a), Value::File(b)) => Ok(a == b),
            (Value::BString(a), Value::BString(b)) => Ok(a == b),
            _ => Ok(false),
        }
    }

    pub fn serialize(&self) -> String {
        match self {
            Value::Number(n) => {
                if *n == n.trunc() && n.is_finite() {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            Value::String(s) => s.clone(),
            Value::Bool(b) => {
                if *b {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            Value::Null => "null".to_string(),
            Value::File(p) => format!("@\"{}\"", p),
            Value::Table(t) => {
                let mut parts: Vec<String> =
                    t.fields.keys().map(|k| format!("{} = ...", k)).collect();
                for (i, _) in t.array.iter().enumerate() {
                    parts.push(format!("[{}] = ...", i));
                }
                format!("{{ {} }}", parts.join(", "))
            }
            Value::Function(f) => format!("<function {}>", f.name),
            Value::NativeFunction(n) => format!("<native function {}>", n.name),
            Value::BString(b) => String::from_utf8_lossy(b).to_string(),
        }
    }

    /// Index into a value. For Table lookups, returns the stored GcRef directly.
    /// For String/Number methods, allocates on the heap.
    /// NOTE: Avoid calling this inside a `with_ref` on the same heap as `heap`
    /// parameter — clone the value first to avoid nested buffer borrows.
    pub fn index(&self, key: &Value, heap: &GcHeap) -> Result<ValueRef, RuntimeError> {
        match self {
            Value::Table(t) => match key {
                Value::Number(n) => {
                    let idx = *n as usize;
                    if idx < t.array.len() {
                        Ok(t.array[idx])
                    } else {
                        Ok(heap.allocate(Value::Null))
                    }
                }
                _ => {
                    let s = key.to_sstring()?;
                    if let Some(v) = t.fields.get(&s) {
                        Ok(*v)
                    } else {
                        Ok(heap.allocate(Value::Null))
                    }
                }
            },
            Value::String(s) => {
                let method = key.to_sstring()?;
                if method == "length" {
                    Ok(heap.allocate(Value::Number(s.chars().count() as f64)))
                } else {
                    Err(RuntimeError::new(format!(
                        "String has no method '{}'",
                        method
                    )))
                }
            }
            Value::Number(n) => {
                let method = key.to_sstring()?;
                match method.as_str() {
                    "sqrt" => Ok(heap.allocate(Value::Number(n.sqrt()))),
                    "floor" => Ok(heap.allocate(Value::Number(n.floor()))),
                    "ceil" => Ok(heap.allocate(Value::Number(n.ceil()))),
                    "log2" => Ok(heap.allocate(Value::Number(n.log2()))),
                    "log" => Ok(heap.allocate(Value::Number(n.log10()))),
                    _ => Err(RuntimeError::new(format!(
                        "Number has no method '{}'",
                        method
                    ))),
                }
            }
            _ => Err(RuntimeError::new(format!(
                "Cannot index into {}",
                self.get_type_name()
            ))),
        }
    }

    pub fn set_index(&mut self, key: &Value, val: ValueRef) -> Result<(), RuntimeError> {
        match self {
            Value::Table(t) => match key {
                Value::Number(n) => {
                    let idx = *n as usize;
                    if idx >= t.array.len() {
                        t.array.resize(idx + 1, GcRef::NULL);
                    }
                    t.array[idx] = val;
                    Ok(())
                }
                _ => {
                    let s = key.to_sstring()?;
                    t.fields.insert(s, val);
                    Ok(())
                }
            },
            _ => Err(RuntimeError::new(format!(
                "Cannot set index on {}",
                self.get_type_name()
            ))),
        }
    }

    /// Get all keys of this value (for foreach iteration).
    /// Needs heap access to allocate key Value objects.
    pub fn get_keys(&self, heap: &GcHeap) -> Result<Vec<ValueRef>, RuntimeError> {
        match self {
            Value::Table(t) => {
                let mut keys: Vec<ValueRef> = t
                    .fields
                    .keys()
                    .map(|k| heap.allocate(Value::String(k.clone())))
                    .collect();
                for i in 0..t.array.len() {
                    keys.push(heap.allocate(Value::Number(i as f64)));
                }
                Ok(keys)
            }
            _ => Err(RuntimeError::new(format!(
                "Cannot iterate over {}",
                self.get_type_name()
            ))),
        }
    }
}

// ===== Convenience constructors =====
impl Value {
    pub fn number(n: f64) -> Self {
        Value::Number(n)
    }
    pub fn string(s: String) -> Self {
        Value::String(s)
    }
    pub fn bool_val(b: bool) -> Self {
        Value::Bool(b)
    }
    pub fn null() -> Self {
        Value::Null
    }
    pub fn file(path: String) -> Self {
        Value::File(path)
    }
    pub fn table() -> Self {
        Value::Table(TableData {
            fields: HashMap::new(),
            array: Vec::new(),
        })
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.serialize())
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "Number({})", n),
            Value::String(s) => write!(f, "String({:?})", s),
            Value::Bool(b) => write!(f, "Bool({})", b),
            Value::Null => write!(f, "Null"),
            Value::File(p) => write!(f, "File({})", p),
            Value::Table(_) => write!(f, "Table"),
            Value::Function(fd) => write!(f, "Function({})", fd.name),
            Value::NativeFunction(nd) => write!(f, "NativeFunction({})", nd.name),
            Value::BString(b) => write!(f, "BString({} bytes)", b.len()),
        }
    }
}
