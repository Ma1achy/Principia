//! The chart classification's binding words (REQ-COL-102; `render::bind::chart_words`, `CHART_BINDING`; TASK-M2-32):
//! bit `i mod 32` of word `i / 32` set where sample `i` is forbidden, the rest clear, at least one word. Each test
//! registers its negative control (R-176).

use render::bind::chart_words;
use validation::negative_control;

/// Checks `words` against a set of `n` samples with `set` forbidden.
fn check(words: &dyn Fn(&[bool]) -> Vec<u32>, n: usize, set: &[usize]) {
    let mut f = vec![false; n];
    for &i in set {
        f[i] = true;
    }
    let mut want = vec![0u32; n.div_ceil(32).max(1)];
    for &i in set {
        want[i / 32] |= 1 << (i % 32);
    }
    assert_eq!(words(&f), want, "{n} samples, {set:?} forbidden");
}

fn cases(words: &dyn Fn(&[bool]) -> Vec<u32>) {
    check(words, 0, &[]);
    check(words, 1, &[]);
    check(words, 1, &[0]);
    check(words, 32, &[0, 31]);
    check(words, 33, &[32]);
    check(words, 70, &[1, 5, 33, 34, 65, 69]);
    check(words, 64, &[]);
}

#[test]
fn chart_words_one_bit_per_sample() {
    cases(&chart_words);
}

negative_control!(
    chart_words_one_bit_per_sample,
    "words that drop every bit past the first word",
    expected = "33 samples, [32] forbidden",
    cases(&|f| {
        let mut w = chart_words(f);
        for x in w.iter_mut().skip(1) {
            *x = 0;
        }
        w
    })
);
