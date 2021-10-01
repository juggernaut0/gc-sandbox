use gc::{Gc, GcBor, GcContext, GcNew, GcPtr, GcRoot, Trace};
use std::mem::size_of;

#[derive(GcNew, Trace)]
struct SelfRef {
    x: i32,
    other: Option<GcPtr<SelfRef>>,
}

// TODO implement as proc macro
impl SelfRef {
    fn set_x(this: GcBor<Self>, x: impl gc::unsafe_into::UnsafeInto<i32>) {
        unsafe {
            (*this.as_ptr()).x = x.unsafe_into();
        }
    }

    fn set_other(this: GcBor<Self>, other: impl gc::unsafe_into::UnsafeInto<Option<GcPtr<SelfRef>>>) {
        unsafe {
            (*this.as_ptr()).other = other.unsafe_into();
        }
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

    let opt: Option<GcBor<SelfRef>> = None;
    let a: GcBor<SelfRef> = SelfRef::gc_new(&ctx, 1, opt);
    let b: GcBor<SelfRef> = SelfRef::gc_new(&ctx, 2, Some(a));

    SelfRef::set_x(a, 5);
    SelfRef::set_other(a, Some(b));

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
