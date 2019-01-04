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
}

impl<T, M, R> AnyRegex<T, M, R> where
    M: Zero,
    R: Regex<T, M>,
{
    pub fn empty(&self) -> bool { self.re.empty() }
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

impl<T, M, R: Regex<T, M>> Clone for AnyRegex<T, M, R> {
    fn clone(&self) -> Self { self.re.clone_reset() }
}

/// All grammar types/combinators must implement `Regex`.
pub trait Regex<T, M>: Sized {
    fn empty(&self) -> bool;
    fn active(&self) -> bool;
    fn shift(&mut self, c : &T, mark : M) -> M;
    fn reset(&mut self);
    fn clone_reset(&self) -> AnyRegex<T, M, Self>;
}
