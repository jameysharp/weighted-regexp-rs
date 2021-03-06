//! Building blocks for defining language grammars. You need this module
//! if you want to write down the exact type of a language you've
//! defined, or if you want to create your own combinators over language
//! grammars.
//!
//! Everything in this module is parameterized over:
//!
//! - `T`, the type of individual elements of the input; and
//! - `M`, an arbitrary semiring used for tracking state during parsing.
//!
//! Semiring implementations that are widely useful can be found in the
//! `weights` module, but you can write your own to do all sorts of
//! exotic things.

use num_traits::{Zero, zero, One, one};
use std::marker::PhantomData;

pub struct AnyRegex<T, M, R> {
    re: R,
    active: bool,
    input_type: PhantomData<T>,
    mark_type: PhantomData<M>,
}

impl<T, M, R> AnyRegex<T, M, R>
    where M: Zero + One, R: Regex<T, M>
{
    pub fn over<I>(&mut self, over : I) -> M
        where I: IntoIterator<Item=T>
    {
        let mut iter = over.into_iter();
        let mut result;
        if let Some(c) = iter.next() {
            result = self.shift(&c, one());
        } else {
            return if self.empty() { one() } else { zero() };
        }
        while let Some(c) = iter.next() {
            result = self.shift(&c, zero());
        }
        self.reset();
        return result;
    }
}

impl<T, M, R> AnyRegex<T, M, R> where
    R: Regex<T, M>,
{
    pub fn new(re: R) -> Self
    {
        AnyRegex {
            active: re.active(),
            re: re,
            input_type: PhantomData,
            mark_type: PhantomData,
        }
    }

    pub fn boxed(self) -> Box<Regex<T, M>> where
        R: 'static,
    {
        Box::new(self.re)
    }
}

impl<T, M, R> AnyRegex<T, M, R> where
    M: Zero,
    R: Regex<T, M>,
{
    pub fn empty(&mut self) -> bool { self.re.empty() }
    pub fn active(&self) -> bool { self.active }
    pub fn shift(&mut self, c : &T, mark : M) -> M {
        if !self.active && mark.is_zero() {
            return mark;
        }
        let mark = self.re.shift(c, mark);
        self.active = self.re.active();
        mark
    }
    pub fn reset(&mut self) {
        if self.active {
            self.re.reset();
            self.active = self.re.active();
        }
    }
}

/// Grammar types must implement `Regex`.
pub trait Regex<T, M> {
    fn empty(&mut self) -> bool;
    fn active(&self) -> bool;
    fn shift(&mut self, c : &T, mark : M) -> M;
    fn reset(&mut self);
}

impl<T, M, R: CloneRegex<T, M>> AnyRegex<T, M, R> {
    pub fn clone_reset(&self) -> Self { self.re.clone_reset() }
}

/// Grammar types _should_ implement `CloneRegex`.
pub trait CloneRegex<T, M>: Regex<T, M> + Sized {
    fn clone_reset(&self) -> AnyRegex<T, M, Self>;
}

/// Like std::convert::Into, except that the conversion may optionally
/// use the current item of parse input in addition to `self`.
///
/// Weight types should at least provide a reflexive implementation
/// which ignores the input and performs an identity conversion, so that
/// grammars can explicitly choose which type of weight to produce.
///
/// Weight types should also provide an implementation for converting
/// `bool` to a weight. This may be as simple as ignoring the input and
/// just returning `one()` if `self` is `true`, or `zero()` otherwise.
/// But weight implementations can choose to record the matched input
/// within the weights, and this trait allows generic bool-returning
/// grammars to work with those weights.
pub trait IntoWithInput<T, M> {
    fn into_with_input(self, input: &T) -> M;
}
