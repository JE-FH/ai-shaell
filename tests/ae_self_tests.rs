//! Shæll self-tests — language features tested from within Shæll scripts.
//! Each test module runs a Shæll script through the interpreter.

use shaell::interpreter::Interpreter;
use shaell::parser::Parser;

fn run_ae(source: &str) -> String {
    let mut parser = Parser::new(source);
    let program = parser.parse_program().expect("parse failed");
    let mut interpreter = Interpreter::new();
    let result = interpreter
        .execute_program(&program)
        .expect("execution failed");
    interpreter.heap.with_ref(result, |v| v.to_string())
}

#[test]
fn test_variables_and_assignment() {
    let src = r#"
let T = {}; let out = ""
T:assert = fn (c, m) if not c then throw m end end
T:assert(42 == 42, "eq")
let x = 5; x = 10; T:assert(x == 10, "reassign")
let y; T:assert(y == null, "uninit null")
newvar = 99; T:assert(newvar == 99, "create missing")
let z = 5; z += 3; T:assert(z == 8, "+=")
z = 10; z -= 3; T:assert(z == 7, "-=")
z = 4; z *= 3; T:assert(z == 12, "*=")
z = 10; z /= 2; T:assert(z == 5, "/=")
z = 10; z %= 3; T:assert(z == 1, "%=")
z = 2; z **= 3; T:assert(z == 8, "**=")
out = "variables ok"
out
"#;
    assert!(run_ae(src).contains("variables ok"));
}

#[test]
fn test_arithmetic() {
    let src = r#"
let T = {}; let out = ""
T:assert = fn (c, m) if not c then throw m end end
T:assert(2 + 3 == 5, "add")
T:assert(10 - 3 == 7, "sub")
T:assert(4 * 5 == 20, "mul")
T:assert(10 / 2 == 5, "div")
T:assert(10 / 4 == 2.5, "float div")
T:assert(10 % 3 == 1, "mod")
T:assert(2 ** 8 == 256, "pow")
T:assert(2 + 3 * 4 == 14, "precedence")
T:assert((2 + 3) * 4 == 20, "parens")
out = "arithmetic ok"
out
"#;
    assert!(run_ae(src).contains("arithmetic ok"));
}

#[test]
fn test_strings() {
    let src = r#"
let T = {}; let out = ""
T:assert = fn (c, m) if not c then throw m end end
T:assert("hello " + "world" == "hello world", "concat")
T:assert("x" + "" == "x", "str+empty")
T:assert("v: " + 42 == "v: 42", "str+num")
T:assert("hi" * 3 == "hihihi", "repeat")
T:assert("x" * 0 == "", "repeat 0")
let x = 42; T:assert("ans is ${x}" == "ans is 42", "interp")
T:assert(+"42" == 42, "+str to num")
out = "strings ok"
out
"#;
    assert!(run_ae(src).contains("strings ok"));
}

#[test]
fn test_control_flow() {
    let src = r#"
let T = {}; let out = ""
T:assert = fn (c, m) if not c then throw m end end
let x = 0; if true then x = 1 end; T:assert(x == 1, "if true")
if false then x = 99 else x = 2 end; T:assert(x == 2, "else")
if 1 then x = 1 end; T:assert(x == 1, "nonzero truthy")
if 0 then x = 99 end; T:assert(x == 1, "zero falsy")
let i = 0; while i < 5 do i = i + 1 end; T:assert(i == 5, "while")
let c = 0; while false do c = c + 1 end; T:assert(c == 0, "while never")
let s = 0; for let j = 0, j < 5, j = j + 1 do s = s + j end; T:assert(s == 10, "for")
let t = {[0]=10,[1]=20,[2]=30}; let s2 = 0; foreach v in t do s2 = s2 + v end; T:assert(s2 == 60, "foreach")
out = "control flow ok"
out
"#;
    assert!(run_ae(src).contains("control flow ok"));
}

#[test]
fn test_break() {
    let src = r#"
let T = {}; let out = ""
T:assert = fn (c, m) if not c then throw m end end
let x = 0; while true do x = x + 1; if x >= 5 then break end; end; T:assert(x == 5, "break while")
let s = 0; for let i = 0, i < 10, i = i + 1 do s = s + i; if i >= 4 then break end; end; T:assert(s == 10, "break for")
out = "break ok"
out
"#;
    assert!(run_ae(src).contains("break ok"));
}

#[test]
fn test_functions() {
    let src = r#"
let T = {}; let out = ""
T:assert = fn (c, m) if not c then throw m end end
fn add(a,b)=>a+b; T:assert(add(3,4)==7,"add")
fn square(x)=>x*x; T:assert(square(5)==25,"square")
let triple=fn(x)=>x*3; T:assert(triple(7)==21,"triple")
fn answer()=>42; T:assert(answer()==42,"no-arg")
let base=10; fn add_base(x)=>x+base; T:assert(add_base(5)==15,"closure")
fn make_counter() let n=0; fn count() n=n+1;return n end;return count end
let c=make_counter(); T:assert(c()==1,"c1"); T:assert(c()==2,"c2"); T:assert(c()==3,"c3")
fn apply(f,x)=>f(x); fn double(n)=>n*2; T:assert(apply(double,7)==14,"apply")
fn fac(n) if n<=1 then return 1 end; return n*fac(n-1) end; T:assert(fac(5)==120,"fac")
fn early(x) if x>5 then return "big" end; return "small" end
T:assert(early(3)=="small","early small"); T:assert(early(10)=="big","early big")
out = "functions ok"
out
"#;
    assert!(run_ae(src).contains("functions ok"));
}

#[test]
fn test_tables() {
    let src = r#"
let T = {}; let out = ""
T:assert = fn (c, m) if not c then throw m end end
let t={name="Alice",age=30}
T:assert(t:name=="Alice","field")
T:assert(t["name"]=="Alice","bracket")
T:assert(t:age==30,"num field")
t:age=31; T:assert(t:age==31,"update")
let a={[0]="zero",[1]="one"}; T:assert(a[0]=="zero","idx0"); T:assert(a[1]=="one","idx1")
let m={a=1}; T:assert(m:b==null,"missing key")
let ct={count=10}; ct:count+=5; T:assert(ct:count==15,"table +=")
out = "tables ok"
out
"#;
    assert!(run_ae(src).contains("tables ok"));
}

#[test]
fn test_logic() {
    let src = r#"
let T = {}; let out = ""
T:assert = fn (c, m) if not c then throw m end end
T:assert(5==5,"eq"); T:assert(5!=3,"neq")
T:assert(5>3,"gt"); T:assert(3<5,"lt"); T:assert(5>=5,"geq")
T:assert(true&&true,"and"); T:assert(true||false,"or")
T:assert(not true==false,"not true"); T:assert(not false==true,"not false")
T:assert(not 1==false,"not 1"); T:assert(not 0==true,"not 0")
fn ex() throw "nope" end; let r=false&&ex(); T:assert(not r,"sc and")
let r2=true||ex(); T:assert(r2,"sc or")
T:assert(not(5=="5"),"cross num!=str")
T:assert(not(true==1),"cross bool!=num")
T:assert(not(null==false),"cross null!=bool")
out = "logic ok"
out
"#;
    assert!(run_ae(src).contains("logic ok"));
}

#[test]
fn test_try_throw() {
    let src = r#"
let T = {}; let out = ""
T:assert = fn (c, m) if not c then throw m end end
let r=try 42 end; T:assert(r:status==0,"status"); T:assert(r:value==42,"value")
let e=try throw "err" end; T:assert(e:status!=0,"caught"); T:assert(e:error!=null,"has error")
out = "try ok"
out
"#;
    assert!(run_ae(src).contains("try ok"));
}

#[test]
fn test_bang_command() {
    let src = r#"
let T = {}; let out = ""
T:assert = fn (c, m) if not c then throw m end end
!echo with ("hello") -> let captured
T:assert(captured != null, "!echo captures")
let msg="eval"
!echo with (msg) -> let captured2
T:assert(captured2 != null, "!echo with(var)")
out = "bang ok"
out
"#;
    assert!(run_ae(src).contains("bang ok"));
}
