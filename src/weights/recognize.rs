//! Check whether an input sequence matches a specified grammar. This
//! semiring produces only a `true` or `false` result, without capturing
//! any information from the input.

use num_traits::{Zero, One};
use std::ops::{Add, Mul};
use ::core::{Regex, AnyRegex};

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

pub fn has_match<T, R, I>(re : &mut AnyRegex<T, Match, R>, over : I) -> bool
    where R: Regex<T, Match>, I: IntoIterator<Item=T>
{
    re.over(over).0
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::*;
    use itertools::{Itertools, repeat_n};

    quickcheck! {
        fn epsilon_bool(to_match : Vec<bool>) -> bool {
            to_match.is_empty() == has_match(&mut empty(), to_match)
        }

        fn epsilon_char(to_match : String) -> bool {
            to_match.is_empty() == has_match(&mut empty(), to_match.chars())
        }

        fn fn_bool(to_match : Vec<bool>) -> bool {
            ({
                let mut iter = to_match.iter();
                match (iter.next(), iter.next()) {
                    (Some(&expected), None) => expected,
                    _ => false,
                }
            }) == has_match(&mut is(|&c| Match(c)), to_match)
        }

        fn fn_char(to_match : String) -> bool {
            ({
                let mut iter = to_match.chars();
                match (iter.next(), iter.next()) {
                    (Some(expected), None) => expected.is_uppercase(),
                    _ => false,
                }
            }) == has_match(&mut is(|&c| Match(char::is_uppercase(c))), to_match.chars())
        }

        fn fn_any_bool(to_match : Vec<bool>) -> bool {
            let mut re = is(|_| Match(true));
            (to_match.len() == 1) == has_match(&mut re, to_match)
        }

        fn fn_any_char(to_match : String) -> bool {
            let mut re = is(|_| Match(true));
            (to_match.chars().count() == 1) == has_match(&mut re, to_match.chars())
        }

        fn fn_none_bool(to_match : Vec<bool>) -> bool {
            let mut re = is(|_| Match(false));
            !has_match(&mut re, to_match)
        }

        fn fn_none_char(to_match : String) -> bool {
            let mut re = is(|_| Match(false));
            !has_match(&mut re, to_match.chars())
        }

        fn not_epsilon(to_match : String) -> bool {
            !to_match.is_empty() == has_match(&mut !empty(), to_match.chars())
        }

        fn not_all_bools(to_match : Vec<bool>) -> bool {
            !to_match.iter().all(|&b| b) ==
                has_match(&mut !many(is(|&c| Match(c))), to_match)
        }

        fn alternative(to_match : String) -> bool {
            let a = is(|&c| Match(c == 'a'));
            let b = is(|&c| Match(c == 'b'));
            ({
                let mut iter = to_match.chars();
                match (iter.next(), iter.next()) {
                    (Some(expected), None) => expected == 'a' || expected == 'b',
                    _ => false,
                }
            }) == has_match(&mut (a | b), to_match.chars())
        }

        fn alternative_any_epsilon(to_match : String) -> bool {
            let re = is(|_| Match(true));
            (to_match.chars().count() <= 1) ==
                has_match(&mut (re | empty()), to_match.chars())
        }

        fn alternative_epsilon_any(to_match : String) -> bool {
            let re = is(|_| Match(true));
            (to_match.chars().count() <= 1) ==
                has_match(&mut (empty() | re), to_match.chars())
        }

        fn and(to_match : Vec<u8>) -> bool {
            let hexes = many(is(|&c| Match(c > 10)));
            let uppers = many(is(|&c| Match(c % 2 == 0)));
            to_match.iter().all(|&c| c > 10 && c % 2 == 0) ==
                has_match(&mut (hexes & uppers), to_match)
        }

        fn and_impossible(to_match : String) -> bool {
            let something = is(|_| Match(true));
            let nothing = empty();
            !has_match(&mut (something & nothing), to_match.chars())
        }

        fn sequence_epsilon_left_identity(to_match : String) -> bool {
            let mut re = is(|&c| Match(char::is_uppercase(c)));
            has_match(&mut (empty() + re.clone_reset()), to_match.chars()) ==
                has_match(&mut re, to_match.chars())
        }

        fn sequence_epsilon_right_identity(to_match : String) -> bool {
            let mut re = is(|&c| Match(char::is_uppercase(c)));
            has_match(&mut (re.clone_reset() + empty()), to_match.chars()) ==
                has_match(&mut re, to_match.chars())
        }

        fn sequence_repeat_epsilon_right_identity(to_match : String) -> bool {
            let mut re = many(is(|&c| Match(char::is_uppercase(c))));
            has_match(&mut (re.clone_reset() + empty()), to_match.chars()) ==
                has_match(&mut re, to_match.chars())
        }

        fn repeat_epsilon(to_match : String) -> bool {
            to_match.is_empty() ==
                has_match(&mut many(empty()), to_match.chars())
        }

        fn repeat_any(to_match : String) -> bool {
            let re = is(|_| Match(true));
            has_match(&mut many(re), to_match.chars())
        }

        fn repeat_char(to_match : String) -> bool {
            let re = is(|&c| Match(c == 'A'));
            to_match.chars().all(|c| c == 'A') ==
                has_match(&mut many(re), to_match.chars())
        }

        fn repeat_repeat_char(to_match : String) -> bool {
            let re = is(|&c| Match(c == 'A'));
            to_match.chars().all(|c| c == 'A') ==
                has_match(&mut many(many(re)), to_match.chars())
        }

        fn delayed(to_match : Option<bool>) -> bool {
            let mut re = delay(|| is(|&b| Match(b)).boxed());
            (to_match == Some(true)) == has_match(&mut re, to_match)
        }
    }

    #[test]
    fn balanced_parens() {
        fn parens() -> AnyRegex<u8, Match, impl Regex<u8, Match>> {
            let open = is(|&c| Match(c == b'('));
            let close = is(|&c| Match(c == b')'));
            many(open + delay(|| parens().boxed()) + close)
        }

        fn reference(s: &[u8]) -> bool {
            let mut last_depth = 0;
            let valid_nesting = s
                .iter()
                .scan(0isize, |depth, &c| {
                    match c {
                        b'(' => *depth += 1,
                        b')' => *depth -= 1,
                        _ => return Some(-1),
                    }
                    Some(*depth)
                })
                .inspect(|&depth| last_depth = depth)
                .all(|depth| depth >= 0);
            valid_nesting && last_depth == 0
        }

        let mut parens = parens();
        let alphabet = b"()x".to_vec();
        let alphabet = alphabet.iter().cloned();
        for len in 0..=8 {
            for to_match in repeat_n(alphabet.clone(), len).multi_cartesian_product() {
                let expected = reference(&to_match);
                let actual = has_match(&mut parens, to_match.iter().cloned());
                assert!(expected == actual,
                        "{} {}",
                        std::str::from_utf8(&to_match).unwrap(),
                        if expected { "should match" } else { "should not match" },
                    );
            }
        }
    }
}
