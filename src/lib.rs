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

    #[test]
    fn epsilon_empty() {
        let to_match : Option<()> = None;
        assert!(has_match(&mut Epsilon, to_match));
    }

    #[test]
    fn epsilon_nonempty() {
        let to_match = Some(());
        assert!(!has_match(&mut Epsilon, to_match));
    }

    fn in_class(c : &char) -> Match {
        Match(match *c {
            'a' | 'b' | 'c' => true,
            _ => false
        })
    }

    #[test]
    fn class_empty() {
        assert!(!has_match(&mut in_class, "".chars()));
    }

    #[test]
    fn class_nonmatch() {
        assert!(!has_match(&mut in_class, "A".chars()));
    }

    #[test]
    fn class_match() {
        assert!(has_match(&mut in_class, "a".chars()));
    }

    #[test]
    fn class_long() {
        assert!(!has_match(&mut in_class, "ab".chars()));
    }

    #[test]
    fn alternative_empty() {
        assert!(has_match(&mut Alternative::new(Box::new(in_class) as Box<Regex<_, _>>, Epsilon), None));
    }
}
