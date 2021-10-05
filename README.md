# GC Sandbox

## Concepts

### Crates

* **gc_derive**: Provides Derive macros, primarily implementing Trace.
* **gc**: Provides library types.
* **example**: example usage and testing grounds.

### Types

* **Gc**: Allocation arena, implements allocation, tracing, and removal logic.
* **GcContext**: Provides a span in which a Gc collection is guaranteed *not* to occur. Allows access to and allocation 
of Gc managed data.
* **Trace**: All Gc managed data must be of a type that implements Trace.

### Gc managed ptr types

* **GcPtr**: Lifetime-less pointer to Gc managed data that can only exist behind a GcBor or GcRoot.
* **GcBor**: Pointer bound by the lifetimes of a Gc and GcContext. Obtained from a GcContext. Can be dereferenced 
into an immutable reference, or unsafely into a mutable reference (see below).
* **GcRoot**: Rooted Gc managed data.

## Lessons Learned
* With this model (lifetime-less GcPtr behind lifetime-bound GcBor), any mutable reference (either with a Cell or
not) is unsafe, as it would allow use of std::mem::replace or *Cell::replace to move a GcPtr out onto the stack
where it could outlive the GcContext and the Gc itself.

* A Cell could still be used to enforce aliasing rules, for an unknown runtime cost. However, RefCell and Cell 
cannot be in Gc managed data (cannot impl Trace) because they would enable "unsafe safe" mutable references. A 
custom Cell implementation could be used to provide an unsafe borrow_mut method.
     
* "Autorooting" GcPtrs basically requires reference counting, with an unknown runtime cost.
