# ctx

[![Build Status](https://travis-ci.org/rkusa/ctx.svg?branch=master)](https://travis-ci.org/rkusa/ctx)
[![Crates.io](https://img.shields.io/crates/v/ctx.svg)](https://crates.io/crates/ctx)

[Documentation](https://docs.rs/ctx)

Ctx defines the `Context` type, which carries deadlines, cancelation [futures](https://github.com/alexcrichton/futures-rs), and other request-scoped values across API boundaries and between processes.

It is similar to Go's [context](https://blog.golang.org/context) [package](https://golang.org/pkg/context/). The main use case is to have incoming requests to a server create a Context. This Context is propagated in the chain of function calls between the incoming request until the outging response. On its way, the Context can be replaced with a derived Context using `with_cancel`, `with_deadline`, `with_timeout`, or `with_value`.


