use std::collections::HashSet;
use std::hash::Hash;

pub trait Regex<T> {
    fn empty(&self) -> bool;
    fn shift(&mut self, c : &T, mark : bool) -> bool;
    fn reset(&mut self);
}

impl<T,U> Regex<T> for Box<U> where U : Regex<T> {
    fn empty(&self) -> bool { self.as_ref().empty() }
    fn shift(&mut self, c : &T, mark : bool) -> bool { self.as_mut().shift(c, mark) }
    fn reset(&mut self) { self.as_mut().reset() }
}

pub struct Epsilon;

impl<T> Regex<T> for Epsilon {
    fn empty(&self) -> bool { true }
    fn shift(&mut self, _c : &T, _mark : bool) -> bool { false }
    fn reset(&mut self) { }
}

pub struct Class<T: Eq + Hash> {
    accept : HashSet<T>,
}

impl<T: Eq + Hash> Class<T> {
    pub fn new<I>(accept : I) -> Self
        where I: IntoIterator<Item=T>
    {
        Class { accept: accept.into_iter().collect() }
    }
}

impl<T: Eq + Hash> Regex<T> for Class<T> {
    fn empty(&self) -> bool { false }
    fn shift(&mut self, c : &T, mark : bool) -> bool {
        mark && self.accept.contains(c)
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

impl<T, L, R> Regex<T> for Alternative<L, R> where L : Regex<T> + Sized, R : Regex<T> + Sized {
    fn empty(&self) -> bool { self.left.empty() || self.right.empty() }
    fn shift(&mut self, c : &T, mark : bool) -> bool {
        self.left.shift(c, mark) || self.right.shift(c, mark)
    }
    fn reset(&mut self) {
        self.left.reset();
        self.right.reset();
    }
}

pub struct Sequence<L, R> {
    left : L,
    right : R,
    marked_left : bool,
}

impl<T, L, R> Regex<T> for Sequence<L, R> where L : Regex<T> + Sized, R : Regex<T> + Sized {
    fn empty(&self) -> bool { self.left.empty() && self.right.empty() }
    fn shift(&mut self, c : &T, mark : bool) -> bool {
        let marked_left = self.left.shift(c, mark);
        let marked_right = self.right.shift(c, self.marked_left || (mark && self.left.empty()));
        self.marked_left = marked_left;
        (marked_left && self.right.empty()) || marked_right
    }
    fn reset(&mut self) {
        self.left.reset();
        self.right.reset();
        self.marked_left = false;
    }
}

pub struct Repetition<R> {
    re : R,
    marked : bool,
}

impl<T, R> Regex<T> for Repetition<R> where R : Regex<T> + Sized {
    fn empty(&self) -> bool { true }
    fn shift(&mut self, c : &T, mark : bool) -> bool {
        self.marked = self.re.shift(c, mark || self.marked);
        self.marked
    }
    fn reset(&mut self) {
        self.re.reset();
        self.marked = false;
    }
}

pub fn match_regex<T, I>(re : &mut Regex<T>, over : I) -> bool
    where I: IntoIterator<Item=T>
{
    let mut iter = over.into_iter();
    let mut result;
    if let Some(c) = iter.next() {
        result = re.shift(&c, true);
    } else {
        return re.empty();
    }
    while let Some(c) = iter.next() {
        result = re.shift(&c, false);
    }
    re.reset();
    return result;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epsilon_empty() {
        let to_match : Option<()> = None;
        assert!(match_regex(&mut Epsilon, to_match));
    }

    #[test]
    fn epsilon_nonempty() {
        let to_match = Some(());
        assert!(!match_regex(&mut Epsilon, to_match));
    }

    fn make_class() -> Class<char> {
        Class::new("abc".chars())
    }

    #[test]
    fn class_empty() {
        assert!(!match_regex(&mut make_class(), "".chars()));
    }

    #[test]
    fn class_nonmatch() {
        assert!(!match_regex(&mut make_class(), "A".chars()));
    }

    #[test]
    fn class_match() {
        assert!(match_regex(&mut make_class(), "a".chars()));
    }

    #[test]
    fn class_long() {
        assert!(!match_regex(&mut make_class(), "ab".chars()));
    }

    #[test]
    fn alternative_empty() {
        assert!(match_regex(&mut Alternative::new(Box::new(make_class()), Epsilon), None));
    }
}
