//! Capture scoped execution Trace from an instrumented closure.
//!
//! ## Usage
//! This program:
//! ```
//! use scoped_tree_trace::Trace;
//!
//! fn main() {
//!     let (_, trace) = Trace::root(|| foo());
//!     println!("{trace}");
//! }
//!
//! fn foo() {
//!     bar();
//!     baz();
//! }
//!
//! fn bar() {
//!     Trace::leaf();
//! }
//!
//! fn baz() {
//!     Trace::leaf();
//! }
//! ```
//! ...prints something like:
//! ```text
//! ╼ inlining::main::{{closure}} at example.rs:4:38
//!   ├╼ inlining::foo at example.rs:9:5
//!   │  └╼ inlining::bar at example.rs:14:5
//!   └╼ inlining::foo at example.rs:10:5
//!      └╼ inlining::baz at example.rs:18:5
//! ```

use backtrace::BacktraceFrame;
use std::{cell::Cell, ffi::c_void, fmt, ptr};

mod symbol;
mod tree;

use symbol::Symbol;
use tree::Tree;

type Backtrace = Vec<BacktraceFrame>;
type SymbolTrace = Vec<Symbol>;

/// An execution trace.
///
/// This trace is constructed by calling [`Trace::root`] with a closure, and
/// includes all execution Trace of that closure that end in an invocation of
/// [`Trace::leaf`].
#[derive(Clone)]
pub struct Trace {
    // The linear backtraces that comprise this trace. These linear traces can
    // be re-knitted into a tree.
    backtraces: Vec<Backtrace>,
}

/// The ambiant backtracing context.
struct Context {
    /// The address of `Trace::trace` establishes an upper unwinding bound on
    /// the backTrace in `Trace`.
    root_addr: *const c_void,
    /// The collection of backTrace collected beneath the invocation of
    /// `Trace::trace`.
    trace: Trace,
}

impl Trace {
    /// Invokes `f`, returning its result and the collection of backTrace
    /// captured at each sub-invocation of `Trace::capture`.
    pub fn root<F, R>(f: F) -> (R, Trace)
    where
        F: FnOnce() -> R,
    {
        // initialize the current backtracing context with an empty `Context`.
        Context::with_current(|cell| Self::root_inner(f, cell))
    }

    // This function is marked `#[inline(never)]` to ensure that it gets a distinct
    // `Frame` in the backtrace, above which we do not need to unwind.
    #[inline(never)]
    fn root_inner<F, R>(f: F, cell: &Cell<Option<Context>>) -> (R, Trace)
    where
        F: FnOnce() -> R,
    {
        cell.set(Some(Context::new::<F, R>()));

        // if `f()` panics, reset the ambiant context to `None`.
        let _deferred = defer(|| {
            cell.set(None);
        });

        let result = f();

        // take the resulting `Trace` and return it.
        let context = cell.take().unwrap();

        (result, context.trace)
    }

    /// If this is a sub-invocation of `Trace::trace`, capture a backtrace.
    ///
    /// The captured backtrace will be returned by `Trace::root`.
    // This function is marked `#[inline(never)]` to ensure that it gets a distinct `Frame` in the
    // backtrace, below which frames should not be included in the backtrace (since they reflect the
    // internal implementation details of this crate).
    #[inline(never)]
    pub fn leaf() {
        Context::with_current(|context_cell| {
            if let Some(mut context) = context_cell.take() {
                let mut frames = vec![];
                let mut above_leaf = false;
                backtrace::trace(|frame| {
                    let below_root = !ptr::eq(frame.symbol_address(), context.root_addr);

                    // only capture frames above `Trace::capture()` and below
                    // `Trace::trace_inner()`.
                    if above_leaf && below_root {
                        frames.push(frame.to_owned().into());
                    }

                    if ptr::eq(frame.symbol_address(), Self::leaf as *const _) {
                        above_leaf = true;
                    }

                    // only continue unwinding if we're below `Trace::trace`
                    below_root
                });
                context.trace.backtraces.push(frames);
                context_cell.set(Some(context));
            }
        });
    }
}

impl fmt::Display for Trace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Tree::from_trace(self.clone()).fmt(f)
    }
}

impl Context {
    /// Constructs a new, empty ambient backtracing context.
    fn new<F, R>() -> Self
    where
        F: FnOnce() -> R,
    {
        // the address of this function is used to establish an upper unwinding bound
        let root_addr = Trace::root_inner::<F, R> as *const c_void;

        Self {
            root_addr,
            trace: Trace { backtraces: vec![] },
        }
    }

    /// Manipulate the current active backtracing context.
    fn with_current<F, R>(f: F) -> R
    where
        F: FnOnce(&Cell<Option<Context>>) -> R,
    {
        thread_local! {
            /// The current ambiant backtracing context, if any.
            static CURRENT_CONTEXT: Cell<Option<Context>> = const { Cell::new(None) };
        }

        CURRENT_CONTEXT.with(f)
    }
}

fn defer<F: FnOnce() -> R, R>(f: F) -> impl Drop {
    struct Defer<F: FnOnce() -> R, R>(Option<F>);

    impl<F: FnOnce() -> R, R> Drop for Defer<F, R> {
        fn drop(&mut self) {
            self.0.take().unwrap()();
        }
    }

    Defer(Some(f))
}
