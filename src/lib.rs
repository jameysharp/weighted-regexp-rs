#[cfg(test)]
#[macro_use]
extern crate quickcheck;

extern crate num_traits;

use num_traits::{Zero, zero, One, one};
use std::ops::{Add, Mul};

pub trait Regex<T, M> {
    fn empty(&self) -> bool;
    fn shift(&mut self, c : &T, mark : M) -> M;
    fn reset(&mut self);
}

impl<T, M> Regex<T, M> for Box<Regex<T, M>> {
    fn empty(&self) -> bool { self.as_ref().empty() }
    fn shift(&mut self, c : &T, mark : M) -> M { self.as_mut().shift(c, mark) }
    fn reset(&mut self) { self.as_mut().reset() }
}

#[derive(Copy, Clone)]
pub struct Epsilon;

impl<T, M: Zero> Regex<T, M> for Epsilon {
    fn empty(&self) -> bool { true }
    fn shift(&mut self, _c : &T, _mark : M) -> M { zero() }
    fn reset(&mut self) { }
}

impl<T, M: Mul<Output=M>, F: Fn(&T) -> M> Regex<T, M> for F {
    fn empty(&self) -> bool { false }
    fn shift(&mut self, c : &T, mark : M) -> M {
        mark * self(c)
    }
    fn reset(&mut self) { }
}

#[derive(Copy, Clone)]
pub struct Alternative<L, R> {
    left : L,
    right : R,
}

impl<L, R> Alternative<L, R> {
    pub fn new(left : L, right : R) -> Self
    {
        Alternative { left : left, right : right }
    }
}

impl<T, M: Add<Output=M> + Clone, L, R> Regex<T, M> for Alternative<L, R> where L : Regex<T, M> + Sized, R : Regex<T, M> + Sized {
    fn empty(&self) -> bool { self.left.empty() || self.right.empty() }
    fn shift(&mut self, c : &T, mark : M) -> M {
        self.left.shift(c, mark.clone()) + self.right.shift(c, mark)
    }
    fn reset(&mut self) {
        self.left.reset();
        self.right.reset();
    }
}

#[derive(Copy, Clone)]
pub struct And<L, R> {
    left : L,
    right : R,
}

impl<L, R> And<L, R> {
    pub fn new(left : L, right : R) -> Self
    {
        And { left : left, right : right }
    }
}

impl<T, M: Mul<Output=M> + Clone, L, R> Regex<T, M> for And<L, R> where L : Regex<T, M> + Sized, R : Regex<T, M> + Sized {
    fn empty(&self) -> bool { self.left.empty() && self.right.empty() }
    fn shift(&mut self, c : &T, mark : M) -> M {
        self.left.shift(c, mark.clone()) * self.right.shift(c, mark)
    }
    fn reset(&mut self) {
        self.left.reset();
        self.right.reset();
    }
}

pub struct Sequence<M, L, R> {
    left : L,
    right : R,
    from_left : M,
}

impl<M: Zero, L, R> Sequence<M, L, R> {
    pub fn new(left : L, right : R) -> Self
    {
        Sequence { left : left, right : right, from_left : zero() }
    }
}

impl<M: Zero, L: Clone, R: Clone> Clone for Sequence<M, L, R> {
    fn clone(&self) -> Self
    {
        Sequence::new(self.left.clone(), self.right.clone())
    }
}

impl<T, M: Zero + Clone, L, R> Regex<T, M> for Sequence<M, L, R> where L : Regex<T, M> + Sized, R : Regex<T, M> + Sized {
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
        let old_from_left = Unshifted(std::mem::replace(&mut self.from_left, shifted(from_left)));
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
}

pub struct Repetition<M, R> {
    re : R,
    marked : Option<M>,
}

impl<M, R> Repetition<M, R> {
    pub fn new(re : R) -> Self
    {
        Repetition { re : re, marked : None }
    }
}

impl<M, R: Clone> Clone for Repetition<M, R> {
    fn clone(&self) -> Self
    {
        Repetition::new(self.re.clone())
    }
}

impl<T, M: Zero + Clone, R> Regex<T, M> for Repetition<M, R> where R : Regex<T, M> + Sized {
    fn empty(&self) -> bool { true }
    fn shift(&mut self, c : &T, mark : M) -> M {
        let was_marked = self.marked.take().unwrap_or_else(zero);
        let new_mark = self.re.shift(c, mark + was_marked);
        self.marked = Some(new_mark.clone());
        new_mark
    }
    fn reset(&mut self) {
        self.re.reset();
        self.marked = None;
    }
}

pub fn match_regex<T, M, I>(re : &mut Regex<T, M>, over : I) -> M
    where I: IntoIterator<Item=T>, M: Zero + One
{
    let mut iter = over.into_iter();
    let mut result;
    if let Some(c) = iter.next() {
        result = re.shift(&c, one());
    } else {
        return if re.empty() { one() } else { zero() };
    }
    while let Some(c) = iter.next() {
        result = re.shift(&c, zero());
    }
    re.reset();
    return result;
}

#[derive(Copy, Clone)]
pub struct Match(bool);

impl Add for Match {
    type Output = Match;
    fn add(self, rhs : Match) -> Match { Match(self.0 || rhs.0) }
}

impl Zero for Match {
    fn zero() -> Match { Match(false) }
    fn is_zero(&self) -> bool { !self.0 }
}

impl Mul for Match {
    type Output = Match;
    fn mul(self, rhs : Match) -> Match { Match(self.0 && rhs.0) }
}

impl One for Match {
    fn one() -> Match { Match(true) }
}

pub fn has_match<T, I>(re : &mut Regex<T, Match>, over : I) -> bool
    where I: IntoIterator<Item=T>
{
    match_regex(re, over).0
}

#[cfg(test)]
mod tests {
    use super::*;

    quickcheck! {
        fn epsilon_bool(to_match : Vec<bool>) -> bool {
            to_match.is_empty() == has_match(&mut Epsilon, to_match)
        }

        fn epsilon_char(to_match : String) -> bool {
            to_match.is_empty() == has_match(&mut Epsilon, to_match.chars())
        }

        fn fn_bool(to_match : Vec<bool>) -> bool {
            ({
                let mut iter = to_match.iter();
                match (iter.next(), iter.next()) {
                    (Some(&expected), None) => expected,
                    _ => false,
                }
            }) == has_match(&mut |c: &bool| Match(*c), to_match)
        }

        fn fn_char(to_match : String) -> bool {
            ({
                let mut iter = to_match.chars();
                match (iter.next(), iter.next()) {
                    (Some(expected), None) => expected.is_uppercase(),
                    _ => false,
                }
            }) == has_match(&mut |c: &char| Match(c.is_uppercase()), to_match.chars())
        }

        fn fn_any_bool(to_match : Vec<bool>) -> bool {
            let mut re = |_: &bool| Match(true);
            (to_match.len() == 1) == has_match(&mut re, to_match)
        }

        fn fn_any_char(to_match : String) -> bool {
            let mut re = |_: &char| Match(true);
            (to_match.chars().count() == 1) == has_match(&mut re, to_match.chars())
        }

        fn fn_none_bool(to_match : Vec<bool>) -> bool {
            let mut re = |_: &bool| Match(false);
            !has_match(&mut re, to_match)
        }

        fn fn_none_char(to_match : String) -> bool {
            let mut re = |_: &char| Match(false);
            !has_match(&mut re, to_match.chars())
        }

        fn alternative(to_match : String) -> bool {
            let a = |c: &char| Match(*c == 'a');
            let b = |c: &char| Match(*c == 'b');
            ({
                let mut iter = to_match.chars();
                match (iter.next(), iter.next()) {
                    (Some(expected), None) => expected == 'a' || expected == 'b',
                    _ => false,
                }
            }) == has_match(&mut Alternative::new(a, b), to_match.chars())
        }

        fn alternative_any_epsilon(to_match : String) -> bool {
            let re = |_: &char| Match(true);
            (to_match.chars().count() <= 1) ==
                has_match(&mut Alternative::new(re, Epsilon), to_match.chars())
        }

        fn alternative_epsilon_any(to_match : String) -> bool {
            let re = |_: &char| Match(true);
            (to_match.chars().count() <= 1) ==
                has_match(&mut Alternative::new(Epsilon, re), to_match.chars())
        }

        fn and(to_match : Vec<u8>) -> bool {
            let hexes = Repetition::new(|c: &u8| Match(*c > 10));
            let uppers = Repetition::new(|c: &u8| Match(c % 2 == 0));
            to_match.iter().all(|c| *c > 10 && c % 2 == 0) ==
                has_match(&mut And::new(hexes, uppers), to_match)
        }

        fn and_impossible(to_match : String) -> bool {
            let something = |_: &char| Match(true);
            let nothing = Epsilon;
            !has_match(&mut And::new(something, nothing), to_match.chars())
        }

        fn sequence_epsilon_left_identity(to_match : String) -> bool {
            let mut re = |c: &char| Match(c.is_uppercase());
            has_match(&mut Sequence::new(Epsilon, re), to_match.chars()) ==
                has_match(&mut re, to_match.chars())
        }

        fn sequence_epsilon_right_identity(to_match : String) -> bool {
            let mut re = |c: &char| Match(c.is_uppercase());
            has_match(&mut Sequence::new(re, Epsilon), to_match.chars()) ==
                has_match(&mut re, to_match.chars())
        }

        fn sequence_repeat_epsilon_right_identity(to_match : String) -> bool {
            let mut re = Repetition::new(|c: &char| Match(c.is_uppercase()));
            has_match(&mut Sequence::new(re.clone(), Epsilon), to_match.chars()) ==
                has_match(&mut re, to_match.chars())
        }

        fn repeat_epsilon(to_match : String) -> bool {
            to_match.is_empty() ==
                has_match(&mut Repetition::new(Epsilon), to_match.chars())
        }

        fn repeat_any(to_match : String) -> bool {
            let re = |_: &char| Match(true);
            has_match(&mut Repetition::new(re), to_match.chars())
        }

        fn repeat_char(to_match : String) -> bool {
            let re = |c: &char| Match(*c == 'A');
            to_match.chars().all(|c| c == 'A') ==
                has_match(&mut Repetition::new(re), to_match.chars())
        }

        fn repeat_repeat_char(to_match : String) -> bool {
            let re = |c: &char| Match(*c == 'A');
            to_match.chars().all(|c| c == 'A') ==
                has_match(&mut Repetition::new(Repetition::new(re)), to_match.chars())
        }
    }
}
