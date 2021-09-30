use gc::*;
use std::mem::size_of;
use std::cell::{Cell, RefCell};

#[derive(GcNew, Trace)]
struct SelfRef {
    x: i32,
    other: Option<GcPtr<SelfRef>>,
}

fn main() {
    dbg!(size_of::<SelfRef>());
    dbg!(size_of::<GcPtr<SelfRef>>());
    dbg!(size_of::<GcCell<SelfRef>>());
    dbg!(size_of::<GcBor<SelfRef>>());
    dbg!(size_of::<GcRoot<SelfRef>>());

    let gc = Gc::new();
    gc.stats();

    let ctx: GcContext = gc.context();

    let a: GcBor<SelfRef> = SelfRef::gc_new(&ctx, 1, None);
    let b: GcBor<SelfRef> = SelfRef::gc_new(&ctx, 2, Some(a));

    // TODO allow setting a.foo to Some(b)

    let a_root = gc.root(a);

    gc.stats();
    eprintln!("ctx.collect()");
    ctx.collect();
    gc.stats();

    let new_ctx = gc.context();
    let a = a_root.borrow(&new_ctx);
    dbg!(a.x);
    //dbg!(a.other.borrow().unwrap().x);

    drop(a_root);

    eprintln!("new_ctx.collect()");
    new_ctx.collect();
    gc.stats();
}
