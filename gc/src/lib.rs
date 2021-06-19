use std::ops::Deref;
use std::marker::PhantomData;
use std::cell::{RefCell, RefMut};

pub use gc_derive::*;

pub struct GcPtr<T> {
    ptr: *const T,
}

impl<T> GcPtr<T> {
    pub unsafe fn from_bor(bor: GcBor<T>) -> GcPtr<T> {
        GcPtr { ptr: bor.ptr }
    }
}

impl<T> Deref for GcPtr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // safety: self.ptr cannot be constructed by user code and is guaranteed by module to be init and valid
        unsafe { &*self.ptr }
    }
}

pub struct GcBor<'ctx, 'gc, T> {
    ptr: *const T,
    // ctx: &'ctx GcContext<'gc>,
    phantom: PhantomData<&'ctx GcContext<'gc>>,
}

impl<'ctx, 'gc, T> GcBor<'ctx, 'gc, T> {
    fn new(ptr: *const T, _ctx: &'ctx GcContext<'gc>) -> GcBor<'ctx, 'gc, T> {
        GcBor { ptr, phantom: PhantomData::default() }
    }
}

impl<'ctx, 'gc, T> Deref for GcBor<'ctx, 'gc, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // safety: self.ptr cannot be constructed by user code and is guaranteed by module to be init and valid
        unsafe { &*self.ptr }
    }
}

impl<'ctx, 'gc, T> Clone for GcBor<'ctx, 'gc, T> {
    fn clone(&self) -> Self {
        GcBor { ptr: self.ptr, phantom: PhantomData::default() }
    }
}

impl<'ctx, 'gc, T> Copy for GcBor<'ctx, 'gc, T> {}

pub struct GcRoot<'gc, T> {
    ptr: *const T,
    //gc: &'gc Gc,
    phantom: PhantomData<&'gc Gc>,
}

impl<'gc, T> GcRoot<'gc, T> {
    pub fn borrow<'ctx>(&self, ctx: &'ctx GcContext<'gc>) -> GcBor<'ctx, 'gc, T> {
        // safety: self.ptr cannot be constructed by user code and is guaranteed by module to be init and valid
        GcBor::new(unsafe { &*self.ptr }, ctx)
    }
}

pub struct Gc {
    context_ref: RefCell<()>,
}

pub struct GcContext<'gc> {
    r: RefMut<'gc, ()>,
    phantom: PhantomData<&'gc Gc>
}

#[derive(Debug)]
pub struct GcContextError;

impl Gc {
    pub fn new() -> Gc {
        Gc { context_ref: RefCell::new(()) }
    }

    pub fn try_context(&self) -> Result<GcContext, GcContextError> {
        match self.context_ref.try_borrow_mut() {
            Ok(r) => Ok(GcContext { r, phantom: PhantomData::default() }),
            Err(_) => Err(GcContextError)
        }
    }

    pub fn context(&self) -> GcContext {
        self.try_context().expect("Context already exists")
    }

    pub fn root<'gc, T>(&'gc self, bor: GcBor<T>) -> GcRoot<'gc, T> {
        GcRoot { ptr: bor.ptr, phantom: PhantomData::default() } // TODO store for tracing later
    }
}

impl<'gc> GcContext<'gc> {
    pub fn allocate<'ctx, T>(&'ctx self, t: T) -> GcBor<'ctx, 'gc, T> {
        GcBor::new(Box::into_raw(Box::new(t)), self) // TODO store for collection later
    }

    pub fn collect(self) {
        // TODO
    }
}
