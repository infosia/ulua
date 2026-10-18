//! P6.S8 pins export order for synchronous calls and tasks.
use std::ptr;
use ulua::*;

#[test]
fn every_export_uses_its_source_index_for_calls_and_tasks() {
    let source = b"export function zebra(n:number):number{return n+7;}export function alpha(b:boolean):boolean{return !b;}export function middle(s:string,n:number):string{return `${s}:${n}`;}export async function later(n:number):Promise<number>{return n*3;}export async function done():Promise<void>{}";
    let mut context = ptr::null_mut();
    let mut module = ptr::null_mut();
    let mut instance = ptr::null_mut();
    let mut diagnostic = Diagnostic::default();
    // SAFETY: All inputs and outputs remain live. The context owns the module and instance.
    unsafe {
        assert_eq!(ulua_context_new(&mut context), ULUA_OK);
        assert_eq!(
            ulua_compile(
                context,
                source.as_ptr().cast(),
                source.len(),
                ptr::null(),
                0,
                ptr::null(),
                &mut module,
                &mut diagnostic
            ),
            ULUA_OK
        );
        assert_eq!(
            ulua_instance_new(
                context,
                module,
                &Host::default(),
                ptr::null(),
                &mut instance,
                &mut diagnostic
            ),
            ULUA_OK
        );
        let mut number = Value::default();
        number.r#type = ULUA_TYPE_NUMBER;
        number.number = 4.0;
        let mut boolean = Value::default();
        boolean.r#type = ULUA_TYPE_BOOLEAN;
        boolean.boolean = 1;
        let mut string = Value::default();
        string.r#type = ULUA_TYPE_STRING;
        string.data = c"word".as_ptr();
        string.length = 4;
        let cases = [
            ("zebra", vec![number], ULUA_TYPE_NUMBER, 11.0),
            ("alpha", vec![boolean], ULUA_TYPE_BOOLEAN, 0.0),
            ("middle", vec![string, number], ULUA_TYPE_STRING, 0.0),
            ("later", vec![number], ULUA_TYPE_NUMBER, 12.0),
            ("done", vec![], ULUA_TYPE_VOID, 0.0),
        ];
        // The instance must keep its exports after the module is freed.
        for (index, (name, _, _, _)) in cases.iter().enumerate() {
            let mut found = u32::MAX;
            assert_eq!(
                ulua_module_find_export(module, name.as_ptr().cast(), name.len(), &mut found),
                ULUA_OK
            );
            assert_eq!(found, index as u32);
        }
        assert_eq!(ulua_module_free(module), ULUA_OK);
        for _ in 0..2 {
            for (index, (_, args, tag, expected)) in cases.iter().enumerate() {
                for start in [false, true] {
                    let mut result = Value::default();
                    if start {
                        let mut task = 0;
                        assert_eq!(
                            ulua_call_start(
                                instance,
                                index as u32,
                                args.as_ptr(),
                                args.len(),
                                &mut task,
                                &mut diagnostic
                            ),
                            ULUA_OK
                        );
                        assert_eq!(
                            ulua_task_result(instance, task, &mut result, &mut diagnostic),
                            ULUA_OK
                        );
                    } else {
                        assert_eq!(
                            ulua_call(
                                instance,
                                index as u32,
                                args.as_ptr(),
                                args.len(),
                                &mut result,
                                &mut diagnostic
                            ),
                            ULUA_OK
                        );
                    }
                    assert_eq!(result.r#type, *tag);
                    if *tag == ULUA_TYPE_NUMBER {
                        assert_eq!(result.number, *expected);
                    }
                    if *tag == ULUA_TYPE_BOOLEAN {
                        assert_eq!(result.boolean, 0);
                    }
                    if *tag == ULUA_TYPE_STRING {
                        assert_eq!(
                            std::slice::from_raw_parts(result.data.cast::<u8>(), result.length),
                            b"word:4"
                        );
                    }
                }
            }
        }
        let mut result = Value::default();
        let mut task = 0;
        assert_eq!(
            ulua_call(instance, 5, ptr::null(), 0, &mut result, &mut diagnostic),
            ULUA_INVALID_ARGUMENT
        );
        assert_eq!(
            ulua_call_start(instance, 5, ptr::null(), 0, &mut task, &mut diagnostic),
            ULUA_INVALID_ARGUMENT
        );
        assert_eq!(ulua_context_free(context), ULUA_OK);
    }
}
