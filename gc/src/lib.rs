use std::ops::Deref;
use std::marker::PhantomData;
use std::cell::{RefCell, RefMut};

pub use gc_derive::*;
use std::fmt::{Debug, Formatter};
use std::collections::{HashMap, HashSet};
use std::ptr::NonNull;
use crate::unsafe_into::UnsafeInto;

pub mod unsafe_into;

// === GcPtr ===

pub struct GcPtr<T> {
    ptr: NonNull<T>,
}

impl<T> GcPtr<T> {
    /// # Safety
    ///
    /// Returned GcPtr must be immediately moved into another Gc managed object in the same GcContext as Bor
    pub unsafe fn from_bor(bor: GcBor<T>) -> GcPtr<T> {
        GcPtr { ptr: bor.ptr }
    }
}

impl<T: Debug> Debug for GcPtr<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.deref())
    }
}

impl<T> Deref for GcPtr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // safety: self.ptr cannot be constructed by user code and is guaranteed by module to be init and valid
        unsafe { self.ptr.as_ref() }
    }
}

/*impl<T> DerefMut for GcPtr<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(self.ptr as *mut T) }
    }
}*/

impl<T> UnsafeInto<GcPtr<T>> for GcBor<'_, '_, T> {
    unsafe fn unsafe_into(self) -> GcPtr<T> {
        GcPtr::from_bor(self)
    }
}

impl<T> UnsafeInto<Option<GcPtr<T>>> for Option<GcBor<'_, '_, T>> {
    unsafe fn unsafe_into(self) -> Option<GcPtr<T>> {
        self.map(|it| it.unsafe_into())
    }
}

// === GcCell ===

/*pub struct GcCell<T> {
    ptr: RefCell<*const T>,
}

impl<T> GcCell<T> {
    pub unsafe fn from_bor(bor: GcBor<T>) -> GcCell<T> {
        GcCell { ptr: RefCell::new(bor.ptr) }
    }
}

impl<T> Deref for GcCell<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // safety: self.ptr cannot be constructed by user code and is guaranteed by module to be init and valid
        unsafe { &**self.ptr.borrow() }
    }
}

impl<T> DerefMut for GcCell<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *(*self.ptr.borrow_mut() as *mut T) }
    }
}

impl<T> UnsafeInto<GcCell<T>> for GcBor<'_, '_, T> {
    unsafe fn unsafe_into(self) -> GcCell<T> {
        GcCell::from_bor(self)
    }
}

impl<T> UnsafeInto<Option<GcCell<T>>> for Option<GcBor<'_, '_, T>> {
    unsafe fn unsafe_into(self) -> Option<GcCell<T>> {
        self.map(|it| it.unsafe_into())
    }
}*/

// === GcBor ===

pub struct GcBor<'ctx, 'gc, T> {
    ptr: NonNull<T>,
    // ctx: &'ctx GcContext<'gc>,
    phantom: PhantomData<&'ctx GcContext<'gc>>,
}

impl<'ctx, 'gc, T> GcBor<'ctx, 'gc, T> {
    fn new(ptr: NonNull<T>, _ctx: &'ctx GcContext<'gc>) -> GcBor<'ctx, 'gc, T> {
        GcBor { ptr, phantom: PhantomData::default() }
    }

    pub fn as_ptr(self) -> *mut T {
        self.ptr.as_ptr()
    }
}

impl<T: Debug> Debug for GcBor<'_, '_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.deref())
    }
}

impl<'ctx, 'gc, T> Deref for GcBor<'ctx, 'gc, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // safety: self.ptr cannot be constructed by user code and is guaranteed by module to be init and valid
        unsafe { self.ptr.as_ref() }
    }
}

impl<'ctx, 'gc, T> Clone for GcBor<'ctx, 'gc, T> {
    fn clone(&self) -> Self {
        GcBor { ptr: self.ptr, phantom: PhantomData::default() }
    }
}

impl<'ctx, 'gc, T> Copy for GcBor<'ctx, 'gc, T> {}

// === GcRoot ===

pub struct GcRoot<'gc, T: Trace + 'static> {
    ptr: NonNull<T>,
    gc: &'gc Gc,
}

impl<'gc, T: Trace> GcRoot<'gc, T> {
    pub fn borrow<'ctx>(&self, ctx: &'ctx GcContext<'gc>) -> GcBor<'ctx, 'gc, T> {
        // safety: self.ptr cannot be constructed by user code and is guaranteed by module to be init and valid
        GcBor::new(self.ptr, ctx)
    }
}

impl<T: Trace + 'static> Drop for GcRoot<'_, T> {
    fn drop(&mut self) {
        self.gc.unroot(self.ptr)
    }
}

// === Gc & GcContext ===

#[derive(Default)]
pub struct Gc {
    context_ref: RefCell<()>,
    objs: RefCell<HashMap<NonNull<dyn Trace>, bool>>,
    roots: RefCell<HashSet<NonNull<dyn Trace>>>,
}

pub struct GcContext<'gc> {
    _r: RefMut<'gc, ()>,
    gc: &'gc Gc,
}

#[derive(Debug)]
pub struct GcContextError;

impl Gc {
    pub fn new() -> Gc {
        Gc::default()
    }

    pub fn stats(&self) {
        eprintln!("objs: {}", self.objs.borrow().len());
        eprintln!("roots: {}", self.roots.borrow().len());
        eprintln!("size: {}", self.objs.borrow().iter().map(|(ptr, _)| unsafe { std::mem::size_of_val(ptr.as_ref()) }).sum::<usize>());
    }

    pub fn try_context(&self) -> Result<GcContext, GcContextError> {
        match self.context_ref.try_borrow_mut() {
            Ok(r) => Ok(GcContext { _r: r, gc: self }),
            Err(_) => Err(GcContextError)
        }
    }

    pub fn context(&self) -> GcContext {
        self.try_context().expect("Context already exists")
    }

    pub fn root<'gc, T: Trace + 'static>(&'gc self, bor: GcBor<T>) -> GcRoot<'gc, T> {
        let ptr: NonNull<dyn Trace> = bor.ptr;
        self.roots.borrow_mut().insert(ptr);
        GcRoot { ptr: bor.ptr, gc: self } // TODO store for tracing later
    }

    fn unroot<T: Trace + 'static>(&self, ptr: NonNull<T>) {
        let ptr: NonNull<dyn Trace> = ptr;
        self.roots.borrow_mut().remove(&ptr);
    }

    fn allocate<T: Trace + 'static>(&self, ptr: NonNull<T>) -> NonNull<T> {
        self.objs.borrow_mut().insert(ptr, false);
        ptr
    }

    fn collect(&self) {
        let mut objs = self.objs.borrow_mut();
        unsafe {
            for ptr in self.roots.borrow().iter() {
                objs.insert(*ptr, true);
            }
            let mut tracer = Tracer { objs: &mut objs };
            for ptr in self.roots.borrow().iter() {
                ptr.as_ref().trace(&mut tracer)
            }

            objs.retain(|ptr, marked| {
                if *marked {
                    *marked = false;
                    true
                } else {
                    drop(Box::from_raw(ptr.as_ptr()));
                    false
                }
            });
        }
    }
}

impl<'gc> GcContext<'gc> {
    pub fn allocate<'ctx, T: Trace + 'static>(&'ctx self, t: T) -> GcBor<'ctx, 'gc, T> {
        // Safety: Box::into_raw(Box::new(...)) is guaranteed to return an init non-null pointer
        let ptr = unsafe { self.gc.allocate(NonNull::new_unchecked(Box::into_raw(Box::new(t)))) };
        GcBor::new(self.gc.allocate(ptr), self)
    }

    pub fn collect(self) {
        self.gc.collect()
    }
}

// unsafe if manually implemented
pub unsafe trait Trace {
    fn trace(&self, tracer: &mut Tracer);
}

unsafe impl<T: Trace + 'static> Trace for GcPtr<T> {
    fn trace(&self, tracer: &mut Tracer) {
        if tracer.mark(self.ptr) {
            self.deref().trace(tracer);
        }
    }
}

/*unsafe impl<T: Trace + 'static> Trace for GcCell<T> {
    fn trace(&self, tracer: &mut Tracer) {
        if tracer.mark(*self.ptr.borrow()) {
            self.deref().trace(tracer);
        }
    }
}*/

unsafe impl<T: Trace + 'static> Trace for Option<T> {
    fn trace(&self, tracer: &mut Tracer) {
        if let Some(t) = self {
            t.trace(tracer);
        }
    }
}

macro_rules! noop_trace {
    ($t: ty) => {
        unsafe impl Trace for $t {
            fn trace(&self, _tracer: &mut Tracer) {
                // noop
            }
        }
    };
}

noop_trace!(i32);
noop_trace!(i64);

pub struct Tracer<'a> {
    objs: &'a mut HashMap<NonNull<dyn Trace>, bool>
}

impl Tracer<'_> {
    // returns true if ptr has not been seen and tracing should continue
    fn mark<T: Trace + 'static>(&mut self, ptr: NonNull<T>) -> bool {
        let ptr: NonNull<dyn Trace> = ptr;
        !self.objs.insert(ptr, true).unwrap()
    }
}
