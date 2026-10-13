//! S3 ABI boundary gate. Measured Debug cost: 0.53 seconds.
//! Reproduce: cargo test --offline -p ulua --test s3_limits -- --nocapture.
//! Large cases: 64 KiB sources, one million interrupts, and 24 string doublings.
// This target uses peak counts; the live count serves the heap sampler only.
#[allow(dead_code)]
#[path = "../examples/support/alloc.rs"]
mod allocations;
// The growth gate uses the graph generators from this shared source module.
#[allow(dead_code)]
#[path = "../src/s3_inputs.rs"]
mod inputs;
use std::{ffi::CStr, ptr, time::Instant};
use ulua::*;
use ulua_compiler::{
    ARGUMENT_BOUND, DEPTH_CEILING, LOCAL_BUDGET, PARAMETER_BOUND, SOURCE_CEILING,
    TEMPLATE_PART_BOUND,
};

struct Script {
    context: *mut Context,
    module: *mut Module,
    instance: *mut Instance,
}
fn code(d: &Diagnostic) -> String {
    // SAFETY: The ABI always terminates the fixed diagnostic code array.
    unsafe {
        CStr::from_ptr(d.code.as_ptr())
            .to_string_lossy()
            .into_owned()
    }
}
fn message(d: &Diagnostic) -> String {
    // SAFETY: The ABI always terminates the fixed diagnostic message array.
    unsafe {
        CStr::from_ptr(d.message.as_ptr())
            .to_string_lossy()
            .into_owned()
    }
}
impl Script {
    fn compile(source: &str, limits: CompileLimits, expected: &str) -> Self {
        let mut s = Self {
            context: ptr::null_mut(),
            module: ptr::null_mut(),
            instance: ptr::null_mut(),
        };
        let mut d = Diagnostic::default();
        // SAFETY: Source and descriptors cover their lengths; the test serializes all object access.
        unsafe {
            assert_eq!(ulua_context_new(&mut s.context), ULUA_OK);
            let status = ulua_compile(
                s.context,
                source.as_ptr().cast(),
                source.len(),
                &limits,
                &mut s.module,
                &mut d,
            );
            if expected.is_empty() {
                assert_eq!(status, ULUA_OK, "{}: {}", code(&d), message(&d));
                assert!(!s.module.is_null());
            } else {
                assert_eq!(status, ULUA_REJECTED, "{}: {}", code(&d), message(&d));
                assert_eq!(code(&d), expected);
                assert!(s.module.is_null());
            }
        }
        s
    }
    fn new(source: &str) -> Self {
        Self::compile(source, CompileLimits::default(), "")
    }
    fn index(&self, name: &str) -> u32 {
        let mut index = u32::MAX;
        // SAFETY: The fixture owns a live module, name bytes, and output storage.
        unsafe {
            assert_eq!(
                ulua_module_find_export(self.module, name.as_ptr().cast(), name.len(), &mut index),
                ULUA_OK
            );
        }
        index
    }
    fn instance(&mut self, limits: RunLimits) -> (Status, String) {
        let mut d = Diagnostic::default();
        // SAFETY: The fixture owns both pointers and serializes their lifetime transitions.
        unsafe {
            ulua_instance_free(self.instance);
            self.instance = ptr::null_mut();
            let status = ulua_instance_new(
                self.context,
                self.module,
                &Host::default(),
                &limits,
                &mut self.instance,
                &mut d,
            );
            assert!(
                status == ULUA_OK || status == ULUA_FAILED,
                "{}: {}",
                code(&d),
                message(&d)
            );
            (status, code(&d))
        }
    }
    fn call(&self, index: u32, arguments: &[Value]) -> (Status, String, Value) {
        let mut d = Diagnostic::default();
        let mut value = Value::default();
        // SAFETY: The fixture owns a live instance and every argument and output covers its length.
        unsafe {
            let status = ulua_call(
                self.instance,
                index,
                arguments.as_ptr(),
                arguments.len(),
                &mut value,
                &mut d,
            );
            assert!(
                status == ULUA_OK || status == ULUA_FAILED,
                "{}: {}",
                code(&d),
                message(&d)
            );
            (status, code(&d), value)
        }
    }
}
impl Drop for Script {
    fn drop(&mut self) {
        // SAFETY: The fixture owns its context; context_free releases all remaining children once.
        unsafe {
            assert_eq!(ulua_context_free(self.context), ULUA_OK);
        }
    }
}
fn number(n: usize) -> Value {
    let mut value = Value::default();
    value.r#type = ULUA_TYPE_NUMBER;
    value.number = n as f64;
    value
}
fn pair(at: String, past: String, diagnostic: &str) {
    Script::new(&at);
    Script::compile(&past, CompileLimits::default(), diagnostic);
}
fn source_and_memory() {
    for kind in ["flat", "functions"] {
        let at = inputs::source_ceiling(kind, SOURCE_CEILING as usize);
        let baseline = allocations::start();
        let script = Script::new(&at);
        let peak = allocations::peak(baseline);
        // Compiler heap is outside the VM budget. The source ceiling bounds this measured envelope.
        assert!(
            peak <= 32 * 1024 * 1024,
            "compiler Rust heap {kind}: {peak}"
        );
        eprintln!("S3 compiler Rust heap {kind}: {peak} bytes");
        drop(script);
        let past = format!("{at} ");
        let mut limits = CompileLimits::default();
        limits.max_source_bytes = u64::MAX;
        Script::compile(&past, limits, "limit:source-bytes");
    }
    let source = inputs::bindings(1);
    let mut limits = CompileLimits::default();
    limits.max_source_bytes = source.len() as u64;
    Script::compile(&source, limits, "");
    limits.max_source_bytes -= 1;
    Script::compile(&source, limits, "limit:source-bytes");
    limits.max_source_bytes = 0;
    Script::compile(&source, limits, "limit:source-bytes");
}
fn nesting() {
    assert_eq!(DEPTH_CEILING, 48);
    assert_eq!(inputs::FORMS.len(), 13);
    for (form, levels) in [
        ("parentheses", 42),
        ("not", 42),
        ("minus", 42),
        ("conditional", 42),
        ("if", 22),
        ("else-if", 43),
        ("block", 44),
        ("while", 22),
        ("for", 22),
        ("call", 42),
        ("flat", 42),
        ("template", 42),
    ] {
        pair(
            inputs::nested(form, levels),
            inputs::nested(form, levels + 1),
            "limit:nesting-depth",
        );
        let mut clamped = CompileLimits::default();
        clamped.max_nesting_depth = u32::MAX;
        Script::compile(
            &inputs::nested(form, levels + 1),
            clamped,
            "limit:nesting-depth",
        );
    }
    // Assignment chains remain rejected; the guard must still bound their parser and AST depth.
    Script::compile(
        &inputs::nested("assignment", 41),
        CompileLimits::default(),
        "C11",
    );
    Script::compile(
        &inputs::nested("assignment", 42),
        CompileLimits::default(),
        "limit:nesting-depth",
    );
    let mut zero = CompileLimits::default();
    zero.max_nesting_depth = 0;
    Script::compile(&inputs::bindings(0), zero, "limit:nesting-depth");
    // Host limits below the library ceiling retain their meaning.
    let mut custom = CompileLimits::default();
    custom.max_nesting_depth = 16;
    Script::compile(&inputs::nested("parentheses", 10), custom, "");
    Script::compile(
        &inputs::nested("parentheses", 11),
        custom,
        "limit:nesting-depth",
    );
}
fn compiler_bounds() {
    pair(
        inputs::parameters(PARAMETER_BOUND, false, false),
        inputs::parameters(PARAMETER_BOUND + 1, false, false),
        "limit:parameters",
    );
    pair(
        inputs::parameters(PARAMETER_BOUND, true, false),
        inputs::parameters(PARAMETER_BOUND + 1, true, false),
        "limit:parameters",
    );
    let at = inputs::parameters(PARAMETER_BOUND, false, true);
    let past = at.replace(
        &format!("f({});", vec!["1"; ARGUMENT_BOUND].join(",")),
        &format!("f({});", vec!["1"; ARGUMENT_BOUND + 1].join(",")),
    );
    pair(at, past, "limit:arguments");
    // A template always has 2*n+1 parts; the next substitution adds two parts.
    pair(
        inputs::template((TEMPLATE_PART_BOUND - 1) / 2),
        inputs::template(TEMPLATE_PART_BOUND.div_ceil(2)),
        "limit:template-parts",
    );
    // Ended loops release generated locals. Nested loops retain their live copies.
    Script::new(&inputs::loops(200));
    let head = (0..32)
        .map(|i| format!("i{i}=0"))
        .collect::<Vec<_>>()
        .join(",");
    let nested = |heads: usize, tails: usize| {
        format!(
            "{}{}{}",
            format!("for(let {head};false;){{").repeat(heads),
            "for(;false;){".repeat(tails),
            "}".repeat(heads + tails)
        )
    };
    let module = |body| format!("export function main():void{{{body}}}");
    // 132 generated locals, two shared slots, and two tails reach 136.
    pair(
        module(nested(4, 2)),
        module(nested(4, 3)),
        "limit:temporaries",
    );
    // Defaults add 32 copies. Three heads and three tails reach the same bound.
    let defaults = |tails| {
        inputs::parameters(PARAMETER_BOUND, true, false).replacen(
            ":void {}",
            &format!(":void {{{}}}", nested(3, tails)),
            1,
        )
    };
    pair(defaults(3), defaults(4), "limit:temporaries");
    let at = inputs::bindings(LOCAL_BUDGET - 1);
    let past = inputs::bindings(LOCAL_BUDGET);
    // Both sides compile through luau_compile. The next binding uses a typed spill table.
    Script::new(&at);
    Script::new(&past);
    let emit = |s: &str| {
        ulua_compiler::compile(s.as_bytes(), ulua_compiler::Limits::default())
            .unwrap()
            .luau
    };
    assert!(!emit(&at).contains("local __ulua_spill"));
    assert!(emit(&past).contains("local __ulua_spill"));
}
fn combined() {
    let mut body = String::new();
    for level in 0..42 {
        body.push('{');
        for binding in 0..16 {
            body.push_str(&format!("const a{level}_{binding}=1;"));
        }
    }
    body.push_str(&"}".repeat(42));
    let nested = format!("export function main():void {{{body}}}");
    Script::compile(
        &format!("export function main():void {{ {{ {body} }} }}"),
        CompileLimits::default(),
        "limit:nesting-depth",
    );
    let mut s = Script::new(&nested);
    assert_eq!(s.instance(RunLimits::default()).0, ULUA_OK);
    assert_eq!(s.call(0, &[]).0, ULUA_OK);
    // Module captures span both ordinary locals and spill fields, beside the maximum if-chain depth.
    let mut source = String::new();
    for i in 0..224 {
        source.push_str(&format!("const a{i}=1;"));
    }
    source.push_str("export function main():void {let sum=0;");
    for i in 0..224 {
        source.push_str(&format!("sum+=a{i};"));
    }
    source.push_str(&"if(false) {} else ".repeat(43));
    source.push_str("{} print(String(sum));}");
    let mut s = Script::new(&source);
    assert_eq!(s.instance(RunLimits::default()).0, ULUA_OK);
    assert_eq!(s.call(0, &[]).0, ULUA_OK);
    // All parameters and all arguments coexist with a full local budget.
    let declaration = format!(
        "export function f({}):number {{let sum=0;{} return sum;}}",
        (0..PARAMETER_BOUND)
            .map(|i| format!("p{i}:number"))
            .collect::<Vec<_>>()
            .join(","),
        (0..PARAMETER_BOUND)
            .map(|i| format!("sum+=p{i};"))
            .collect::<String>()
    );
    let mut main = (0..LOCAL_BUDGET)
        .map(|i| format!("const a{i}=1;"))
        .collect::<String>();
    main.push_str(&format!(
        "f({});",
        (0..ARGUMENT_BOUND)
            .map(|i| format!("a{i}"))
            .collect::<Vec<_>>()
            .join(",")
    ));
    let mut s = Script::new(&format!(
        "{declaration} export function main():void {{{main}}}"
    ));
    assert_eq!(s.instance(RunLimits::default()).0, ULUA_OK);
    let args = vec![number(1); PARAMETER_BOUND];
    let result = s.call(s.index("f"), &args);
    assert_eq!(result.0, ULUA_OK);
    assert_eq!(result.2.number, PARAMETER_BOUND as f64);
    assert_eq!(s.call(s.index("main"), &[]).0, ULUA_OK);
}
fn memory_budget(bytes: u64) -> RunLimits {
    let mut limits = RunLimits::default();
    limits.max_memory_bytes = bytes;
    limits
}
#[cfg(feature = "measurements")]
fn native_ccalls() {
    // SAFETY: The optional probe owns its state and protects every recursive C call.
    #[link(name = "ulua_test_native", kind = "static")]
    unsafe extern "C" {
        fn ulua_internal_probe_ccalls(count: i32) -> i32;
        fn ulua_internal_probe_ccalls_protected(count: i32) -> i32;
    }
    // P1 cannot express pcall or a recursive C callback. Test the pinned VM fixture separately.
    // SAFETY: The probe takes only a count and releases its private VM before it returns.
    unsafe {
        assert_eq!(ulua_internal_probe_ccalls(198), 0);
        assert_eq!(ulua_internal_probe_ccalls(199), 2);
        assert_eq!(ulua_internal_probe_ccalls_protected(198), 0);
        assert_eq!(ulua_internal_probe_ccalls_protected(199), 2);
    }
}
fn runtime() {
    let source = "export function f(n:number):void {let i=0;while(i<n){i++;}} export function main():void {}";
    let mut s = Script::new(source);
    let index = s.index("f");
    assert_eq!(s.instance(RunLimits::default()).0, ULUA_OK);
    // The loop emits exactly one interrupt per backedge, plus the call-entry interrupt.
    let at = s.call(index, &[number(DEFAULT_STEPS as usize - 1)]);
    assert_eq!(at.0, ULUA_OK, "{}", at.1);
    let past = s.call(index, &[number(DEFAULT_STEPS as usize)]);
    assert_eq!((past.0, past.1), (ULUA_FAILED, "limit:steps".into()));
    assert_eq!(s.call(index, &[number(0)]).0, ULUA_OK);
    let mut zero = RunLimits::default();
    zero.max_steps = 0;
    assert_eq!(s.instance(zero), (ULUA_FAILED, "limit:steps".into()));
    let mut s = Script::new(inputs::recursion());
    let index = s.index("recurse");
    assert_eq!(s.instance(RunLimits::default()).0, ULUA_OK);
    // The equality helper uses one additional frame at the deepest comparison.
    assert_eq!(s.call(index, &[number(19996)]).0, ULUA_OK);
    let past = s.call(index, &[number(19997)]);
    assert_eq!((past.0, past.1), (ULUA_FAILED, "limit:stack".into()));
    assert_eq!(s.call(index, &[number(0)]).0, ULUA_OK);
    let source = "export function f(n:number):void {let text=\"x\";let i=0;while(i<n){text=`${text}${text}`;i++;}} export function main():void {}";
    let mut s = Script::new(source);
    let index = s.index("f");
    assert_eq!(s.instance(RunLimits::default()).0, ULUA_OK);
    assert_eq!(s.call(index, &[number(24)]).0, ULUA_OK);
    // Reset the VM so the allocation boundary does not depend on the previous GC schedule.
    assert_eq!(s.instance(RunLimits::default()).0, ULUA_OK);
    let past = s.call(index, &[number(25)]);
    assert_eq!((past.0, past.1), (ULUA_FAILED, "limit:memory".into()));
    assert_eq!(s.call(index, &[number(0)]).0, ULUA_OK);
    // Measure the allocator's exact byte threshold for VM creation and module initialization.
    let mut s = Script::new("export function main():void {}");
    let mut lo = 0;
    let mut hi = 1024 * 1024;
    while lo + 1 < hi {
        let mid = (lo + hi) / 2;
        let status = s.instance(memory_budget(mid));
        if status.0 == ULUA_OK {
            hi = mid;
        } else {
            assert_eq!(status.1, "limit:memory");
            lo = mid;
        }
    }
    assert_eq!(s.instance(memory_budget(hi)).0, ULUA_OK);
    assert_eq!(
        s.instance(memory_budget(hi - 1)),
        (ULUA_FAILED, "limit:memory".into())
    );
    eprintln!("S3 minimum instance memory: {hi} bytes");
    assert_eq!(
        s.instance(memory_budget(0)),
        (ULUA_FAILED, "limit:memory".into())
    );
}
#[test]
fn s3_abi_boundaries() {
    let start = Instant::now();
    std::thread::Builder::new()
        .name("s3-abi-gate".into())
        .stack_size(ULUA_MIN_RUN_STACK)
        .spawn(|| {
            source_and_memory();
            nesting();
            compiler_bounds();
            combined();
            #[cfg(feature = "measurements")]
            native_ccalls();
            runtime();
        })
        .unwrap()
        .join()
        .unwrap();
    eprintln!("S3 added gate wall time: {:?}", start.elapsed());
}
