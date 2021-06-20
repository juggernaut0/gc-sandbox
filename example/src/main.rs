use gc::*;
use std::mem::size_of;
use std::cell::{Cell, RefCell};

// TODO Option<GcPtr<T>>
// TODO making GcBor mutable

#[derive(GcNew, Trace)]
struct SelfRef {
    x: i32,
    other: GcPtr<Foo>,
}

#[derive(Trace)]
struct Foo {
    foo: Option<GcPtr<SelfRef>>,
}

impl Foo {
    fn gc_new<'ctx, 'gc>(__gc_ctx: &'ctx gc::GcContext<'gc>, other: Option<GcBor<'ctx, 'gc, SelfRef>>) -> GcBor<'ctx, 'gc, Self> {
        __gc_ctx.allocate(Self { foo: other.map(|other| unsafe { GcPtr::from_bor(other) }) })
    }
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
    let a: GcBor<SelfRef> = SelfRef::gc_new(&ctx, 1, Foo::gc_new(&ctx, None));
    let b: GcBor<SelfRef> = SelfRef::gc_new(&ctx, 2, Foo::gc_new(&ctx, Some(a)));
    *a.other = *Foo::gc_new(&ctx, Some(b));
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
