use gc::{Gc, GcBor, GcContext, GcPtr, GcRoot, Trace};
use std::mem::size_of;

#[derive(Trace)]
struct SelfRef {
    x: i32,
    other: Option<GcPtr<SelfRef>>,
}

impl SelfRef {
    fn new<'ctx, 'gc>(ctx: &'ctx GcContext<'gc>, x: i32, other: Option<GcBor<SelfRef>>) -> GcBor<'ctx, 'gc, SelfRef> {
        //allocate!(ctx, SelfRef { x: x, other: other }, other)
        ctx.allocate(unsafe {
            use gc::unsafe_into::UnsafeInto;
            SelfRef { x: x, other: other.unsafe_into() }
        })
    }
}

fn main() {
    dbg!(size_of::<SelfRef>());
    dbg!(size_of::<GcPtr<SelfRef>>());
    dbg!(size_of::<Option<GcPtr<SelfRef>>>());
    dbg!(size_of::<GcBor<SelfRef>>());
    dbg!(size_of::<Option<GcBor<SelfRef>>>());
    dbg!(size_of::<GcRoot<SelfRef>>());

    let gc = Gc::new();
    gc.stats();

    let ctx: GcContext = gc.context();

    let a = SelfRef::new(&ctx, 1, None);
    let b = SelfRef::new(&ctx, 2, Some(a));

    unsafe {
        let mut_a = a.as_mut(); // safety: there are no other references to the data in a
        mut_a.x = 5;
        mut_a.other = Some(GcPtr::from_bor(b)); // safety: The GcPtr will be solely owned by Gc managed data
    }

    let a_root = gc.root(a);

    gc.stats();
    eprintln!("ctx.collect()");
    ctx.collect();
    gc.stats();

    let new_ctx = gc.context();
    let a = a_root.borrow(&new_ctx);
    assert_eq!(a.x, 5);
    assert_eq!(a.other.as_ref().unwrap().x, 2);

    drop(a_root);

    eprintln!("new_ctx.collect()");
    new_ctx.collect();
    gc.stats();
}
