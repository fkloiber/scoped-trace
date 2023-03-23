<!-- Do not edit README.md manually. Instead, edit the module comment of `src/lib.rs`. -->

# scoped-trace

Capture scoped backtraces.

Use [`Trace::root`][__link0] to define the upper unwinding bound of an execution trace, and [`Trace::leaf`][__link1] to define its lower bounds (the points at which backtraces are collected). The resulting traces are trees, since a single invocation of [`Trace::root`][__link2] may have multiple sub-invocations of [`Trace::leaf`][__link3].

For example, running this program:


```rust
use scoped_tree_trace::Trace;

fn main() {
    let (_, trace) = Trace::root(|| foo());
    println!("{trace}");
}

fn foo() {
    bar();
    baz();
}

fn bar() {
    Trace::leaf();
}

fn baz() {
    Trace::leaf();
}
```

…will produce an output like:


```text
╼ inlining::main::{{closure}} at example.rs:4:38
  ├╼ inlining::foo at example.rs:9:5
  │  └╼ inlining::bar at example.rs:14:5
  └╼ inlining::foo at example.rs:10:5
     └╼ inlining::baz at example.rs:18:5
```



## License

This project is licensed under the Apache License, Version 2.0, or the MIT
license, at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in scoped-trace by you, shall be licensed as MIT and Apache 2.0,
without any additional terms or conditions.