use gc::*;
use std::mem::size_of;

#[derive(GcNew)]
struct A {
    b: GcPtr<B>,
}

struct B {
    i: i32
}

fn main() {
    let gc = Gc::new();

    let ctx: GcContext = gc.context();
    let b: GcBor<B> = ctx.allocate(B { i: 42 });
    let a: GcBor<A> = A::gc_new(&ctx, b);
    let a_root: GcRoot<A> = gc.root(a);
    println!("{}", b.i);

    /*let ctx2: GcContext = gc.context();
    let b2: GcBor<B> = ctx2.allocate(B { i: 69 });*/

    ctx.collect();

    //println!("{}", b2.i);

    let new_ctx = gc.context();
    println!("{}", a_root.borrow(&new_ctx).b.i);

    dbg!(size_of::<GcPtr<i64>>());
    dbg!(size_of::<GcBor<i64>>());
    dbg!(size_of::<GcRoot<i64>>());
}
