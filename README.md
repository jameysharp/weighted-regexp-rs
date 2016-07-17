This is a Rust implementation of the ideas from ["A Play on Regular
Expressions"](http://sebfisch.github.io/haskell-regexp/). Initially I
more-or-less followed the Python treatment of the same ideas from ["An
Efficient and Elegant Regular Expression Matcher in
Python"](https://morepypy.blogspot.com/2010/05/efficient-and-elegant-regular.html),
and then I've added support for weights.

This is not ready for real use: test coverage is minimal, and the
interface for constructing and matching regular expressions is
inconvenient.

However, this does illustrate that Rust can do (almost) everything that
the original paper needed from Haskell, so it should be possible to
duplicate all of the paper's results.
