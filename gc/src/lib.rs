use std::ops::Deref;
use std::marker::PhantomData;
use std::cell::{RefCell, RefMut};

pub use gc_derive::*;
use std::fmt::{Debug, Formatter};
use std::collections::{HashMap, HashSet};

pub struct GcPtr<T> {
    ptr: *const T,
}

impl<T> GcPtr<T> {
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

impl<T: Debug> Debug for GcBor<'_, '_, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.deref())
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

pub struct GcRoot<'gc, T: Trace + 'static> {
    ptr: *const T,
    gc: &'gc Gc,
}

impl<'gc, T: Trace> GcRoot<'gc, T> {
    pub fn borrow<'ctx>(&self, ctx: &'ctx GcContext<'gc>) -> GcBor<'ctx, 'gc, T> {
        // safety: self.ptr cannot be constructed by user code and is guaranteed by module to be init and valid
        GcBor::new(unsafe { &*self.ptr }, ctx)
    }
}

impl<T: Trace + 'static> Drop for GcRoot<'_, T> {
    fn drop(&mut self) {
        self.gc.unroot(self.ptr)
    }
}

pub struct Gc {
    context_ref: RefCell<()>,
    objs: RefCell<HashMap<*mut dyn Trace, bool>>,
    roots: RefCell<HashSet<*mut dyn Trace>>,
}

pub struct GcContext<'gc> {
    _r: RefMut<'gc, ()>,
    gc: &'gc Gc,
}

#[derive(Debug)]
pub struct GcContextError;

impl Gc {
    pub fn new() -> Gc {
        Gc {
            context_ref: RefCell::default(),
            objs: RefCell::default(),
            roots: RefCell::default(),
        }
    }

    pub fn stats(&self) {
        eprintln!("objs: {}", self.objs.borrow().len());
        eprintln!("roots: {}", self.roots.borrow().len());
        eprintln!("size: {}", self.objs.borrow().iter().map(|(ptr, _)| unsafe { std::mem::size_of_val(&**ptr) }).sum::<usize>());
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
        let ptr: *mut dyn Trace = bor.ptr as *mut T;
        self.roots.borrow_mut().insert(ptr);
        GcRoot { ptr: bor.ptr, gc: self } // TODO store for tracing later
    }

    fn unroot<T: Trace + 'static>(&self, ptr: *const T) {
        let ptr: *mut dyn Trace = ptr as *mut T;
        self.roots.borrow_mut().remove(&ptr);
    }

    fn allocate<T: Trace + 'static>(&self, ptr: *mut T) -> *const T {
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
                (&mut **ptr).trace(&mut tracer)
            }

            objs.retain(|ptr, marked| {
                if *marked {
                    *marked = false;
                    true
                } else {
                    drop(Box::from_raw(*ptr));
                    false
                }
            });
        }
    }
}

impl<'gc> GcContext<'gc> {
    pub fn allocate<'ctx, T: Trace + 'static>(&'ctx self, t: T) -> GcBor<'ctx, 'gc, T> {
        GcBor::new(self.gc.allocate(Box::into_raw(Box::new(t))), self)
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
        tracer.mark(self.ptr);
        self.deref().trace(tracer);
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
    objs: &'a mut HashMap<*mut dyn Trace, bool>
}

impl Tracer<'_> {
    fn mark<T: Trace + 'static>(&mut self, ptr: *const T) {
        let ptr: *mut dyn Trace = ptr as *mut T;
        if self.objs.contains_key(&ptr) {
            self.objs.insert(ptr, true);
        }
    }
}
