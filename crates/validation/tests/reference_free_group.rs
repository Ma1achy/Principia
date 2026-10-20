//! The reference free-group reducer (`validation::reference::free_group`) on hand-computed cases from payload §3's
//! rules (dd_simstate_payload §3): free reduction, the length cap, the closed form of `W` and the packed word. The
//! kernel's word tests compare the append against this reducer, so it is checked here on its own.

use validation::negative_control;
use validation::reference::free_group::{closed_form, digit, packed, reduce, Reduced};

/// A reducer: [`reduce`], or the control's faulty one.
type Reduce = fn(&[u32]) -> Reduced;

/// A reduced word, not truncated.
fn word(symbols: &[u32]) -> Reduced {
    Reduced {
        symbols: symbols.to_vec(),
        truncated: false,
    }
}

/// `reduce` on hand-reduced streams, codes `a = 0, A = 1, b = 2, B = 3`: cancellation, a pop at the cap, a push at the
/// cap and the crossings after it.
fn check_reduce(reduce: Reduce) {
    let mut a76 = vec![0; 76];
    let cases: Vec<(Vec<u32>, Reduced)> = vec![
        (vec![], word(&[])),
        (vec![0, 1], word(&[])),
        // a b B A: b B cancels, then A cancels a.
        (vec![0, 2, 3, 1], word(&[])),
        // a b A: no cancellation, A follows b.
        (vec![0, 2, 1], word(&[0, 2, 1])),
        // 76 a, then A: a pop at the cap, 75 a.
        ([&a76[..], &[1]].concat(), word(&[0; 75])),
        // 76 a, then a: a push at the cap truncates; the A after it is ignored.
        (
            [&a76[..], &[0, 1]].concat(),
            Reduced {
                symbols: vec![0; 76],
                truncated: true,
            },
        ),
    ];
    for (stream, expected) in &cases {
        assert_eq!(reduce(stream), *expected, "reduce({stream:?})");
    }
    a76.push(3);
    assert_eq!(reduce(&a76).length(), 127, "a truncated word's length");
    assert_eq!(reduce(&[0, 2]).length(), 2, "a word's length");
}

#[test]
fn reference_free_group_reduce() {
    check_reduce(reduce);
}

/// A reducer testing the cap before the inverse, so a pop at the cap truncates.
#[cfg(feature = "controls")]
fn reduce_cap_first(stream: &[u32]) -> Reduced {
    let mut symbols: Vec<u32> = Vec::new();
    for &s in stream {
        if symbols.len() == 76 {
            return Reduced {
                symbols,
                truncated: true,
            };
        } else if symbols.last() == Some(&[1, 0, 3, 2][s as usize]) {
            symbols.pop();
        } else {
            symbols.push(s);
        }
    }
    Reduced {
        symbols,
        truncated: false,
    }
}

negative_control!(
    reference_free_group_reduce,
    "a reducer capping before it pops truncates a pop at the cap",
    expected = "reduce(",
    check_reduce(reduce_cap_first)
);

/// `closed_form` and `packed` on hand-computed words, through `closed`.
fn check_closed_form(closed: fn(&[u32]) -> u128) {
    // a b: d₀ = 0, b continues a by digit 1: W = 1.
    // B a b: d₀ = 3, a continues B by 2, b continues a by 1: W = 3·9 + 2·3 + 1 = 34.
    // A A A: d₀ = 1, digits 0, 0: W = 9.
    for (symbols, w) in [
        (&[][..], 0),
        (&[2][..], 2),
        (&[0, 2][..], 1),
        (&[3, 0, 2][..], 34),
        (&[1, 1, 1][..], 9),
    ] {
        assert_eq!(closed(symbols), w, "W of {symbols:?}");
    }
    assert_eq!(
        packed(&word(&[3, 0, 2])),
        [34, 0, 0, 3 << 25],
        "the packed word"
    );
    assert_eq!(
        packed(&Reduced {
            symbols: vec![1; 76],
            truncated: true,
        }),
        {
            let w = 3u128.pow(75);
            [
                w as u32,
                (w >> 32) as u32,
                (w >> 64) as u32,
                (w >> 96) as u32 | 127 << 25,
            ]
        },
        "the packed truncated word"
    );
    assert_eq!(
        (digit(0, 1), digit(3, 0), digit(2, 2)),
        (None, Some(2), Some(0)),
        "digits"
    );
}

#[test]
fn reference_free_group_closed_form() {
    check_closed_form(closed_form);
}

/// `W` packed base 4, a flat 2-bit code, not payload §3's mixed radix.
#[cfg(feature = "controls")]
fn closed_form_base4(symbols: &[u32]) -> u128 {
    symbols.iter().fold(0, |w, &s| 4 * w + u128::from(s))
}

negative_control!(
    reference_free_group_closed_form,
    "a flat base-4 packing is not the closed form",
    expected = "W of",
    check_closed_form(closed_form_base4)
);
