//! cuda-compiler-agent — Deliberation bytecode engine in Rust

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Op { Push, Pop, Dup, Swap, Add, Sub, Mul, Div, Mod, And, Or, Not,
    Eq, Ne, Lt, Gt, Lte, Gte, Jmp, Jz, Jnz, Call, Ret, Halt,
    Consider, Resolve, Intent, Emit, Explain, Learn, Load, Store, LoadAttr }

#[derive(Debug, Clone)]
pub struct TensorCell { pub value: f64, pub confidence: f64 }

impl TensorCell {
    pub fn new(v: f64, c: f64) -> Self { TensorCell { value: v, confidence: c.clamp(0.0, 1.0) } }
    pub fn pure(v: f64) -> Self { TensorCell::new(v, 1.0) }
}

#[derive(Debug, Clone)]
pub struct Instruction { pub op: Op, pub operand: Option<String>, pub label: String, pub confidence: f64 }

impl Instruction {
    pub fn new(op: Op) -> Self { Instruction { op, operand: None, label: String::new(), confidence: 1.0 } }
    pub fn with_op(op: Op, o: &str) -> Self { Instruction { op, operand: Some(o.to_string()), label: String::new(), confidence: 1.0 } }
    pub fn labeled(op: Op, l: &str) -> Self { Instruction { op, operand: None, label: l.to_string(), confidence: 1.0 } }
}

pub struct DeliberationVM { pub stack: Vec<TensorCell>, pub variables: std::collections::HashMap<String, TensorCell>,
    pub instructions: Vec<Instruction>, pub pc: usize, pub log: Vec<String>, pub halted: bool }

impl DeliberationVM {
    pub fn new() -> Self { DeliberationVM { stack: Vec::new(), variables: std::collections::HashMap::new(), instructions: Vec::new(), pc: 0, log: Vec::new(), halted: false } }
    pub fn load(&mut self, i: Vec<Instruction>) { self.instructions = i; self.pc = 0; self.halted = false; }
    fn cc(a: f64, b: f64) -> f64 { a * b }
    pub fn step(&mut self) -> bool {
        if self.pc >= self.instructions.len() || self.halted { self.halted = true; return false; }
        let ins = self.instructions[self.pc].clone();
        match ins.op {
            Op::Push => { let v = ins.operand.and_then(|s| s.parse().ok()).unwrap_or(0.0); self.stack.push(TensorCell::new(v, ins.confidence)); }
            Op::Pop => { self.stack.pop(); }
            Op::Dup => { if let Some(t) = self.stack.last().cloned() { self.stack.push(t); } }
            Op::Add => { if let (Some(b),Some(a)) = (self.stack.pop(), self.stack.pop()) { self.stack.push(TensorCell::new(a.value+b.value, Self::cc(a.confidence,b.confidence))); } }
            Op::Sub => { if let (Some(b),Some(a)) = (self.stack.pop(), self.stack.pop()) { self.stack.push(TensorCell::new(a.value-b.value, Self::cc(a.confidence,b.confidence))); } }
            Op::Mul => { if let (Some(b),Some(a)) = (self.stack.pop(), self.stack.pop()) { self.stack.push(TensorCell::new(a.value*b.value, Self::cc(a.confidence,b.confidence))); } }
            Op::Lt => { if let (Some(b),Some(a)) = (self.stack.pop(), self.stack.pop()) { self.stack.push(TensorCell::new(if a.value<b.value{1.0}else{0.0}, Self::cc(a.confidence,b.confidence))); } }
            Op::Gt => { if let (Some(b),Some(a)) = (self.stack.pop(), self.stack.pop()) { self.stack.push(TensorCell::new(if a.value>b.value{1.0}else{0.0}, Self::cc(a.confidence,b.confidence))); } }
            Op::Eq => { if let (Some(b),Some(a)) = (self.stack.pop(), self.stack.pop()) { self.stack.push(TensorCell::new(if (a.value-b.value).abs()<1e-10{1.0}else{0.0}, Self::cc(a.confidence,b.confidence))); } }
            Op::Not => { if let Some(a) = self.stack.pop() { self.stack.push(TensorCell::new(if a.value<0.5{1.0}else{0.0}, a.confidence)); } }
            Op::Load => { if let Some(n) = &ins.operand { self.stack.push(self.variables.get(n).cloned().unwrap_or(TensorCell::new(0.0,0.0))); } }
            Op::Store => { if let Some(n) = ins.operand.clone() { if let Some(v) = self.stack.pop() { self.variables.insert(n, v); } } }
            Op::Jmp => { if let Some(pos) = self.instructions.iter().position(|i| i.label == ins.label) { self.pc = pos; return true; } }
            Op::Jz => { if let Some(top) = self.stack.pop() { if top.value < 0.5 { if let Some(pos) = self.instructions.iter().position(|i| i.label == ins.label) { self.pc = pos; return true; } } } }
            Op::Emit => { if let Some(t) = self.stack.last() { self.log.push(format!("EMIT: {}", t.value)); } }
            Op::Halt => { self.halted = true; return false; }
            Op::Consider => {} Op::Resolve => {} Op::Intent => { self.log.push(format!("INTENT: {}", ins.operand.unwrap_or_default())); }
            Op::Explain => {} Op::Learn => {}
            _ => {}
        }
        self.pc += 1; true
    }
    pub fn run(&mut self, max: usize) -> u32 { let mut s = 0; while s < max && self.step() { s += 1; } s }
}

#[cfg(test)]
mod tests { use super::*;
    #[test] fn test_arith() { let mut vm = DeliberationVM::new();
        vm.load(vec![Instruction::with_op(Op::Push,"10"), Instruction::with_op(Op::Push,"20"), Instruction::new(Op::Add), Instruction::new(Op::Halt)]);
        vm.run(100); assert!((vm.stack.last().unwrap().value-30.0).abs()<0.01); }
    #[test] fn test_conf() { let mut vm = DeliberationVM::new();
        let mut i1 = Instruction::with_op(Op::Push,"5"); i1.confidence = 0.8;
        let mut i2 = Instruction::with_op(Op::Push,"3"); i2.confidence = 0.9;
        vm.load(vec![i1, i2, Instruction::new(Op::Mul), Instruction::new(Op::Halt)]);
        vm.run(100); assert!((vm.stack.last().unwrap().confidence-0.72).abs()<0.01); }
}
