use core::{Regex, AnyRegex};
use num_traits::{Zero, zero, One, one};
use std::mem::replace;
use std::ops;

pub struct Empty;

impl<T, M: Zero> Regex<T, M> for Empty {
    fn empty(&self) -> bool { true }
    fn shift(&mut self, _c : &T, _mark : M) -> M { zero() }
    fn reset(&mut self) { }
    fn clone_reset(&self) -> AnyRegex<T, M, Self> { empty() }
}

/// Language which only matches an empty string.
pub fn empty<T, M>() -> AnyRegex<T, M, Empty> { AnyRegex::new(Empty) }

impl<T, M, F> Regex<T, M> for F where
    M: ops::Mul<Output=M>,
    F: Fn(&T) -> M + Clone,
{
    fn empty(&self) -> bool { false }
    fn shift(&mut self, c : &T, mark : M) -> M {
        mark * self(c)
    }
    fn reset(&mut self) { }
    fn clone_reset(&self) -> AnyRegex<T, M, Self> { is(self.clone()) }
}

/// Language which only matches inputs containing exactly one item, and
/// passes that item to an arbitrary function you provide.
///
/// This function can return any value within the weights semiring `M`;
/// in simple cases, you probably want to return `zero()` if you want
/// the input to not match, or `one()` if it should match.
pub fn is<T, M: ops::Mul<Output=M>, F>(f: F) -> AnyRegex<T, M, F>
    where F: Fn(&T) -> M
{
    AnyRegex::new(f)
}

pub struct Not<T, M, R>(AnyRegex<T, M, R>);

impl<T, M, R> ops::Not for AnyRegex<T, M, R> {
    type Output = AnyRegex<T, M, Not<T, M, R>>;
    fn not(self) -> Self::Output { AnyRegex::new(Not(self)) }
}

impl<T, M: Zero + One, R> Regex<T, M> for Not<T, M, R> where R : Regex<T, M> {
    fn empty(&self) -> bool { !self.0.empty() }
    fn shift(&mut self, c : &T, mark : M) -> M {
        let new_mark = self.0.shift(c, mark);
        if new_mark.is_zero() { one() } else { zero() }
    }
    fn reset(&mut self) {
        self.0.reset();
    }
    fn clone_reset(&self) -> AnyRegex<T, M, Self> { !self.0.clone() }
}

pub struct Or<T, M, L, R> {
    left : AnyRegex<T, M, L>,
    right : AnyRegex<T, M, R>,
}

impl<T, M, L, R> ops::BitOr<AnyRegex<T, M, R>> for AnyRegex<T, M, L>
{
    type Output = AnyRegex<T, M, Or<T, M, L, R>>;
    fn bitor(self, other: AnyRegex<T, M, R>) -> Self::Output
    {
        AnyRegex::new(Or { left: self, right: other })
    }
}

impl<T, M: ops::Add<Output=M> + Clone, L, R> Regex<T, M> for Or<T, M, L, R> where L : Regex<T, M>, R : Regex<T, M> {
    fn empty(&self) -> bool { self.left.empty() || self.right.empty() }
    fn shift(&mut self, c : &T, mark : M) -> M {
        self.left.shift(c, mark.clone()) + self.right.shift(c, mark)
    }
    fn reset(&mut self) {
        self.left.reset();
        self.right.reset();
    }
    fn clone_reset(&self) -> AnyRegex<T, M, Self> {
        self.left.clone() | self.right.clone()
    }
}

pub struct And<T, M, L, R> {
    left : AnyRegex<T, M, L>,
    right : AnyRegex<T, M, R>,
}

impl<T, M, L, R> ops::BitAnd<AnyRegex<T, M, R>> for AnyRegex<T, M, L>
{
    type Output = AnyRegex<T, M, And<T, M, L, R>>;
    fn bitand(self, other: AnyRegex<T, M, R>) -> Self::Output
    {
        AnyRegex::new(And { left: self, right: other })
    }
}

impl<T, M: ops::Mul<Output=M> + Clone, L, R> Regex<T, M> for And<T, M, L, R> where L : Regex<T, M>, R : Regex<T, M> {
    fn empty(&self) -> bool { self.left.empty() && self.right.empty() }
    fn shift(&mut self, c : &T, mark : M) -> M {
        self.left.shift(c, mark.clone()) * self.right.shift(c, mark)
    }
    fn reset(&mut self) {
        self.left.reset();
        self.right.reset();
    }
    fn clone_reset(&self) -> AnyRegex<T, M, Self> {
        self.left.clone() & self.right.clone()
    }
}

pub struct Sequence<T, M, L, R> {
    left : AnyRegex<T, M, L>,
    right : AnyRegex<T, M, R>,
    from_left : M,
}

impl<T, M: Zero, L, R> ops::Add<AnyRegex<T, M, R>> for AnyRegex<T, M, L>
{
    type Output = AnyRegex<T, M, Sequence<T, M, L, R>>;
    fn add(self, other: AnyRegex<T, M, R>) -> Self::Output
    {
        AnyRegex::new(Sequence { left: self, right: other, from_left: zero() })
    }
}

impl<T, M: Zero + Clone, L, R> Regex<T, M> for Sequence<T, M, L, R> where L : Regex<T, M>, R : Regex<T, M> {
    fn empty(&self) -> bool { self.left.empty() && self.right.empty() }
    fn shift(&mut self, c : &T, mark : M) -> M {
        // If any parameter or intermediate value is unused, then we've
        // done something wrong.
        //
        // From the self parameter, we specifically need to use all of
        // these values, exactly once each:
        // - self.from_left
        // - self.left.empty
        // - self.left.shift
        // - self.right.empty
        // - self.right.shift
        // In order to use self.from_left, we also need a new value to
        // replace the old one with.
        #![forbid(unused)]

        // These wrapper types let the type-checker verify that every
        // mark which contributes to the return value is the result of
        // exactly one call to shift(c). We need to use the current
        // input or this isn't a correct shift, but we can't use the
        // same input twice in the history of a mark.

        // Marks from parameters must be wrapped with Unshifted().
        // All mark arguments to shift must be unwrapped by unshifted().
        #[derive(Clone)]
        #[must_use]
        struct Unshifted<M>(M);
        #[must_use]
        fn unshifted<M>(m: Unshifted<M>) -> M { m.0 }

        // The result of shift must be wrapped with Shifted().
        // All marks in the return value must be unwrapped by shifted().
        #[derive(Clone)]
        #[must_use]
        struct Shifted<M>(M);
        #[must_use]
        fn shifted<M>(m: Shifted<M>) -> M { m.0 }

        // Given the above rules, there are very few ways this function
        // could possibly be written. For performance reasons, we
        // further constrain it to call clone() as infrequently as
        // possible.

        let from_input = Unshifted(mark);

        let skip_empty_left =
            if self.left.empty() { from_input.clone() } else { Unshifted(zero()) };

        let from_left = Shifted(self.left.shift(c, unshifted(from_input)));

        let skip_empty_right =
            if self.right.empty() { from_left.clone() } else { Shifted(zero()) };

        // By the shift-only-once rule, we can't shift from_left through
        // the right child. Instead, save it for the next round and use
        // the value that the left child produced during the previous
        // round.
        let old_from_left = Unshifted(replace(&mut self.from_left, shifted(from_left)));
        // The old mark was shifted with a previous value of c, but it
        // has not yet been shifted with the current value of c.

        let from_right = Shifted(self.right.shift(c, unshifted(skip_empty_left) + unshifted(old_from_left)));

        shifted(skip_empty_right) + shifted(from_right)
    }
    fn reset(&mut self) {
        self.left.reset();
        self.right.reset();
        self.from_left = zero();
    }
    fn clone_reset(&self) -> AnyRegex<T, M, Self> {
        self.left.clone() + self.right.clone()
    }
}

pub struct Many<T, M, R> {
    re : AnyRegex<T, M, R>,
    marked : M,
}

/// Language which matches zero or more copies of another language. In
/// regular expressions, this is usually called "Kleene star" or just
/// "star", and written `*`.
pub fn many<T, M: Zero, R>(re: AnyRegex<T, M, R>) -> AnyRegex<T, M, Many<T, M, R>>
{
    AnyRegex::new(Many { re: re, marked: zero() })
}

impl<T, M: Zero + Clone, R> Regex<T, M> for Many<T, M, R> where R : Regex<T, M> {
    fn empty(&self) -> bool { true }
    fn shift(&mut self, c : &T, mark : M) -> M {
        let was_marked = replace(&mut self.marked, zero());
        self.marked = self.re.shift(c, mark + was_marked);
        self.marked.clone()
    }
    fn reset(&mut self) {
        self.re.reset();
        self.marked = zero();
    }
    fn clone_reset(&self) -> AnyRegex<T, M, Self> {
        many(self.re.clone())
    }
}