# Scannerless parsing of boolean grammars with derivatives in Rust

This is yet another library for writing parsers in Rust. What makes this
one different is that I've combined some existing academic work in a way
that I think is novel. The result is an unusually flexible parsing
library while still offering competitive performance and memory usage.

- This library is efficient for matching regular expressions, but also
  flexible enough to parse context-sensitive programming languages
  without a separate lexer.

- Grammars are written directly in Rust using simple combinators. You
  can also write your own combinators, with some constraints that
  preserve the library's time and space properties.

- Unlike many regular expression libraries, you can match against
  streams of any type, not just bytes or Unicode characters.

- The matching type can be changed without changing the grammar. Simple
  examples include just returning whether the input matches the grammar,
  finding the position and length of the leftmost-longest match, or
  generating a complete parse forest. But you can implement much more
  complex custom behaviors.

- You can parse all context-free languages and some context-sensitive
  languages. Several different features each allow some degree of
  context-sensitivity, and when they're all combined I have no idea what
  languages this library can handle. At minimum, though, it supports
  Boolean grammars, which are context-free grammars augmented with "and"
  and "not" combinators.

- Parsing time should be O(n) for regular expressions, and at worst
  O(n^3) for ambiguous grammars, which is the best any parsing algorithm
  can do. (But I might have performance bugs in the implementation.)

- Many of the grammar combinators don't need any state, so memory usage
  is proportional to the number of concatenation and repetition
  operators active in your grammar at any given time. For a regular
  expression, that number is constant; more complex grammars grow
  proportionally to the depth of the parse tree.

- For regular expressions, memory usage is constant and known at compile
  time, so the state can be stack-allocated. More complex grammars can
  heap-allocate in chunks, where each chunk is the state for a portion
  of the grammar that is a regular expression.

- When Rust monomorphizes the generic types that describe your grammar,
  the compiler is building a dedicated parsing machine specifically for
  your language. Standard compiler optimizations like inlining and
  constant-folding can have a big impact here. But if you need smaller
  code at the cost of a slower parser, you can use Rust's trait objects.

# Related work

This is a Rust implementation of the ideas from several papers about
parsing and languages. I'll divide these papers up into three lines of
research:

- Allowing user-defined parse state in the presence of ambiguous
  grammars

- Making parsing algorithms easier to understand and implement

- Generalizing parsing beyond context-free grammars

## User-defined parse state

First and foremost, my favorite academic paper ever:
["A Play on Regular Expressions"][play]. (I'll just refer to this as
"Play".)

Most of the terminology I use comes from this paper, as well as the
overall design. In particular:

- parse state is maintained as a "marked" or "weighted" regular
  expression;
- the weights can be of any type that satisfies the semiring laws;
- the implementation is derived by applying Brzozowski derivatives to
  regular expressions;
- context-free grammars are supported by allowing lazy construction of a
  potentially infinite regular expression.

I strongly encourage reading this paper. It's written in a very fun
style (it's a play!), and it's intended to help the reader understand
the material, not just to present an unusual algorithm.

[play]: http://sebfisch.github.io/haskell-regexp/

## Understandable parsing algorithms

Next up is ["Parsing with Derivatives"][pwd] ("PWD") and its sequel,
["On the Complexity and Performance of Parsing with Derivatives"][pwd2].
(The first PWD paper was published at the same conference as "Play", one
year later, but doesn't cite that prior work, which I find a little
disappointing.)

The goal of these papers was to describe a parsing algorithm that's
easier to explain than the traditional ones taught in Computer Science
programs&mdash;ideally, without sacrificing performance.

[pwd]: http://matt.might.net/papers/might2011derivatives.pdf
[pwd2]: https://michaeldadams.org/papers/derivatives2/derivatives2.pdf

Like "Play", PWD is based on Brzozowski derivatives of regular
expressions, with context-free grammars supported by recursively defined
regular expressions.

Unlike "Play", PWD does not allow parse results to be arbitrary
semirings. But PWD does show how to construct a parse forest as the
result of running the parser, which "Play" did not do.

Interestingly, a parse forest as defined by the PWD papers satisfies the
semiring laws, which means it's trivial to implement for a parser like
this one. In fact, the parse forest with ambiguity nodes is the [free
semiring][], meaning you can derive the data type you need just by
looking at the semiring laws, aside from efficiency questions like how
to represent shared nodes.

[free semiring]: https://en.wikipedia.org/wiki/Free_object

In addition, the second PWD paper proved O(n^3) asymptotic complexity
bounds on their algorithm, as long as a provided set of optimizations
were applied.

I suspect some of the optimizations from the PWD papers can be applied
in some form to this implementation, or are implicitly already present.
I hope the complexity bounds can be shown to hold for this
implementation too.

One notable difference is that "Play" says their approach does not
support grammars in left-recursive form, while PWD says their approach
handles "left-recursion, right-recursion, ill-founded recursion or any
combination thereof." I don't yet understand how PWD handles left
recursion, but I hope studying that work more closely will reveal some
way to make "Play" support it too.

## Generalizing parsing

The parsing tools that people usually use are limited to some subset of
context-free grammars. So you might think that most programming
languages would have context-free grammars. You'd only be sort of right.
There are widely-known issues with writing an unambiguous context-free
grammar for C, for example, but you can get pretty close.

However, that's only true if your parser is coupled with a lexical
analyzer (usually called a lexer, or sometimes called a scanner). If you
try to run your parser over the individual characters of the source
code, then basic things that people take for granted fall outside the
context-free realm.

For example, context-free grammars can't prevent variables from having
the same name as keywords, or force taking the longest matching
identifier even when a prefix of it would still allow what comes next to
parse.

That's why the paper ["Scannerless Boolean Parsing"][sbp] ("SBP") is
interesting. It describes how to apply an extension of context-free
grammars, called Boolean grammars, to practical problems like these.

[sbp]: http://www.megacz.com/berkeley/research/papers/sbp-preprint.pdf

SBP cites previous work, ["Scannerless Generalized-LR Parsing"][sdf2],
which introduced several empirically-chosen extensions to context-free
grammars in order to handle common cases that occur in common
programming languages. SBP's main contribution was to show that those
extensions are all special cases of Boolean grammars.

[sdf2]: http://citeseerx.ist.psu.edu/viewdoc/summary?doi=10.1.1.37.7828&rank=1

Context-free grammars can be constructed from recursion, concatenation,
and union ("or") operators. Conjunctive grammars add intersection
("and") to that, and Boolean grammars further add complement ("not").

A result from language theory is that if you combine two regular
languages using any of the Boolean operators ("or", "and", "not"), the
result is still regular. But if you apply "and" or "not" to context-free
grammars, the result is no longer context-free.

Adding support for "and" and "not" to this library was very easy,
because the semiring laws already provided the necessary foundations.
And because the "Play" approach to context-free parsing is to add lazy
recursion to regular expression matching, that means we get full support
for Boolean grammars as well.

For additional background on Boolean grammars, as well as examples of
languages that are Boolean but not context-free, I recommend a survey
paper titled ["Conjunctive and Boolean grammars: the true general case
of the context-free grammars"][bool-survey], as well as
[its author's site][bool-site].

[bool-survey]: http://users.utu.fi/aleokh/papers/boolean_survey.pdf
[bool-site]: http://users.utu.fi/aleokh/boolean/
