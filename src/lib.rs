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

pub struct Sequence<M, L, R> {
    left : L,
    right : R,
    marked_left : M,
}

impl<M: Zero, L, R> Sequence<M, L, R> {
    pub fn new(left : L, right : R) -> Self
    {
        Sequence { left : left, right : right, marked_left : zero() }
    }
}

impl<T, M: Zero + Mul + Clone, L, R> Regex<T, M> for Sequence<M, L, R> where L : Regex<T, M> + Sized, R : Regex<T, M> + Sized {
    fn empty(&self) -> bool { self.left.empty() && self.right.empty() }
    fn shift(&mut self, c : &T, mark : M) -> M {
        // Shift the new mark through the left child.
        let marked_left = self.left.shift(c, mark.clone());

        // Save the new mark that came out of the left child and get the
        // previously-saved mark. The previous left mark is the one we
        // feed into the right child.
        let mut old_marked_left = marked_left.clone();
        std::mem::swap(&mut self.marked_left, &mut old_marked_left);

        // If the left child could match the empty string, then in
        // addition to its previous mark, we also feed our new input
        // mark to the right child.
        if self.left.empty() {
            old_marked_left = old_marked_left + mark;
        }
        let marked_right = self.right.shift(c, old_marked_left);

        // Whatever the right child produced is our result, except if
        // the right child could match the empty string, then the left
        // child's result is included in the output too.
        if self.right.empty() {
            marked_left + marked_right
        } else {
            marked_right
        }
    }
    fn reset(&mut self) {
        self.left.reset();
        self.right.reset();
        self.marked_left = zero();
    }
}

pub struct Repetition<M, R> {
    re : R,
    marked : M,
}

impl<M: Zero, R> Repetition<M, R> {
    pub fn new(re : R) -> Self
    {
        Repetition { re : re, marked : zero() }
    }
}

impl<T, M: Zero + Clone, R> Regex<T, M> for Repetition<M, R> where R : Regex<T, M> + Sized {
    fn empty(&self) -> bool { true }
    fn shift(&mut self, c : &T, mark : M) -> M {
        self.marked = self.re.shift(c, mark + self.marked.clone());
        self.marked.clone()
    }
    fn reset(&mut self) {
        self.re.reset();
        self.marked = zero();
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

        fn sequence_epsilon_left_identity(to_match : String) -> bool {
            let mut re = |c: &char| Match(c.is_uppercase());
            has_match(&mut Sequence::new(Epsilon, &re), to_match.chars()) ==
                has_match(&mut re, to_match.chars())
        }

        fn sequence_epsilon_right_identity(to_match : String) -> bool {
            let mut re = |c: &char| Match(c.is_uppercase());
            has_match(&mut Sequence::new(&re, Epsilon), to_match.chars()) ==
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
