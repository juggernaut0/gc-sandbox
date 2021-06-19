use gc::*;
use std::mem::size_of;

#[derive(GcNew, Debug)]
struct A {
    b: GcPtr<B>,
}

unsafe impl Trace for A {
    fn trace(&self, tracer: &mut Tracer) {
        self.b.trace(tracer);
    }
}

#[derive(Debug)]
struct B {
    i: i32
}

unsafe impl Trace for B {
    fn trace(&self, tracer: &mut Tracer) {
        self.i.trace(tracer);
    }
}

fn main() {
    dbg!(size_of::<A>());
    dbg!(size_of::<B>());
    dbg!(size_of::<GcPtr<A>>());
    dbg!(size_of::<GcBor<A>>());
    dbg!(size_of::<GcRoot<A>>());

    let gc = Gc::new();
    gc.stats();

    let ctx: GcContext = gc.context();
    let b: GcBor<B> = ctx.allocate(B { i: 42 });
    let b2: GcBor<B> = ctx.allocate(B { i: 69 });
    let a: GcBor<A> = A::gc_new(&ctx, b);
    let a_root: GcRoot<A> = gc.root(a);
    dbg!(b);
    dbg!(b2);

    gc.stats();
    ctx.collect();
    gc.stats();

    // compile error
    // dbg!(b2);

    let new_ctx = gc.context();
    dbg!(a_root.borrow(&new_ctx));
}
